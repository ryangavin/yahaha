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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patches::Route;

    fn send(pm: &mut PortMap, m: &[u8]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        pm.send(m, |x| out.push(x.to_vec()));
        out
    }

    #[test]
    fn the_port_mirrors_the_style_unless_mapped_programs_are_on() {
        let routes = Arc::new(Routes::new());
        let mut prog = [None; 128];
        prog[33] = Some(Route::sound_font(1, 8, 34));
        routes.write_bank(1, &prog, Some(Route::sound_font(1, 128, 25)));
        let mut pm = PortMap::new(routes.clone());
        pm.set_bank(1);
        // Default: unchanged.
        assert_eq!(send(&mut pm, &[0xBA, 0, 0]), [vec![0xBA, 0, 0]]);
        assert_eq!(send(&mut pm, &[0xCA, 33]), [vec![0xCA, 33]]);
        routes.port_mapped.store(true, Relaxed);
        assert_eq!(send(&mut pm, &[0xCA, 33]), [vec![0xBA, 0, 8], vec![0xBA, 32, 0], vec![0xCA, 34]]);
        // A drum kit goes out on the XG drum bank.
        assert_eq!(send(&mut pm, &[0xC9, 0]), [vec![0xB9, 0, 127], vec![0xB9, 32, 0], vec![0xC9, 25]]);
        // Unmapped programs, the keyboard parts' channels, CC7 and notes: as sent.
        assert_eq!(send(&mut pm, &[0xCA, 40]), [vec![0xCA, 40]]);
        assert_eq!(send(&mut pm, &[0xC0, 33]), [vec![0xC0, 33]]);
        assert_eq!(send(&mut pm, &[0xBA, 7, 90]), [vec![0xBA, 7, 90]]);
        assert_eq!(send(&mut pm, &[0x9A, 40, 100]), [vec![0x9A, 40, 100]]);
        // The other bank (another style's map) has no route for 33.
        pm.set_bank(0);
        assert_eq!(send(&mut pm, &[0xCA, 33]), [vec![0xCA, 33]]);
        // Without a table (tests, the input thread's Out): as sent.
        assert_eq!(send(&mut PortMap::default(), &[0xCA, 33]), [vec![0xCA, 33]]);
    }
}
