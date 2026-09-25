//! The mock session's Registration Memory and Playlist: `app/src/lib/api/mock-registration.ts`
//! in Rust, on the engine's own types (the bank, sequence and playlist logic come from
//! `yahaha::registration`, so they can't drift). Banks and playlists live in memory; two
//! demo banks and a playlist are there from the start.

use std::collections::BTreeMap;

use yahaha::api::*;
use yahaha::fingering::Fingering;
use yahaha::launchkey::{Anim, Level};
use yahaha::registration::playlist::PLAYLIST_EXT;
use yahaha::registration::{file_name, Group, Groups, BANK_EXT, Playlist, PlaylistSort, Record, RecordTarget, SeqMove, Sequence, SequenceEnd, BUTTONS};

const BANK_DIR: &str = "/Users/me/Documents/yahaha/Registration";
const LIST_DIR: &str = "/Users/me/Documents/yahaha/Playlists";

/// One button: what the engine's registrables store, flattened.
#[derive(Clone, Debug)]
struct Memory {
    name: String,
    groups: Groups,
    style: Option<(String, String)>,
    tempo: Option<f64>,
    main: Option<u8>,
    ots_link: Option<bool>,
    chord: Option<(Fingering, bool, bool, u8)>,
    /// Left Hold (the session's `chord.leftHold`).
    left_hold: Option<bool>,
    mixer: Option<(Vec<u8>, Vec<bool>)>,
    /// Right 1, Right 2, Right 3, Left: (on, program, volume, octave).
    parts: Option<Vec<Option<(bool, u8, u8, i8)>>>,
    transpose: Option<(i8, i8)>,
    /// Keyboard Harmony/Arpeggio (the session's `harmonyArp` section); its `pedalHold` is
    /// the pedal's and is never recalled.
    harmony_arp: Option<HarmonyArpState>,
}

#[derive(Clone, Debug)]
struct Bank {
    name: String,
    memories: Vec<Option<Memory>>,
    sequence: Sequence,
}

impl Bank {
    fn empty(name: &str) -> Bank {
        Bank { name: name.into(), memories: vec![None; BUTTONS], sequence: Sequence::default() }
    }
}

/// What a recall or a record load asks the session to do.
pub enum Effect {
    /// Load this style (by path).
    LoadStyle(String),
    /// Press Main n (with OTS Link held off).
    Main(u8),
    Message(String, bool),
}

pub struct MockRegist {
    banks: BTreeMap<String, Bank>,
    lists: BTreeMap<String, Playlist>,
    bank: Bank,
    path: Option<String>,
    dirty: bool,
    selected: Option<u8>,
    memory: bool,
    memorize: Groups,
    freeze: bool,
    frozen: Groups,
    seq_pos: Option<usize>,
    /// Sequence On/Off: a panel setting, not part of the bank (Genos Data List).
    seq_on: bool,
    list: Playlist,
    list_path: Option<String>,
    list_dirty: bool,
    sort: PlaylistSort,
    current: Option<usize>,
    /// A recall's sections waiting for its style and section (`apply_pending`).
    pending: Option<(Memory, Groups)>,
}

/// File names as the session makes them (`registration::file_name`: "A:B" is "A_B").
fn bank_path(name: &str) -> String {
    format!("{BANK_DIR}/{}", file_name(name, BANK_EXT))
}

fn list_path(name: &str) -> String {
    format!("{LIST_DIR}/{}", file_name(name, PLAYLIST_EXT))
}

/// The key of `files` that is `path` on a case-insensitive file system (the Mac's).
fn existing<T>(files: &BTreeMap<String, T>, path: &str) -> Option<String> {
    files.keys().find(|k| k.eq_ignore_ascii_case(path)).cloned()
}

fn demo(name: &str, style: &(String, String), tempo: f64, programs: [u8; 4], on: [bool; 4]) -> Memory {
    Memory {
        name: name.into(),
        groups: Groups::all(),
        style: Some(style.clone()),
        tempo: Some(tempo),
        main: Some(0),
        ots_link: Some(false),
        chord: Some((Fingering::FingeredOnBass, false, true, 54)),
        left_hold: Some(false),
        mixer: Some((vec![100, 100, 96, 80, 76, 70, 88, 84], vec![true; 8])),
        parts: Some((0..4).map(|i| Some((on[i], programs[i], 100, 0))).collect()),
        transpose: Some((0, 0)),
        harmony_arp: None,
    }
}

impl MockRegist {
    /// Two demo banks and a playlist, over `styles` (path, name).
    pub fn new(styles: &[(String, String)]) -> MockRegist {
        let mut m = MockRegist {
            banks: BTreeMap::new(),
            lists: BTreeMap::new(),
            bank: Bank::empty("New Bank"),
            path: None,
            dirty: false,
            selected: None,
            memory: false,
            memorize: Groups::all(),
            freeze: false,
            frozen: Groups::NONE,
            seq_pos: None,
            seq_on: true,
            list: Playlist::default(),
            list_path: None,
            list_dirty: false,
            sort: PlaylistSort::Normal,
            current: None,
            pending: None,
        };
        if styles.is_empty() {
            return m;
        }
        let st = |i: usize| &styles[i % styles.len()];
        let mut gig = Bank::empty("Friday Gig");
        gig.memories[0] = Some(demo("Intro ballad", st(0), 72.0, [0, 48, 61, 48], [true, true, false, false]));
        gig.memories[1] = Some(demo("Verse", st(1), 96.0, [4, 48, 61, 32], [true, false, false, false]));
        gig.memories[2] = Some(demo("Chorus", st(1), 96.0, [61, 48, 56, 32], [true, true, true, false]));
        gig.memories[3] = Some(demo("Swing", st(2), 132.0, [26, 48, 65, 32], [true, false, true, false]));
        gig.sequence = Sequence { steps: vec![0, 1, 2, 1, 2, 3], end: SequenceEnd::Next };
        let mut jazz = Bank::empty("Jazz Set");
        jazz.memories[0] = Some(demo("Trio", st(3), 120.0, [0, 32, 48, 32], [true, false, false, false]));
        let mut list = Playlist::new("Friday");
        let rec = |name: &str, target| Record { name: name.into(), target };
        list.records = vec![
            rec("Opener", RecordTarget::Bank { path: bank_path("Friday Gig"), regist: Some(0) }),
            rec("Swing number", RecordTarget::Bank { path: bank_path("Friday Gig"), regist: Some(3) }),
            rec("Jazz Set", RecordTarget::Bank { path: bank_path("Jazz Set"), regist: None }),
            rec(&st(4).1, RecordTarget::Style { path: st(4).0.clone() }),
        ];
        m.banks.insert(bank_path("Friday Gig"), gig);
        m.banks.insert(bank_path("Jazz Set"), jazz);
        m.lists.insert(list_path("Friday"), list);
        m.load_bank(&bank_path("Friday Gig"));
        m.load_list(&list_path("Friday"));
        m
    }

    fn load_bank(&mut self, path: &str) -> bool {
        let Some(b) = self.banks.get(path) else { return false };
        self.bank = b.clone();
        self.path = Some(path.into());
        self.dirty = false;
        self.selected = None;
        self.memory = false;
        self.seq_pos = None;
        true
    }

    fn load_list(&mut self, path: &str) -> bool {
        let Some(l) = self.lists.get(path) else { return false };
        self.list = l.clone();
        self.list_path = Some(path.into());
        self.list_dirty = false;
        self.sort = PlaylistSort::Normal;
        self.current = None;
        true
    }

    /// A saved bank is written at once; a new one waits for a name.
    fn changed(&mut self) {
        self.dirty = true;
        if let Some(p) = &self.path {
            self.banks.insert(p.clone(), self.bank.clone());
            self.dirty = false;
        }
    }

    fn step_bank(&mut self, delta: i8) -> bool {
        let paths: Vec<String> = self.banks.keys().cloned().collect();
        if paths.is_empty() {
            return false;
        }
        let next = match self.path.as_ref().and_then(|p| paths.iter().position(|x| x == p)) {
            Some(p) => {
                let q = p as i64 + delta.signum() as i64;
                if q < 0 || q >= paths.len() as i64 {
                    return false;
                }
                q as usize
            }
            None if delta >= 0 => 0,
            None => paths.len() - 1,
        };
        self.load_bank(&paths[next])
    }

    pub fn registration_cmd(&mut self, c: RegistrationCmd, st: &AppState) -> Vec<Effect> {
        let mut fx = Vec::new();
        match c {
            RegistrationCmd::PressRegist { index } if self.memory => self.memorize_into(index, st, &mut fx),
            RegistrationCmd::PressRegist { index } | RegistrationCmd::RecallRegist { index } => self.recall(index, st, true, &mut fx),
            RegistrationCmd::MemorizeRegist { index } => self.memorize_into(index, st, &mut fx),
            RegistrationCmd::ToggleRegistMemory => self.memory = !self.memory,
            RegistrationCmd::SetMemorizeGroup { group, on } => self.memorize.set(group, on),
            RegistrationCmd::ClearRegist { index } => {
                if let Some(m) = self.bank.memories.get_mut(index as usize) {
                    *m = None;
                    if self.selected == Some(index) {
                        self.selected = None;
                    }
                    self.changed();
                }
            }
            RegistrationCmd::RenameRegist { index, name } => match self.bank.memories.get_mut(index as usize).and_then(|m| m.as_mut()) {
                Some(m) => {
                    m.name = name.trim().into();
                    self.changed();
                }
                None => fx.push(Effect::Message(format!("Registration {} is empty", index + 1), true)),
            },
            RegistrationCmd::StepRegistBank { delta } => {
                if self.banks.is_empty() {
                    fx.push(Effect::Message("no Registration banks saved yet".into(), true));
                } else if self.step_bank(delta) {
                    fx.push(Effect::Message(format!("Bank: {}", self.bank.name), false));
                }
            }
            RegistrationCmd::SelectRegistBank { path } => {
                if !self.load_bank(&path) {
                    fx.push(Effect::Message(format!("{path}: not found"), true));
                }
            }
            RegistrationCmd::NewRegistBank => {
                self.bank = Bank::empty("New Bank");
                self.path = None;
                self.dirty = false;
                self.selected = None;
                self.seq_pos = None;
            }
            RegistrationCmd::SaveRegistBank { name, overwrite } => {
                if name.is_some() || self.path.is_none() {
                    let n = name.unwrap_or_else(|| self.bank.name.clone());
                    let n: String = if n.trim().is_empty() { "Untitled".into() } else { n.trim().into() };
                    let path = bank_path(&n);
                    let there = existing(&self.banks, &path);
                    if there.is_some() && self.path != there && !overwrite {
                        let text = format!("a bank called {n} already exists: save under another name, or overwrite it");
                        fx.push(Effect::Message(text, true));
                        return fx;
                    }
                    // The same file in another case: renamed to the case typed.
                    if let Some(t) = there {
                        self.banks.remove(&t);
                    }
                    self.bank.name = n;
                    self.path = Some(path);
                }
                self.banks.insert(self.path.clone().unwrap(), self.bank.clone());
                self.dirty = false;
                fx.push(Effect::Message(format!("Saved bank {}", self.bank.name), false));
            }
            RegistrationCmd::SetFreeze { on } => self.freeze = on,
            RegistrationCmd::ToggleFreeze => self.freeze = !self.freeze,
            RegistrationCmd::SetFreezeGroup { group, on } => self.frozen.set(group, on),
            RegistrationCmd::SetRegistSequence { steps, end } => {
                self.bank.sequence = Sequence { steps, end }.clean();
                self.seq_pos = None;
                self.changed();
            }
            RegistrationCmd::SetRegistSequenceOn { on } => self.seq_on = on,
            RegistrationCmd::ToggleRegistSequence => self.seq_on = !self.seq_on,
            RegistrationCmd::StepRegistSequence { delta } => {
                if !self.seq_on {
                    fx.push(Effect::Message("Registration Sequence is off".into(), true));
                    return fx;
                }
                let target = match self.bank.sequence.step(self.seq_pos, delta) {
                    SeqMove::Stay => None,
                    SeqMove::Step(p) => Some(p),
                    SeqMove::NextBank | SeqMove::PrevBank => {
                        if self.step_bank(delta) && !self.bank.sequence.steps.is_empty() {
                            Some(if delta > 0 { 0 } else { self.bank.sequence.steps.len() - 1 })
                        } else {
                            None
                        }
                    }
                };
                if let Some(p) = target {
                    self.seq_pos = Some(p);
                    let b = self.bank.sequence.steps[p];
                    self.recall(b, st, false, &mut fx);
                }
            }
        }
        fx
    }

    fn memorize_into(&mut self, index: u8, st: &AppState, fx: &mut Vec<Effect>) {
        self.memory = false;
        let g = self.memorize;
        if (index as usize) >= BUTTONS {
            return;
        }
        if g.is_empty() {
            fx.push(Effect::Message("Memorize: no groups ticked".into(), true));
            return;
        }
        let style = g.has(Group::Style);
        let c = &st.chord;
        let m = Memory {
            name: if style { st.style.name.clone() } else { format!("Registration {}", index + 1) },
            groups: g,
            style: style.then(|| (st.style.path.clone(), st.style.name.clone())),
            tempo: g.has(Group::Tempo).then_some(st.transport.tempo),
            main: style.then_some(st.transport.main),
            ots_link: style.then_some(st.ots.link),
            chord: style.then_some((c.fingering, c.upper, c.manual_bass, c.split)),
            left_hold: style.then_some(c.left_hold),
            mixer: style.then(|| (st.mixer.style_parts.iter().map(|p| p.volume).collect(), st.mixer.style_parts.iter().map(|p| p.on).collect())),
            parts: (style || g.has(Group::Voice)).then(|| {
                st.keyboard_parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| g.has(if i == 3 { Group::Style } else { Group::Voice }).then_some((p.on, p.program, p.volume, p.octave)))
                    .collect()
            }),
            transpose: g.has(Group::Transpose).then_some((c.transpose_keyboard, c.transpose_master)),
            harmony_arp: g.has(Group::HarmonyArp).then(|| st.harmony_arp.clone()),
        };
        self.bank.memories[index as usize] = Some(m);
        self.selected = Some(index);
        fx.push(Effect::Message(format!("Memorized to Registration {}", index + 1), false));
        self.changed();
    }

    /// Recall button `index`. The style load and the Main press come back as effects;
    /// everything else is written to `apply` for the session to copy in.
    fn recall(&mut self, index: u8, st: &AppState, follow: bool, fx: &mut Vec<Effect>) {
        let Some(m) = self.bank.memories.get(index as usize).cloned().flatten() else {
            fx.push(Effect::Message(format!("Registration {} is empty", index + 1), true));
            return;
        };
        self.memory = false;
        self.selected = Some(index);
        if follow {
            self.seq_pos = self.bank.sequence.follow(self.seq_pos, index);
        }
        let allowed = if self.freeze { m.groups.minus(self.frozen) } else { m.groups };
        if allowed.has(Group::Style) {
            if let Some((path, _)) = &m.style {
                if *path != st.style.path {
                    fx.push(Effect::LoadStyle(path.clone()));
                }
            }
            if let Some(main) = m.main {
                if main != st.transport.main {
                    fx.push(Effect::Main(main));
                }
            }
        }
        self.pending = Some((m.clone(), allowed));
        fx.push(Effect::Message(format!("Registration {}: {}", index + 1, m.name), false));
    }

    /// The rest of a recall, once the style and the section are in (the session calls it
    /// after running the effects).
    pub fn apply_pending(&mut self, st: &mut AppState, tempo_before: f64) {
        let Some((m, allowed)) = self.pending.take() else { return };
        if m.groups.has(Group::Tempo) && !allowed.has(Group::Tempo) {
            st.transport.tempo = tempo_before;
        }
        if let (true, Some(t)) = (allowed.has(Group::Tempo), m.tempo) {
            st.transport.tempo = t;
        }
        if allowed.has(Group::Style) {
            if let Some((f, upper, mb, split)) = m.chord {
                // Parameter Lock: a locked group keeps what the player set.
                if !st.param_locks.get(LockItem::FingeringType) {
                    st.chord.fingering = f;
                    st.chord.upper = upper;
                    st.chord.manual_bass = mb;
                }
                if !st.param_locks.get(LockItem::SplitPoint) {
                    st.chord.split = split;
                }
            }
            if let Some(on) = m.left_hold {
                st.chord.left_hold = on;
            }
            if let Some(link) = m.ots_link {
                st.ots.link = link;
            }
            if let Some((vols, on)) = &m.mixer {
                for (i, p) in st.mixer.style_parts.iter_mut().enumerate() {
                    p.volume = vols[i];
                    p.on = on[i];
                }
            }
        }
        if let Some(parts) = &m.parts {
            for (i, p) in parts.iter().enumerate() {
                let g = if i == 3 { Group::Style } else { Group::Voice };
                if let (Some((on, program, volume, octave)), true) = (p, allowed.has(g)) {
                    let k = &mut st.keyboard_parts[i];
                    (k.on, k.program, k.volume, k.octave) = (*on, *program, *volume, *octave);
                }
            }
        }
        if let (true, Some((k, mst))) = (allowed.has(Group::Transpose), m.transpose) {
            st.chord.transpose_keyboard = k;
            st.chord.transpose_master = mst;
        }
        if let (true, Some(h)) = (allowed.has(Group::HarmonyArp), &m.harmony_arp) {
            let pedal_hold = st.harmony_arp.arp.pedal_hold;
            st.harmony_arp = h.clone();
            st.harmony_arp.arp.pedal_hold = pedal_hold;
        }
    }

    pub fn playlist_cmd(&mut self, c: PlaylistCmd, st: &AppState) -> Vec<Effect> {
        let mut fx = Vec::new();
        let sorted = self.sort != PlaylistSort::Normal;
        match c {
            PlaylistCmd::NewPlaylist => {
                self.list = Playlist::default();
                self.list_path = None;
                self.list_dirty = false;
                self.sort = PlaylistSort::Normal;
                self.current = None;
            }
            PlaylistCmd::LoadPlaylist { path } => {
                if !self.load_list(&path) {
                    fx.push(Effect::Message(format!("{path}: not found"), true));
                }
            }
            PlaylistCmd::SavePlaylist { name, overwrite } => {
                if let Some(n) = name.as_ref().map(|n| n.trim()).filter(|n| !n.is_empty()) {
                    let path = list_path(n);
                    let there = existing(&self.lists, &path);
                    if there.is_some() && self.list_path != there && !overwrite {
                        let text = format!("a playlist called {n} already exists: save under another name, or overwrite it");
                        fx.push(Effect::Message(text, true));
                        return fx;
                    }
                    if let Some(t) = there {
                        self.lists.remove(&t);
                    }
                }
                if sorted {
                    let order = self.list.order(self.sort);
                    self.current = self.current.and_then(|c| order.iter().position(|&i| i == c));
                    self.list.apply_order(self.sort);
                    self.sort = PlaylistSort::Normal;
                }
                if name.is_some() || self.list_path.is_none() {
                    let n = name.unwrap_or_else(|| self.list.name.clone());
                    self.list.name = if n.trim().is_empty() { "Untitled".into() } else { n.trim().into() };
                    self.list_path = Some(list_path(&self.list.name));
                }
                self.lists.insert(self.list_path.clone().unwrap(), self.list.clone());
                self.list_dirty = false;
                fx.push(Effect::Message(format!("Saved playlist {}", self.list.name), false));
            }
            PlaylistCmd::AddPlaylistRecord { record } => self.add(record, &mut fx),
            PlaylistCmd::AddCurrentBank => match self.path.clone() {
                Some(path) => {
                    let name = match self.selected {
                        Some(i) => format!("{} [{}]", self.bank.name, i + 1),
                        None => self.bank.name.clone(),
                    };
                    self.add(Record { name, target: RecordTarget::Bank { path, regist: self.selected } }, &mut fx);
                }
                None => fx.push(Effect::Message("save the bank first: a Playlist record links to a bank file".into(), true)),
            },
            PlaylistCmd::AddCurrentStyle => {
                self.add(Record { name: st.style.name.clone(), target: RecordTarget::Style { path: st.style.path.clone() } }, &mut fx)
            }
            PlaylistCmd::AppendPlaylist { path } => match self.lists.get(&path).cloned() {
                Some(other) => {
                    for r in other.records {
                        self.add(r, &mut fx);
                    }
                }
                None => fx.push(Effect::Message(format!("{path}: not found"), true)),
            },
            PlaylistCmd::SetPlaylistRecord { index, record } => {
                if let Some(r) = self.list.records.get_mut(index) {
                    *r = record;
                    self.list_dirty = true;
                }
            }
            PlaylistCmd::MovePlaylistRecord { index, delta } => {
                if sorted {
                    fx.push(Effect::Message("sort the playlist back to Normal to move records".into(), true));
                } else if self.list.move_record(index, delta as i32) {
                    self.list_dirty = true;
                }
            }
            PlaylistCmd::DeletePlaylistRecord { index } => {
                if sorted {
                    fx.push(Effect::Message("sort the playlist back to Normal to delete records".into(), true));
                } else if index < self.list.records.len() {
                    self.list.records.remove(index);
                    self.current = match self.current {
                        Some(c) if c == index => None,
                        Some(c) if c > index => Some(c - 1),
                        c => c,
                    };
                    self.list_dirty = true;
                }
            }
            PlaylistCmd::SetPlaylistSort { sort } => self.sort = sort,
            PlaylistCmd::LoadPlaylistRecord { index } => self.load_record(index, st, &mut fx),
            PlaylistCmd::StepPlaylist { delta } => {
                let order = self.list.order(self.sort);
                if !order.is_empty() {
                    let pos = self.current.and_then(|c| order.iter().position(|&i| i == c));
                    let next = match pos {
                        None if delta >= 0 => 0,
                        None => order.len() - 1,
                        Some(p) => (p as i64 + delta.signum() as i64).clamp(0, order.len() as i64 - 1) as usize,
                    };
                    if Some(next) != pos {
                        self.load_record(order[next], st, &mut fx);
                    }
                }
            }
        }
        fx
    }

    fn add(&mut self, r: Record, fx: &mut Vec<Effect>) {
        if !self.list.push(r) {
            fx.push(Effect::Message("a playlist holds at most 2500 records".into(), true));
        } else {
            self.list_dirty = true;
        }
    }

    fn load_record(&mut self, index: usize, st: &AppState, fx: &mut Vec<Effect>) {
        let Some(r) = self.list.records.get(index).cloned() else { return };
        self.current = Some(index);
        match r.target {
            RecordTarget::Bank { path, regist } => {
                if !self.load_bank(&path) {
                    fx.push(Effect::Message(format!("{path}: not found"), true));
                    return;
                }
                if let Some(i) = regist {
                    self.recall(i, st, true, fx);
                }
            }
            RecordTarget::Style { path } => fx.push(Effect::LoadStyle(path)),
        }
        fx.push(Effect::Message(format!("Playlist {}: {}", index + 1, r.name), false));
    }

    /// `state.registration` and `state.playlist`.
    pub fn fill(&self, st: &mut AppState) {
        let paths: Vec<&String> = self.banks.keys().collect();
        st.registration = RegistrationState {
            bank: BankState {
                name: self.bank.name.clone(),
                path: self.path.clone(),
                dirty: self.dirty,
                position: self.path.as_ref().and_then(|p| paths.iter().position(|x| *x == p)),
            },
            banks: self.banks.iter().map(|(p, b)| BankFile { name: b.name.clone(), path: p.clone() }).collect(),
            folder: Some(BANK_DIR.into()),
            buttons: self
                .bank
                .memories
                .iter()
                .enumerate()
                .map(|(i, m)| match m {
                    None => RegistButton { index: i as u8, ..RegistButton::default() },
                    Some(m) => RegistButton {
                        index: i as u8,
                        stored: true,
                        name: m.name.clone(),
                        groups: m.groups,
                        style: m.style.as_ref().map(|s| s.1.clone()),
                        tempo: m.tempo,
                        voices: m
                            .parts
                            .as_ref()
                            .map(|ps| {
                                ps.iter()
                                    .map(|p| match p {
                                        Some((on, program, _, _)) => RegistVoice { name: gm_name(*program).into(), on: *on },
                                        None => RegistVoice::default(),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    },
                })
                .collect(),
            selected: self.selected,
            memory: self.memory,
            memorize_groups: self.memorize,
            freeze: self.freeze,
            freeze_groups: self.frozen,
            sequence: SequenceState {
                on: self.seq_on,
                steps: self.bank.sequence.steps.clone(),
                end: self.bank.sequence.end,
                position: self.seq_pos,
            },
            pending: false,
        };
        st.playlist = PlaylistState {
            name: self.list.name.clone(),
            path: self.list_path.clone(),
            dirty: self.list_dirty,
            sort: self.sort,
            records: self
                .list
                .order(self.sort)
                .into_iter()
                .map(|i| {
                    let record = self.list.records[i].clone();
                    let missing = matches!(&record.target, RecordTarget::Bank { path, .. } if !self.banks.contains_key(path));
                    PlaylistRow { index: i, record, missing }
                })
                .collect(),
            current: self.current,
            playlists: self.lists.iter().map(|(p, l)| PlaylistFileEntry { name: l.name.clone(), path: p.clone() }).collect(),
            folder: Some(LIST_DIR.into()),
        };
    }

    /// Page 4's pads (src/launchkey.rs `regist_looks`).
    pub fn pads(&self) -> Vec<Pad> {
        const SELECTED: [u8; 3] = [127, 0, 0];
        const STORED: [u8; 3] = [0, 40, 127];
        const PAGE: [u8; 3] = [127, 60, 0];
        const KEYS: [&str; 10] = ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"];
        let pad = |note: u8, label: String, key: &str, action: AppCmd, (rgb, level, anim): ([u8; 3], Level, Anim)| Pad {
            note,
            label,
            key: key.into(),
            rgb,
            level,
            anim,
            action: Some(action),
            palette: None,
        };
        let page = |on: bool, avail: bool| (PAGE, if !avail { Level::Off } else if on { Level::Bright } else { Level::Dim }, Anim::Solid);
        let seq = self.seq_on && !self.bank.sequence.steps.is_empty();
        let banks = !self.banks.is_empty();
        let mut v: Vec<Pad> = (0..10u8)
            .map(|i| {
                let stored = self.bank.memories[i as usize].is_some();
                let look = if self.memory {
                    (SELECTED, Level::Bright, Anim::Flash)
                } else if stored && self.selected == Some(i) {
                    (SELECTED, Level::Bright, Anim::Solid)
                } else {
                    (STORED, if stored { Level::Bright } else { Level::Off }, Anim::Solid)
                };
                let note = if i < 8 { 96 + i } else { 104 + i };
                pad(note, format!("REGIST {}", i + 1), KEYS[i as usize], RegistrationCmd::PressRegist { index: i }.into(), look)
            })
            .collect();
        v.extend([
            pad(114, "BANK -".into(), "F11", RegistrationCmd::StepRegistBank { delta: -1 }.into(), page(false, banks)),
            pad(115, "BANK +".into(), "F12", RegistrationCmd::StepRegistBank { delta: 1 }.into(), page(false, banks)),
            pad(
                116,
                "MEMORY".into(),
                "F5",
                RegistrationCmd::ToggleRegistMemory.into(),
                if self.memory { (SELECTED, Level::Bright, Anim::Flash) } else { page(false, true) },
            ),
            pad(117, "FREEZE".into(), "F6", RegistrationCmd::ToggleFreeze.into(), page(self.freeze, true)),
            pad(118, "REGIST -".into(), "F7", RegistrationCmd::StepRegistSequence { delta: -1 }.into(), page(false, seq)),
            pad(119, "REGIST +".into(), "F8", RegistrationCmd::StepRegistSequence { delta: 1 }.into(), page(false, seq)),
        ]);
        v
    }

    pub fn has_songs(&self) -> bool {
        !self.list.records.is_empty()
    }
}
