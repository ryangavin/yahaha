p = 'src/session/sound_library.rs'
s = open(p).read()


def rep(a, b):
    global s
    assert s.count(a) == 1, a
    s = s.replace(a, b)


rep("""    fn save_part_as_patch(&mut self, part: usize, name: Option<String>) -> Result<(), CmdError> {
        let kp = self.shared.parts.clone();
        let mut p = match self.sound.part_patch[part].clone().and_then(|id| self.sound.lib.patch(&id).cloned()) {
            Some(own) => own,
            None => {
                let Some(file) = self.sf_file.clone() else { return self.sl_fail("the synth plays no SoundFont") };
                let program = kp.channel_program(part);
                Patch {
                    id: String::new(),
                    name: crate::api::gm_name(program).to_string(),
                    category: Category::guess(0, program),
                    tags: Vec::new(),
                    favourite: false,
                    source: PatchSource::SoundFont { file, bank: 0, program },
                    defaults: PatchDefaults::default(),
                }
            }
        };""", """    /// Save what keyboard part `part` plays as a new patch: its plugin (the component id
    /// and its state now, as its editor left it), else its own patch, else the patch the
    /// map sends its GM voice to, else that GM voice on the synth's SoundFont. Its volume
    /// and octave become the defaults.
    fn save_part_as_patch(&mut self, part: usize, name: Option<String>) -> Result<(), CmdError> {
        let kp = self.shared.parts.clone();
        let program = kp.channel_program(part);
        let own = self.sound.part_patch[part].clone().and_then(|id| self.sound.lib.patch(&id).cloned());
        let mapped = || {
            let style = self.sound.lib.style_maps.get(&self.sound.cur_key);
            patches::resolve(&self.sound.lib.map, style, false, program).patch.and_then(|id| self.sound.lib.patch(id)).cloned()
        };
        let plays = own.clone().or_else(mapped);
        let mut p = match (self.channel_plugin_now(parts::CHANNEL[part]), plays) {
            (Some((voice, plugin_name)), plays) => {
                let state = voice.state.as_deref().map(crate::api::base64_encode).unwrap_or_default();
                let source = PatchSource::Plugin { component_id: voice.id, state };
                match own.filter(|q| matches!(q.source, PatchSource::Plugin { .. })) {
                    // Its own plugin patch, with the plugin's state as it is now.
                    Some(q) => Patch { source, ..q },
                    None => Patch {
                        id: String::new(),
                        name: plugin_name,
                        category: plays.map_or_else(|| Category::guess(0, program), |q| q.category),
                        tags: Vec::new(),
                        favourite: false,
                        source,
                        defaults: PatchDefaults::default(),
                    },
                }
            }
            (None, Some(q)) => q,
            (None, None) => {
                let Some(file) = self.sf_file.clone() else { return self.sl_fail("the synth plays no SoundFont") };
                Patch {
                    id: String::new(),
                    name: crate::api::gm_name(program).to_string(),
                    category: Category::guess(0, program),
                    tags: Vec::new(),
                    favourite: false,
                    source: PatchSource::SoundFont { file, bank: 0, program },
                    defaults: PatchDefaults::default(),
                }
            }
        };""")
open(p, 'w').write(s)

p = 'src/session/plugins.rs'
s = open(p).read()
rep("""        pub(crate) fn channel_plugin_state(&self, ch: u8) -> Option<PartPlugin> {""", """        /// What channel `ch`'s plugin is now, and its name: the playing instance's state
        /// as its editor left it (read here), else the state it was chosen with.
        pub(crate) fn channel_plugin_now(&self, ch: u8) -> Option<(PluginVoice, String)> {
            let c = self.plugins.channels[(ch & 15) as usize].as_ref()?;
            let mut voice = c.voice.clone();
            if c.status == PluginStatus::Playing
                && let Some(Ok(s)) = c.editor.as_ref().map(|e| e.state())
            {
                voice.state = Some(s);
            }
            Some((voice, c.name()))
        }

        pub(crate) fn channel_plugin_state(&self, ch: u8) -> Option<PartPlugin> {""")
rep("""    pub(crate) fn channel_plugin_state(&self, _ch: u8) -> Option<PartPlugin> {
        None
    }""", """    pub(crate) fn channel_plugin_state(&self, _ch: u8) -> Option<PartPlugin> {
        None
    }
    pub(crate) fn channel_plugin_now(&self, _ch: u8) -> Option<(PluginVoice, String)> {
        None
    }""")
open(p, 'w').write(s)

p = 'docs/app-api.md'
s = open(p).read()
rep("| `savePartAsPatch` | `part` 0–3, `name` or null | Saves a keyboard part's sound as a new patch: its own patch, else its GM voice on the synth's SoundFont, with its volume and octave as defaults. |",
    "| `savePartAsPatch` | `part` 0–3, `name` or null | Saves what a keyboard part plays as a new patch: its plugin (component id and its state now, after editing), else its own patch, else the patch the program map sends its GM voice to, else its GM voice on the synth's SoundFont. Its volume and octave become the defaults. |")
open(p, 'w').write(s)
print("ok")
