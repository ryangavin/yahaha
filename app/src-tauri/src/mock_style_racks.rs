//! Style racks in the dev mock (docs/racks.md "Styles and OTS"), kept in memory: per style
//! file name, which of OTS 1–4 load one of the user's racks instead of the style's own. As
//! mock-style-racks.ts. The mock has no hardware; OTS Link switches without the guard.

use super::MockSession;
use std::collections::BTreeMap;
use yahaha::api::*;

#[derive(Default)]
pub(super) struct MockStyleRacks {
    styles: BTreeMap<String, [Option<String>; 4]>,
}

impl MockSession {
    /// The loaded style's key: its file name.
    fn style_key(&self) -> String {
        let p = &self.state.style.path;
        p.rsplit('/').next().unwrap_or(p).to_string()
    }

    /// `setOtsRack` / `clearOtsRack`.
    pub(super) fn set_ots_rack(&mut self, index: u8, id: Option<String>) {
        let style = self.state.style.name.clone();
        if index as usize >= self.state.ots.settings.len().min(4) {
            return self.message(format!("{style} has no OTS {}", index + 1), true);
        }
        let name = match &id {
            Some(id) => match self.racks.entries().into_iter().find(|r| r.id == *id) {
                Some(r) => Some(r.name),
                None => return self.message(format!("no rack {id}"), true),
            },
            None => None,
        };
        let key = self.style_key();
        let slots = self.style_racks.styles.entry(key.clone()).or_default();
        slots[index as usize] = id;
        if slots.iter().all(Option::is_none) {
            self.style_racks.styles.remove(&key);
        }
        let text = match name {
            Some(n) => format!("With {style} loaded, OTS {} now loads {n}", index + 1),
            None => format!("OTS {} is back to {style}'s own", index + 1),
        };
        self.message(text, false);
    }

    /// The rack OTS `index` of the loaded style loads, if one is chosen and still exists.
    pub(super) fn style_rack_for(&self, index: usize) -> Option<String> {
        let id = self.style_racks.styles.get(&self.style_key())?.get(index)?.clone()?;
        self.racks.entries().iter().any(|r| r.id == id).then_some(id)
    }

    /// OTS `n` loads rack `id`: through the guard from the app, switching anyway from OTS
    /// Link. Loaded, it counts as that OTS recalled.
    pub(super) fn recall_style_rack(&mut self, n: usize, id: String, unattended: bool) {
        self.rack_cmd(RackCmd::LoadRack { id: id.clone(), discard: unattended });
        let lr = &self.state.live_rack;
        if lr.id.as_deref() == Some(id.as_str()) && lr.prompt.is_none() {
            self.state.transport.acmp = true;
            self.state.ots.applied = n as u8 + 1;
            if !self.state.transport.running {
                self.state.transport.sync_start = true;
            }
        }
    }

    /// Deleting a rack gives every OTS that loaded it back to its style.
    pub(super) fn style_racks_after_rack_cmd(&mut self, c: &RackCmd) {
        if let RackCmd::DeleteRack { id } = c
            && !self.racks.entries().iter().any(|r| r.id == *id)
        {
            for slots in self.style_racks.styles.values_mut() {
                for s in slots.iter_mut() {
                    if s.as_deref() == Some(id.as_str()) {
                        *s = None;
                    }
                }
            }
            self.style_racks.styles.retain(|_, s| s.iter().any(Option::is_some));
        }
    }

}

impl MockStyleRacks {
    /// `ots.racks`, per OTS of the loaded style.
    pub(super) fn fill(&self, st: &mut AppState, racks: &[RackEntry]) {
        let key = st.style.path.rsplit('/').next().unwrap_or(&st.style.path).to_string();
        let slots = self.styles.get(&key);
        st.ots.racks = (0..st.ots.settings.len().min(4))
            .map(|i| match slots.and_then(|s| s[i].clone()) {
                Some(id) => {
                    let found = racks.iter().find(|r| r.id == id);
                    OtsRack { name: found.map(|r| r.name.clone()).unwrap_or_default(), missing: found.is_none(), rack: Some(id) }
                }
                None => OtsRack::default(),
            })
            .collect();
    }
}
