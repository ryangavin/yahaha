//! The Launchkey pads: the pad page, its order, and the pads as the state shows them.

use super::{Control, View};
use crate::api::{AppCmd, CmdError, Pad, PadPageInfo, PadsCmd, PadsState, PaletteLed, QuickRackCmd};
use crate::launchkey::{self, Action, Layer, Led, Page, PageOrder, Panel};
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn pads_cmd(&mut self, c: PadsCmd) -> Result<(), CmdError> {
        match c {
            PadsCmd::SetPadPage { page } => {
                if !self.shared.page_order().contains(page) {
                    return self.fail(format!("the {} pad page is left out of the page order (Settings › Launchkey)", page.name()));
                }
                self.shared.page.store(page.to_u8(), Relaxed);
            }
            PadsCmd::CyclePadPage { delta } => {
                let order = self.shared.page_order();
                self.shared.step_page(|p| order.cycle(p, delta));
            }
            PadsCmd::SetPadPageOrder { pages } => {
                let Some(order) = PageOrder::new(&pages) else {
                    return self.fail("the pad page order names pages 2-5 once each, never Sections");
                };
                self.shared.page_order.store(order.to_bits(), Relaxed);
                // The page on view left out: back to Sections.
                self.shared.step_page(|p| if order.contains(p) { p } else { Page::Sections });
            }
            PadsCmd::SetLayer { layer } => return self.set_layer(layer),
        }
        Ok(())
    }

    /// `setLayer`: the app's mirror holds or releases Sound or a part button, through the
    /// same steps the input thread runs for the Launchkey's (`live::sound_hold`,
    /// `live::swap`). The pads read `Shared::layer` on the next press, so the Racks page
    /// (and capture on a tap, `Action::QuickRackHeld`) follows at once.
    fn set_layer(&mut self, to: Layer) -> Result<(), CmdError> {
        use crate::live::{sound_hold, swap};
        let now = self.shared.layer();
        let next = match to {
            Layer::Sound => sound_hold::press(now),
            Layer::Swap { part } if part as usize >= crate::parts::COUNT => return self.fail(format!("no keyboard part {part}")),
            Layer::Swap { part } => Layer::Swap { part },
            Layer::None if now == Layer::Sound => sound_hold::release(now),
            Layer::None => Layer::None,
        };
        self.shared.layer.store(next.to_u8(), Relaxed);
        // Leaving swap mode: the release after a turn, as the input thread runs it.
        if let (Layer::Swap { part }, Layer::None) = (now, next)
            && let Some(a) = swap::commit(part)
        {
            return self.apply_hardware(a);
        }
        Ok(())
    }

    /// The 16 pads of `page` under `layer`, the panel `pnl` shown on them.
    pub(super) fn pads(&self, pnl: &Panel, page: Page, layer: Layer) -> Vec<Pad> {
        let (s, info) = (&self.snap, &self.info);
        let p = Panel { page, layer, ..*pnl };
        let palette = if self.palette_leds { Some(launchkey::pad_leds(s, &info.has, &p)) } else { None };
        launchkey::looks(s, &info.has, &p)
            .iter()
            .enumerate()
            .map(|(i, (note, look))| {
                let action = launchkey::pad_action(page, layer, *note);
                // Hold Sound: the lit Quick Rack pad and the empty ones capture the live
                // rack (`storeRack`), as a tap on the Launchkey does under the hold.
                let capture = match action {
                    Some(Action::QuickRack(slot)) if layer == Layer::Sound && !pnl.quick.store && self.sound_tap_captures(slot) => Some(slot),
                    _ => None,
                };
                // The label stays "QUICK n": the display names the pad after it acted, and a
                // recalled pad is the lit one by then.
                Pad {
                    note: *note,
                    label: look.label.to_string(),
                    key: look.key.to_string(),
                    rgb: [look.rgb.0, look.rgb.1, look.rgb.2],
                    level: look.level,
                    anim: look.anim,
                    action: capture.map_or_else(|| action.map(AppCmd::from), |slot| Some(QuickRackCmd::StoreRack { slot }.into())),
                    palette: palette.map(|leds| palette_led(leds[i].1)),
                }
            })
            .collect()
    }

    pub(super) fn pads_state(&self, v: &View) -> PadsState {
        let pnl = &v.pnl;
        let order = pnl.order;
        PadsState {
            page: pnl.page,
            // The page the pads show: Racks while Sound is held (`page` stays the one on
            // view, which comes back on release).
            page_name: pnl.layer.pads(pnl.page).name().to_string(),
            page_number: order.position(pnl.page).map_or(0, |i| i as u8 + 1),
            page_count: order.len() as u8,
            pages: order.pages().map(|page| PadPageInfo { page, name: page.name().to_string() }).collect(),
            // Hold Sound: the pads show (and do) what the Racks page does.
            pads: self.pads(pnl, pnl.page, pnl.layer),
            connected: self.pads_connected,
            palette_leds: self.palette_leds,
        }
    }
}

/// What a pad shows in palette mode.
fn palette_led(led: Led) -> PaletteLed {
    let look = |c: u8| {
        let (rgb, level) = launchkey::palette_colour(c);
        ([rgb.0, rgb.1, rgb.2], level)
    };
    let (mode, c, flash) = match led {
        Led::Solid(c) => (launchkey::Anim::Solid, c, None),
        Led::Flash(a, b) => (launchkey::Anim::Flash, a, Some(b)),
        Led::Pulse(c) => (launchkey::Anim::Pulse, c, None),
    };
    let (rgb, level) = look(c);
    PaletteLed {
        mode,
        colour: c,
        rgb,
        level,
        flash_colour: flash,
        flash_rgb: flash.map(|f| look(f).0),
        flash_level: flash.map(|f| look(f).1),
    }
}

#[cfg(test)]
mod tests {
    use crate::api::*;
    use crate::launchkey::{Layer, Page};
    use crate::session::testing::session;

    /// `setLayer {sound}` is the app's Sound hold: the pads are the Racks page from any
    /// page, the page on view kept; `none` gives the page back.
    #[test]
    fn set_layer_sound_shows_the_racks_page_until_none() {
        let s = session();
        s.send(PadsCmd::SetPadPage { page: Page::Chord }).unwrap();
        let before = s.state().pads.clone();
        assert_ne!(before.page_name, "Racks");
        s.send(PadsCmd::SetLayer { layer: Layer::Sound }).unwrap();
        let st = s.state();
        assert_eq!(st.surface.layer, Layer::Sound);
        assert_eq!((st.pads.page, st.pads.page_name.as_str()), (Page::Chord, "Racks"));
        s.send(PadsCmd::SetLayer { layer: Layer::None }).unwrap();
        let st = s.state();
        assert_eq!(st.surface.layer, Layer::None);
        assert_eq!((st.pads.page, st.pads.page_name.as_str()), (Page::Chord, before.page_name.as_str()));
    }

    /// `setLayer {swap}` is the app's part-button hold: the knobs are that part's until
    /// `none`; a part past Left is refused.
    #[test]
    fn set_layer_swap_puts_the_knobs_in_swap_mode_until_none() {
        let s = session();
        let page = s.state().knobs.page_name.clone();
        s.send(PadsCmd::SetLayer { layer: Layer::Swap { part: 1 } }).unwrap();
        let st = s.state();
        assert_eq!((st.surface.layer, st.knobs.page_name.as_str()), (Layer::Swap { part: 1 }, "Swap R2"));
        s.send(PadsCmd::SetLayer { layer: Layer::None }).unwrap();
        let st = s.state();
        assert_eq!((st.surface.layer, st.knobs.page_name.as_str()), (Layer::None, page.as_str()));
        assert!(s.send(PadsCmd::SetLayer { layer: Layer::Swap { part: 4 } }).is_err());
        assert_eq!(s.state().surface.layer, Layer::None);
    }
}
