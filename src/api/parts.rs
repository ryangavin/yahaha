//! Keyboard parts (Right 1-3, Left).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PartsCmd {
    /// Turn a part on/off. Left is refused under Manual Bass (it plays the bass then).
    SetPartOn { part: u8, on: bool },
    TogglePart { part: u8 },
    /// The part the voice commands (`StepVoice`) and the Launchkey voice pads edit.
    SelectPart { part: u8 },
    /// Set a part's voice (GM program 0-127).
    SetPartVoice { part: u8, program: u8 },
    /// Previous/next voice for the selected part.
    StepVoice { delta: i8 },
    /// A part's volume (its CC7, 0-127). The Launchkey fader picks it up.
    SetPartVolume { part: u8, volume: u8 },
    /// A part's octave shift (-2..=2).
    SetPartOctave { part: u8, octave: i8 },
    /// A part's pan (its CC10: 0 = left, 64 = centre, 127 = right).
    SetPartPan { part: u8, pan: u8 },
    /// A part's reverb, chorus or variation (delay) send depth (its CC91, CC93 or CC94,
    /// 0-127).
    SetPartSend { part: u8, send: PartSend, value: u8 },
}

/// A keyboard part's effect send (Genos Mixer > Effect: Reverb and Chorus depth).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PartSend {
    /// CC91.
    Reverb,
    /// CC93.
    Chorus,
    /// CC94: the effect bus's Variation block, the tempo delay (#204).
    Variation,
}

impl PartSend {
    /// Its index in `Parts::fx`.
    pub fn index(self) -> usize {
        match self {
            PartSend::Reverb => crate::parts::REVERB,
            PartSend::Chorus => crate::parts::CHORUS,
            PartSend::Variation => crate::parts::VARIATION,
        }
    }
}

/// A keyboard part: Right 1, Right 2, Right 3 or Left.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardPart {
    /// "Right 1", "Right 2", "Right 3", "Left".
    pub name: String,
    /// MIDI channel, 1-based (Right 1 = 1, Left = 2, Right 2 = 3, Right 3 = 4).
    pub channel: u8,
    /// The part's on/off switch.
    pub on: bool,
    /// It sounds: on, or Left playing the bass under Manual Bass.
    pub sounding: bool,
    /// The part the voice commands edit.
    pub selected: bool,
    /// Volume (its CC7), 0-127.
    pub volume: u8,
    /// The Launchkey fader has moved but not yet reached `volume` (soft takeover).
    pub waiting: bool,
    /// Where its Launchkey fader (Panel page, faders 1-4) physically is; None if it hasn't
    /// moved.
    pub fader: Option<u8>,
    /// The part's own voice, a GM program 0-127.
    pub program: u8,
    /// What its channel plays: its voice, or the Style's Bass voice under Manual Bass.
    pub voice_name: String,
    /// Left playing the Style's Bass voice (Manual Bass).
    pub plays_bass: bool,
    /// The octave setting, -2..=2 (not applied while `plays_bass`).
    pub octave: i8,
    /// Pan (CC10): 0 = left, 64 = centre, 127 = right.
    pub pan: u8,
    /// Reverb send depth (CC91), 0-127.
    pub reverb: u8,
    /// Chorus send depth (CC93), 0-127.
    pub chorus: u8,
    /// Variation (tempo delay) send depth (CC94), 0-127 (#204).
    #[serde(default)]
    pub variation: u8,
    /// The instrument plugin the part plays instead of its SoundFont voice (absent: the
    /// SoundFont voice). Its fader is the same CC7.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<super::PartPlugin>,
    /// Its own sound library patch (`setPartPatch`), if it has one; else its GM voice
    /// plays, through the program map (`voiceName` names the patch it resolves to).
    #[serde(default)]
    pub patch: Option<String>,
    /// The Sound it plays, as the Sounds dialog's catalog names it ([`part_sound`]): its
    /// plugin's preset, library sound or the bare plugin; else (no plugin, or one that
    /// failed) its own SoundFont patch or what the GM map resolves its voice to. Absent: a
    /// GM voice nothing covers. The footer reads "<name> plays <instrument> · <sound>".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<crate::patches::SoundTag>,
    /// Its plugin's state no longer matches `sound` (edited in the plugin's editor, or a
    /// recalled edit): Save or Save as… keeps it. Left out when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sound_edited: bool,
}

/// What a keyboard part plays, named: the Sound (a catalog entry id and its name) and the
/// `voiceName` every panel shows. Built by [`part_sound`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartSound {
    /// `saved:<patch>`, `sf:<file>:<bank>:<program>`, `au:<component>#<preset key>` or
    /// `au:<component>`; None when nothing covers the part's GM voice.
    pub sound: Option<crate::patches::SoundTag>,
    pub voice_name: String,
}

/// What [`part_sound`] reads of a keyboard part.
#[derive(Clone, Copy, Debug, Default)]
pub struct PartSoundOf<'a> {
    /// Its plugin, if it has one.
    pub plugin: Option<&'a super::PartPlugin>,
    /// The library Sound the plugin's voice names (a stored tag: its name may be stale).
    pub plugin_sound: Option<&'a crate::patches::SoundTag>,
    /// Its own library patch (None under Manual Bass).
    pub own: Option<&'a str>,
    /// The GM program its channel plays.
    pub program: u8,
}

/// The Sound a keyboard part plays and its voice name, from what actually sounds. The
/// engine and the dev mock both name parts through this; its twin for the web is
/// `app/src/lib/api/part-sound.ts`.
///
/// - A plugin that is loading, playing or muted: the preset it was given (unless its sound
///   is another library sound, one the user saved), else its library sound, named as the
///   library names it now (a rename shows at once; a deleted one is no longer named), else
///   the bare plugin. The voice name is a preset's (or a factory or file preset's
///   sound's) "<plugin> · <preset>", else the user's sound's name, or the plugin's.
/// - A plugin that failed plays the SoundFont voice, as a part with no plugin does: its own
///   SoundFont patch, else what the GM map (with its auto-fill) resolves its program to;
///   `font_name` names a font preset. A program nothing covers has no Sound, only its GM
///   name.
pub fn part_sound(
    lib: &crate::patches::SoundLibrary,
    style: Option<&str>,
    auto: &crate::patches::AutoFill,
    part: PartSoundOf,
    font_name: &dyn Fn(&crate::patches::FontPreset) -> Option<String>,
) -> PartSound {
    use crate::patches::{Patch, PatchSource, PluginOrigin, SoundId, SoundTag};
    let library = |id: &str| match SoundId::parse(id) {
        Some(SoundId::Library(id)) => lib.patch(&id),
        _ => None,
    };
    if let Some(pl) = part.plugin.filter(|p| p.status != super::PluginStatus::Failed) {
        let saved = part.plugin_sound.and_then(|t| library(&t.id));
        let preset = pl.preset_key.as_deref().zip(pl.preset.as_deref());
        // A preset's own library record (added when it was picked) is that preset.
        let is_preset = |key: &str, p: &Patch| PluginOrigin::from_preset_key(key).is_some_and(|o| p.source.same_plugin_origin(&pl.id, &o));
        // (the Sound, a name of the user's own)
        let (sound, own) = match (preset, saved) {
            (Some((key, name)), s) if s.is_none_or(|p| is_preset(key, p)) => (SoundTag { id: super::plugin_preset_id(&pl.id, key), name: name.to_string() }, false),
            (_, Some(p)) => (p.tag(), matches!(&p.source, PatchSource::Plugin { origin, .. } if origin.is_user())),
            _ => (SoundTag { id: format!("au:{}", pl.id), name: pl.name.clone() }, true),
        };
        let voice_name = if own { sound.name.clone() } else { format!("{} · {}", pl.name, sound.name) };
        return PartSound { sound: Some(sound), voice_name };
    }
    let font = |p: &&Patch| matches!(p.source, PatchSource::SoundFont { .. });
    if let Some(p) = part.own.and_then(|id| lib.patch(id)).filter(font) {
        return PartSound { sound: Some(p.tag()), voice_name: p.name.clone() };
    }
    let gm = || PartSound { sound: None, voice_name: super::gm_name(part.program).to_string() };
    let Some(id) = crate::patches::resolve_gm(lib, style, auto, false, part.program).sound else { return gm() };
    match SoundId::parse(&id) {
        Some(SoundId::Font(f)) => {
            let name = font_name(&f).unwrap_or_else(|| super::gm_name(part.program).to_string());
            PartSound { sound: Some(SoundTag { id, name: name.clone() }), voice_name: name }
        }
        // A plugin sound the map gives the voice, with no plugin playing it: nothing names
        // the SoundFont voice that plays.
        _ => library(&id).filter(font).map_or_else(gm, |p| PartSound { sound: Some(p.tag()), voice_name: p.name.clone() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{PartPlugin, PluginStatus};
    use crate::patches::{AutoFill, Category, FontPreset, Patch, PatchDefaults, PatchSource, PluginOrigin, SoundLibrary, SoundTag};

    const SMP: &str = "aumu Smp7 Fake";

    fn patch(id: &str, name: &str, source: PatchSource) -> Patch {
        Patch { id: id.into(), name: name.into(), category: Category::Piano, tags: vec![], favourite: false, source, defaults: PatchDefaults::default() }
    }

    fn plugin_sound(id: &str, name: &str, origin: PluginOrigin) -> Patch {
        patch(id, name, PatchSource::Plugin { component_id: SMP.into(), state: String::new(), origin })
    }

    fn lib() -> SoundLibrary {
        let mut lib = SoundLibrary::default();
        lib.patches = vec![
            patch("grand", "Stage Grand", PatchSource::SoundFont { file: "A.sf2".into(), bank: 0, program: 0 }),
            plugin_sound("warm", "Warm Keys", PluginOrigin::Factory { number: 3 }),
            plugin_sound("mine", "My Keys", PluginOrigin::User),
        ];
        lib.map.set_override(1, Some("grand".into()));
        lib.map.set_override(2, Some("mine".into()));
        lib
    }

    /// The auto-fill has programs 0-9 on A.sf2, named "A<n>".
    fn auto() -> AutoFill {
        AutoFill { programs: (0..128u8).map(|p| (p < 10).then(|| FontPreset::new("A.sf2", 0, p))).collect(), drums: None }
    }

    /// The cases the web's `partSound` test reads too, so the twins can't drift apart.
    const CASES: &str = include_str!("../../tests/fixtures/part_sound_cases.json");

    #[test]
    fn part_sound_matches_the_shared_cases() {
        use serde_json::Value;
        let lib = lib();
        let cases: Value = serde_json::from_str(CASES).unwrap();
        let s = |v: &Value| v.as_str().map(str::to_string);
        for case in cases["cases"].as_array().unwrap() {
            let of = &case["of"];
            let plugin = of.get("plugin").filter(|p| !p.is_null()).map(|p| PartPlugin {
                id: SMP.into(),
                name: "Sampler Deluxe".into(),
                status: serde_json::from_value::<PluginStatus>(p["status"].clone()).unwrap(),
                preset: s(&p["preset"][1]),
                preset_key: s(&p["preset"][0]),
                ..PartPlugin::default()
            });
            let plugin_sound = of.get("pluginSound").filter(|t| !t.is_null()).map(|t| SoundTag { id: s(&t["id"]).unwrap(), name: s(&t["name"]).unwrap() });
            let own = s(&of["own"]);
            let program = of["program"].as_u64().unwrap() as u8;
            let part = PartSoundOf { plugin: plugin.as_ref(), plugin_sound: plugin_sound.as_ref(), own: own.as_deref(), program };
            let r = part_sound(&lib, None, &auto(), part, &|f| Some(format!("A{}", f.program)));
            let want = &case["want"];
            let voice = s(&want[2]).unwrap_or_else(|| crate::api::gm_name(program).to_string());
            let got = (r.sound.as_ref().map(|t| t.id.clone()), r.sound.map(|t| t.name), r.voice_name);
            assert_eq!(got, (s(&want[0]), s(&want[1]), voice), "{}", case["name"]);
        }
    }
}
