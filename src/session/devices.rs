//! MIDI devices coming and going (#74): keyboards plugged in (or out) while the session
//! runs, and the Launchkey unplugged and plugged back in.
//!
//! The session's CoreMIDI client lives on a thread of its own that runs a run loop
//! (`midi::spawn_client`): CoreMIDI delivers the client's notifications there, and keeps
//! the endpoints other processes create up to date only while it runs. A notification
//! sets a flag; the control thread sees it at its next pump (it wakes every 10 ms), lists
//! the sources again and:
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
use std::sync::atomic::Ordering::{AcqRel, Relaxed};

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

impl Control {
    /// Live: follow the MIDI setup when CoreMIDI says it changed, and every 2 s.
    pub(super) fn pump_devices(&mut self, now: u64) {
        let Some(m) = self.midi.as_ref() else { return };
        let changed = m.changed.swap(false, AcqRel);
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
                    self.open_leds();
                }
            }
        }
    }

    /// Put the Launchkey into DAW mode through its DAW destination, and have every LED
    /// sent again (a Launchkey plugged back in has them dark).
    fn open_leds(&mut self) {
        let Some(port) = self.midi.as_ref().and_then(|m| m.leds_port) else { return };
        let Some((d, _)) = midi::online_destinations().into_iter().find(|(_, n)| super::is_daw(n)) else { return };
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
}
