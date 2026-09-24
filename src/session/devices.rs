//! MIDI devices coming and going (#74): keyboards plugged in (or out) while the session
//! runs, and the Launchkey unplugged and plugged back in.
//!
//! CoreMIDI tells the process of setup changes, and keeps the endpoints other processes
//! create up to date, through the run loop of the thread that first used it, and only
//! while that run loop runs: `midi::init` makes that a thread of our own, and counts the
//! changes (`midi::setup_generation`). The control thread sees a new count at its next
//! pump (it wakes every 10 ms), lists the sources again and:
//! - connects the keyboards `Options::inputs`/`all_inputs` choose (`connect_inputs`), and
//!   releases the held keys of one that went;
//! - connects the Launchkey's DAW port as the pads when it appears, puts the Launchkey
//!   (back) into DAW mode and sends every LED again, since a Launchkey plugged back in
//!   starts in its standalone mode with its LEDs dark (`connect_pads`);
//! - updates `io.inputs`, `io.sources` and `pads.connected`, which the next publish
//!   carries.
//!
//! The sources are also listed every 2 s, in case a change comes without a notification.
//! Only online endpoints count: an unplugged device can stay listed, offline.

use super::{Control, Leds};
use crate::launchkey;
use crate::live::TAG_PADS;
use crate::midi::{self, Endpoint};
use crate::rt::{PacketSink, Target};
use std::sync::atomic::Ordering::Relaxed;

/// How often the sources are listed again without a notification.
const POLL_NS: u64 = 2_000_000_000;

/// What the Launchkey's DAW port did since the last look, from the DAW port yahaha has
/// (`have`) and the one online now (`now`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DawChange {
    /// Nothing: still the same port, or still none.
    Same,
    /// It went (unplugged).
    Gone,
    /// One is there that yahaha doesn't drive yet (plugged in, or back in).
    Came(Endpoint),
}

pub(super) fn daw_change(have: Option<Endpoint>, now: Option<Endpoint>) -> DawChange {
    match (have, now) {
        (Some(a), Some(b)) if a == b => DawChange::Same,
        (_, Some(b)) => DawChange::Came(b),
        (Some(_), None) => DawChange::Gone,
        (None, None) => DawChange::Same,
    }
}

/// The LEDs' DAW destination: from the one they last went to (`have`) and the one
/// online now, while yahaha drives the DAW port (`driving`): the destination they go to
/// from now, and the one to open (DAW mode, every LED) now, if any.
pub(super) fn leds_change(driving: bool, have: Option<Endpoint>, now: Option<Endpoint>) -> (Option<Endpoint>, Option<Endpoint>) {
    if !driving {
        return (None, None);
    }
    match daw_change(have, now) {
        DawChange::Same => (have, None),
        DawChange::Gone => (None, None),
        DawChange::Came(d) => (Some(d), Some(d)),
    }
}

impl Control {
    /// Live: follow the MIDI setup when CoreMIDI says it changed, and every 2 s.
    pub(super) fn pump_devices(&mut self, now: u64) {
        let Some(m) = self.midi.as_mut() else { return };
        let generation = midi::setup_generation();
        let changed = generation != m.setup_gen;
        m.setup_gen = generation;
        if changed || now.saturating_sub(self.sources_ns) >= POLL_NS {
            self.sources_ns = now;
            self.connect_pads();
            self.connect_inputs();
        }
    }

    /// Drive the Launchkey's DAW port (pads, buttons, faders and LEDs) if one is online,
    /// and let it go when it goes. Not with `--no-pads`.
    pub(super) fn connect_pads(&mut self) {
        let Some(m) = self.midi.as_mut() else { return };
        if m.no_pads {
            return;
        }
        let online = midi::online_sources().into_iter().find(|(_, n)| super::is_daw(n));
        match daw_change(m.daw.as_ref().map(|d| d.0), online.as_ref().map(|d| d.0)) {
            DawChange::Same => {}
            DawChange::Gone => {
                if let Some((e, _)) = m.daw.take() {
                    let _ = m.port.disconnect(e);
                }
                self.pads_connected = false;
                // A Shift held as it went is never released.
                self.shared.shift.store(false, Relaxed);
            }
            DawChange::Came(e) => {
                if let Some((old, _)) = m.daw.take() {
                    let _ = m.port.disconnect(old);
                }
                if m.port.connect(e, TAG_PADS).is_ok() {
                    m.daw = online;
                    self.pads_connected = true;
                    // A Launchkey (back) in: DAW mode and every LED again, below.
                    m.leds_dest = None;
                }
            }
        }
        self.follow_leds();
    }

    /// While yahaha drives the DAW port: put the Launchkey into DAW mode through its DAW
    /// destination, and have every LED sent again, when that destination comes online (a
    /// Launchkey plugged back in starts standalone, with its LEDs dark). The destination
    /// can come online after the source: then it happens at the pump that sees it (a
    /// setup change, or the 2 s poll), not never.
    fn follow_leds(&mut self) {
        let Some(m) = self.midi.as_mut() else { return };
        let driving = m.daw.is_some() && m.leds_port.is_some();
        let dest = if driving { midi::online_destinations().into_iter().find(|(_, n)| super::is_daw(n)).map(|d| d.0) } else { None };
        let (have, open) = leds_change(driving, m.leds_dest, dest);
        m.leds_dest = have;
        if let Some(d) = open {
            self.open_leds(d);
        }
    }

    /// DAW mode and every LED again, through DAW destination `d`.
    fn open_leds(&mut self, d: Endpoint) {
        let Some(port) = self.midi.as_ref().and_then(|m| m.leds_port) else { return };
        let mut out = PacketSink::new(Target::Port(port, d));
        out.push(&launchkey::ENTER_DAW);
        out.flush();
        match self.leds.as_mut() {
            Some(l) => l.reconnect(out),
            None => self.leds = Some(Leds::new(out, self.palette_leds)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unplugged, plugged back in (the same endpoint or a new one), another port: each
    /// is seen once.
    #[test]
    fn launchkey_comings_and_goings() {
        assert_eq!(daw_change(None, None), DawChange::Same);
        assert_eq!(daw_change(None, Some(7)), DawChange::Came(7));
        assert_eq!(daw_change(Some(7), Some(7)), DawChange::Same);
        assert_eq!(daw_change(Some(7), None), DawChange::Gone);
        // Back in, as the same endpoint: driven again (after `Gone`, yahaha has none).
        assert_eq!(daw_change(None, Some(7)), DawChange::Came(7));
        // Another Launchkey port in its place.
        assert_eq!(daw_change(Some(7), Some(9)), DawChange::Came(9));
    }

    /// Review of #101, N2: the DAW destination comes online after the DAW source. The pump
    /// that connects the source finds no destination; a later pump that sees it opens it
    /// (DAW mode, every LED), once.
    #[test]
    fn a_late_daw_destination_is_opened_when_it_comes() {
        // Source connected (connect_pads clears `have`), destination not online yet.
        assert_eq!(leds_change(true, None, None), (None, None));
        // It comes online: opened.
        assert_eq!(leds_change(true, None, Some(5)), (Some(5), Some(5)));
        // And not again while it stays.
        assert_eq!(leds_change(true, Some(5), Some(5)), (Some(5), None));
        // It goes, and comes back: opened again.
        assert_eq!(leds_change(true, Some(5), None), (None, None));
        assert_eq!(leds_change(true, None, Some(5)), (Some(5), Some(5)));
        // Not driving the DAW port (unplugged, --no-pads): nothing.
        assert_eq!(leds_change(false, Some(5), Some(5)), (None, None));
    }
}
