#!/usr/bin/env python3
# Apply the #104 plugin registrable edits (run from the worktree root).

def edit(p, pairs):
    s = open(p).read()
    for old, new in pairs:
        assert old in s, (p, old[:80])
        s = s.replace(old, new, 1)
    open(p, 'w').write(s)

# ----- VoiceRef::Plugin -----
edit('src/registration/mod.rs', [
("""        #[serde(default)]
        bank_lsb: u8,
    },
}
""", """        #[serde(default)]
        bank_lsb: u8,
    },
    /// An instrument plugin on the part (#91's `setPartPlugin`, #104): its id
    /// (`"aumu dls  appl"`), its name for Regist Bank Info, its full state (base64; none =
    /// its default preset), and the GM voice the part has underneath, which a build or a
    /// Mac without the plugin plays instead.
    Plugin {
        id: String,
        #[serde(default)]
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<String>,
        #[serde(default)]
        program: u8,
    },
}
"""),
("""    /// The GM program to play, when this build can play the voice.
    pub fn program(&self) -> Option<u8> {
        match self {
            VoiceRef::Gm { program, .. } => Some(*program & 127),
        }
    }""", """    /// The GM program to play, when this build can play the voice: a plugin's is the GM
    /// voice underneath it.
    pub fn program(&self) -> Option<u8> {
        match self {
            VoiceRef::Gm { program, .. } | VoiceRef::Plugin { program, .. } => Some(*program & 127),
        }
    }"""),
("""        assert!(serde_json::from_str::<VoiceRef>(r#"{"kind":"plugin","id":"x"}"#).is_err());""",
"""        assert!(serde_json::from_str::<VoiceRef>(r#"{"kind":"clap","id":"x"}"#).is_err());
        let p: VoiceRef = serde_json::from_str(r#"{"kind":"plugin","id":"aumu dls  appl","program":4}"#).unwrap();
        assert_eq!(p, VoiceRef::Plugin { id: "aumu dls  appl".into(), name: String::new(), state: None, program: 4 });
        assert_eq!(p.program(), Some(4));"""),
])

edit('src/session/registration/tests.rs', [
("""    v["memories"][0]["sections"]["parts"]["parts"][0]["voice"] = serde_json::json!({ "kind": "plugin", "id": "au.x", "state": "..." });""",
 """    v["memories"][0]["sections"]["parts"]["parts"][0]["voice"] = serde_json::json!({ "kind": "clap", "id": "au.x", "state": "..." });"""),
])

# ----- the parts registrable -----
edit('src/session/registration/sections.rs', [
("""            voice: Some(VoiceRef::gm(kp.program[p].load(Relaxed))),""",
 """            voice: Some(c.part_plugin_reg(p).unwrap_or_else(|| VoiceRef::gm(kp.program[p].load(Relaxed)))),"""),
("""        match part.voice.as_ref().and_then(VoiceRef::program) {
            Some(prog) => {
                kp.set_program(p, prog);
                if let Err(e) = recall_patch(c, p, part.patch.as_ref()) {
                    err = Some(e);
                }
            }
            None => err = Some(format!("{}: voice not available", parts::NAMES[p])),
        }""", """        match part.voice.as_ref() {
            Some(v) => {
                kp.set_program(p, v.program().unwrap_or(0));
                let r = match v {
                    VoiceRef::Plugin { id, name, state, .. } => c.recall_part_plugin(p, id, name, state.as_deref()),
                    VoiceRef::Gm { .. } => {
                        // A GM voice: no plugin from the Plugins tab (a library patch's own
                        // plugin is the patch's business, below).
                        c.clear_part_tab_plugin(p);
                        recall_patch(c, p, part.patch.as_ref())
                    }
                };
                if let Err(e) = r {
                    err = Some(e);
                }
            }
            None => err = Some(format!("{}: voice not available", parts::NAMES[p])),
        }"""),
("""                    Some(p) => match &p.patch {
                        Some(patch) => (patch.name.clone(), p.on),
                        None => (p.voice.as_ref().and_then(VoiceRef::program).map_or("?", gm_name).to_string(), p.on),
                    },""", """                    Some(p) => match (&p.patch, &p.voice) {
                        (Some(patch), _) => (patch.name.clone(), p.on),
                        (None, Some(VoiceRef::Plugin { id, name, .. })) => (if name.is_empty() { id.clone() } else { name.clone() }, p.on),
                        (None, v) => (v.as_ref().and_then(VoiceRef::program).map_or("?", gm_name).to_string(), p.on),
                    },"""),
])

# ----- memorize reads the plugins' states fresh -----
edit('src/session/registration.rs', [
("""        let mut m = Memory { name: String::new(), groups, ..Memory::default() };
        for r in REGISTRABLES {""", """        // The parts' plugins as they sound now, not as last autosaved.
        if groups.has(Group::Voice) || groups.has(Group::Style) {
            self.refresh_part_plugin_states();
        }
        let mut m = Memory { name: String::new(), groups, ..Memory::default() };
        for r in REGISTRABLES {"""),
])
