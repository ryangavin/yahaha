//! The Launchkey LEDs.

use crate::engine::Snapshot;
use crate::launchkey::{self, Led, Page, Panel};
use crate::parts::FaderPage;
use crate::rt::PacketSink;

/// The Launchkey LEDs: pads, fader buttons, Pad Bank and Track buttons. Sends only what
/// changed.
pub(super) struct Leds {
    pub(super) out: PacketSink,
    palette: bool,
    last_leds: [(u8, Option<Led>); 16],
    last_rgb: [Option<(u8, u8, u8)>; 16],
    last_fader_btns: Option<(FaderPage, u8, u8, bool, bool)>,
    last_nav: Option<(Page, bool)>,
    buf: Vec<[u8; 3]>,
}

impl Leds {
    pub(super) fn new(out: PacketSink, palette: bool) -> Leds {
        Leds { out, palette, last_leds: [(0, None); 16], last_rgb: [None; 16], last_fader_btns: None, last_nav: None, buf: Vec::new() }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn update(&mut self, s: &Snapshot, has: &[bool], pnl: &Panel, manual_bass: bool, fader_page: FaderPage, styles: bool, beats: f64) {
        if self.palette {
            for (i, (note, led)) in launchkey::pad_leds(s, has, pnl).into_iter().enumerate() {
                if self.last_leds[i] != (note, Some(led)) {
                    self.buf.clear();
                    launchkey::led_msgs(note, led, &mut self.buf);
                    for m in &self.buf {
                        self.out.push(m);
                    }
                    self.last_leds[i] = (note, Some(led));
                }
            }
        } else {
            for (i, (pad, look)) in launchkey::looks(s, has, pnl).iter().enumerate() {
                let rgb = launchkey::rgb_at(look, beats);
                if self.last_rgb[i] != Some(rgb) {
                    self.out.push(&launchkey::rgb_sysex(*pad, rgb));
                    self.last_rgb[i] = Some(rgb);
                }
            }
        }
        let style_on = launchkey::style_lit(s.parts, manual_bass);
        let fb = (fader_page, pnl.parts_on, style_on, pnl.harmony_arp, pnl.plugin_fault);
        if self.last_fader_btns != Some(fb) {
            self.buf.clear();
            launchkey::fader_button_msgs(fb.0, fb.1, fb.2, fb.3, fb.4, &mut self.buf);
            for m in &self.buf {
                self.out.push(m);
            }
            self.last_fader_btns = Some(fb);
        }
        if self.last_nav != Some((pnl.page, styles)) {
            self.buf.clear();
            launchkey::nav_button_msgs(pnl.page, styles, &mut self.buf);
            for m in &self.buf {
                self.out.push(m);
            }
            self.last_nav = Some((pnl.page, styles));
        }
        self.out.flush();
    }

    /// Send through `out` from now on (the Launchkey came back, maybe as another
    /// endpoint), and send every LED again: it comes back with them dark.
    pub(super) fn reconnect(&mut self, out: PacketSink) {
        self.out = out;
        self.forget();
    }

    /// Forget what was sent: the next `update` sends every LED.
    pub(super) fn forget(&mut self) {
        self.last_leds = [(0, None); 16];
        self.last_rgb = [None; 16];
        self.last_fader_btns = None;
        self.last_nav = None;
    }

    /// Palette colours or RGB from now on: every pad is sent again.
    pub(super) fn set_palette(&mut self, on: bool) {
        if self.palette != on {
            self.palette = on;
            self.last_leds = [(0, None); 16];
            self.last_rgb = [None; 16];
        }
    }

    /// Pads and buttons dark, and the Launchkey back out of DAW mode.
    pub(super) fn off(&mut self) {
        for n in (96..104).chain(112..120) {
            self.out.push(&[0x90, n, 0]);
        }
        self.buf.clear();
        launchkey::buttons_off_msgs(&mut self.buf);
        for m in &self.buf {
            self.out.push(m);
        }
        self.out.push(&launchkey::EXIT_DAW);
        self.out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::Target;

    /// A Launchkey plugged back in comes back dark: `reconnect` sends every LED again,
    /// where an `update` with nothing changed sends nothing.
    #[test]
    fn reconnect_sends_every_led_again() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            return;
        }
        let e = crate::engine::Engine::new(Box::new(crate::engine::Prepared::new(&crate::sff::Style::load(&p).unwrap())));
        let s = e.snapshot(0);
        let has = [true; 17];
        let pnl = Panel::default();
        for palette in [false, true] {
            let mut l = Leds::new(PacketSink::new(Target::Null), palette);
            l.update(&s, &has, &pnl, false, FaderPage::Panel, true, 0.0);
            let first = l.out.sent;
            assert!(first > 0);
            l.update(&s, &has, &pnl, false, FaderPage::Panel, true, 0.0);
            assert_eq!(l.out.sent, first, "nothing changed, nothing sent");
            l.reconnect(PacketSink::new(Target::Null));
            l.update(&s, &has, &pnl, false, FaderPage::Panel, true, 0.0);
            assert_eq!(l.out.sent, first, "every LED again, through the new output");
        }
    }
}
