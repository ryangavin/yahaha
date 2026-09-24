//! What the `yahaha` MIDI port gets for the band's program changes (#103).
//!
//! By default the port is a faithful mirror: the style's own bank selects and program
//! changes, unchanged, whatever the program map plays in the built-in synth. With the
//! library's "port sends mapped programs" setting on, a program change the map routes to a
//! SoundFont patch goes out as that patch's bank and program instead (bank MSB = the
//! SoundFont bank, 127 for a drum kit; LSB 0), for a GM/GS module or a DAW instrument set
//! up with the same SoundFont. Everything else (CC7, pan, the sends, notes) is untouched.
//!
//! Runs on the engine thread, inside `live::Out`: fixed arrays and atomic loads only.

use super::Routes;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

/// `live::Out`'s program mapping for the port.
#[derive(Default)]
pub struct PortMap {
    routes: Option<Arc<Routes>>,
    /// The table bank the band plays with (`Sink::route_bank`).
    active: u8,
    /// Each channel's bank MSB as the style last sent it.
    msb: [u8; 16],
}

impl PortMap {
    pub fn new(routes: Arc<Routes>) -> PortMap {
        PortMap { routes: Some(routes), active: 0, msb: [0; 16] }
    }

    pub fn set_bank(&mut self, bank: u8) {
        self.active = bank;
    }

    /// Send `msg` to the port, mapped if it is a band program change the map routes and
    /// the setting is on.
    #[inline]
    pub fn send(&mut self, msg: &[u8], mut midi: impl FnMut(&[u8])) {
        let (ch, st) = (msg.first().map_or(0, |s| s & 0x0F), msg.first().map_or(0, |s| s & 0xF0));
        if st == 0xB0 && msg.get(1) == Some(&0) {
            self.msb[ch as usize] = msg.get(2).copied().unwrap_or(0);
        }
        if st == 0xC0
            && ch >= 8
            && let Some(routes) = &self.routes
            && routes.port_mapped.load(Relaxed)
            && let Some(r) = routes.lookup(self.active, ch, self.msb[ch as usize], msg.get(1).copied().unwrap_or(0))
            && r.font().is_some()
        {
            let msb = if r.bank >= 128 { 127 } else { r.bank as u8 };
            midi(&[0xB0 | ch, 0, msb]);
            midi(&[0xB0 | ch, 32, 0]);
            midi(&[0xC0 | ch, r.program]);
            return;
        }
        midi(msg);
    }
}
