//! The Launchkey pads: the pad page, its order, and the pads as the state shows them.

use super::{Control, View};
use crate::api::{AppCmd, CmdError, Pad, PadPageInfo, PadsCmd, PadsState, PaletteLed};
use crate::launchkey::{self, Layer, Led, Page, PageOrder, Panel};
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
            .map(|(i, (note, look))| Pad {
                note: *note,
                label: look.label.to_string(),
                key: look.key.to_string(),
                rgb: [look.rgb.0, look.rgb.1, look.rgb.2],
                level: look.level,
                anim: look.anim,
                action: launchkey::pad_action(page, layer, *note).map(AppCmd::from),
                palette: palette.map(|leds| palette_led(leds[i].1)),
            })
            .collect()
    }

    pub(super) fn pads_state(&self, v: &View) -> PadsState {
        let pnl = &v.pnl;
        let order = pnl.order;
        PadsState {
            page: pnl.page,
            page_name: pnl.page.name().to_string(),
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
