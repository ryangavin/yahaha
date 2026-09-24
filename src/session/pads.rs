//! The Launchkey pads: the pad page, and the pads as the state shows them.

use super::{Control, View};
use crate::api::{AppCmd, CmdError, Pad, PadsCmd, PadsState, PaletteLed};
use crate::launchkey::{self, Led, Page, Panel};
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn pads_cmd(&mut self, c: PadsCmd) -> Result<(), CmdError> {
        match c {
            PadsCmd::SetPadPage { page } => self.shared.page.store(page.to_u8(), Relaxed),
            PadsCmd::CyclePadPage { delta } => self.shared.step_page(|p| p.cycle(delta)),
        }
        Ok(())
    }

    /// The 16 pads of `page`, the panel `pnl` shown on it.
    pub(super) fn pads(&self, pnl: &Panel, page: Page) -> Vec<Pad> {
        let (s, info) = (&self.snap, &self.info);
        let p = Panel { page, ..*pnl };
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
                action: launchkey::pad_action(page, *note).map(AppCmd::from),
                palette: palette.map(|leds| palette_led(leds[i].1)),
            })
            .collect()
    }

    pub(super) fn pads_state(&self, v: &View) -> PadsState {
        let pnl = &v.pnl;
        PadsState {
            page: pnl.page,
            page_name: pnl.page.name().to_string(),
            page_number: pnl.page.to_u8() + 1,
            page_count: Page::ALL.len() as u8,
            pads: self.pads(pnl, pnl.page),
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
