//! New and missing plugins (docs/racks.md, "Plugins coming and going") through offline
//! sessions on a synthetic style (no corpus). The plugin list comes from a mock scan cache
//! with made-up plugins ("Sampler Deluxe" and friends), and Apple's DLSMusicDevice where a
//! plugin must really play.

use super::super::testing::{data_dir, session_in as session};
use super::{Presence, FILE};
use crate::api::*;
use crate::patches::PatchSource;
use crate::session::Session;

/// The record on its own: the first scan only records, later ones flag newcomers, a
/// plugin opened stops being new, and all of it survives a restart. A newer yahaha's file
/// is never saved over.
#[test]
fn the_record_of_plugins_seen() {
    let d = data_dir("presence-record");
    std::fs::create_dir_all(&d).unwrap();
    let p = |id: &str, name: &str| (id.to_string(), name.to_string(), "Fake Instruments".to_string());
    let mut r = Presence::open(Some(&d));
    r.note_scan(&[p("aumu Smp7 Fake", "Sampler Deluxe")]).unwrap();
    assert!(!r.is_new("aumu Smp7 Fake"), "the first scan ever only records");
    r.note_scan(&[p("aumu Smp7 Fake", "Sampler Deluxe"), p("aumu Org1 Fake", "Organ Deluxe")]).unwrap();
    assert!(r.is_new("aumu Org1 Fake") && !r.is_new("aumu Smp7 Fake"));
    let mut r = Presence::open(Some(&d));
    assert!(r.is_new("aumu Org1 Fake"), "kept in {FILE}");
    assert!(!r.scanned, "nothing is missing before this start's scan");
    assert!(r.mark_seen("aumu Org1 Fake").unwrap());
    assert!(!r.mark_seen("aumu Org1 Fake").unwrap(), "only once");
    assert!(!Presence::open(Some(&d)).is_new("aumu Org1 Fake"));
    // Uninstalled: still known, with its name.
    let mut r = Presence::open(Some(&d));
    r.note_scan(&[p("aumu Org1 Fake", "Organ Deluxe")]).unwrap();
    assert_eq!(r.known("aumu Smp7 Fake").map(|k| k.name.as_str()), Some("Sampler Deluxe"));
    // A newer yahaha's file: its plugins count as seen, and it is left alone.
    let newer = r#"{"format":"yahaha.known-plugins","version":9,"plugins":[{"id":"aumu Smp7 Fake","name":"Sampler Deluxe"}]}"#;
    std::fs::write(d.join(FILE), newer).unwrap();
    let mut r = Presence::open(Some(&d));
    r.note_scan(&[p("aumu Smp7 Fake", "Sampler Deluxe"), p("aumu Pad1 Fake", "Pad Machine")]).unwrap();
    assert!(r.is_new("aumu Pad1 Fake") && !r.is_new("aumu Smp7 Fake"));
    assert_eq!(std::fs::read_to_string(d.join(FILE)).unwrap(), newer, "never saved over");
    let _ = std::fs::remove_dir_all(&d);
}

/// A SoundFont library sound for the Left part.
fn bass_sound(s: &Session) -> String {
    let source = PatchSource::SoundFont { file: "Other.sf2".into(), bank: 0, program: 33 };
    add_sound(s, "My Bass", source)
}

fn add_sound(s: &Session, name: &str, source: PatchSource) -> String {
    let patch = PatchFields { name: name.into(), category: Default::default(), tags: Vec::new(), favourite: false, source };
    s.send(SoundLibraryCmd::CreatePatch { patch }).unwrap();
    s.state().sound_library.patches.iter().find(|p| p.patch.name == name).unwrap().patch.id.clone()
}

/// `replacePartSound` swaps the sound and keeps the part's mix (every setting a rack part
/// holds, bend range included).
#[test]
fn replacing_a_parts_sound_keeps_its_mix() {
    let d = data_dir("presence-replace");
    let s = session(&d);
    let bass = bass_sound(&s);
    s.send(PartsCmd::SetPartVolume { part: 3, volume: 99 }).unwrap();
    s.send(PartsCmd::SetPartPan { part: 3, pan: 70 }).unwrap();
    s.send(PartsCmd::SetPartSend { part: 3, send: PartSend::Chorus, value: 44 }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 3, octave: -1 }).unwrap();
    s.send(ControllersCmd::SetBendRange { part: 3, semitones: 7 }).unwrap();
    let mix = |s: &Session| {
        let p = &s.state().keyboard_parts[3];
        (p.volume, p.pan, p.chorus, p.octave)
    };
    s.send(SoundsCmd::ReplacePartSound { part: 3, id: format!("saved:{bass}") }).unwrap();
    assert_eq!(s.state().keyboard_parts[3].patch.as_deref(), Some(bass.as_str()), "the new sound plays");
    assert_eq!(mix(&s), (99, 70, 44, -1), "with the part's own mix");
    assert_eq!(s.capture_rack("R").parts[3].bend_range, 7);
    assert!(s.send(SoundsCmd::ReplacePartSound { part: 4, id: format!("saved:{bass}") }).is_err());
    // A sound carries no mix, so assigning one keeps the part's mix too.
    s.send(SoundsCmd::AssignSound { part: 3, id: format!("saved:{bass}") }).unwrap();
    assert_eq!(mix(&s), (99, 70, 44, -1));
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(feature = "plugins")]
mod with_plugins {
    use super::*;
    use crate::plugin::{PluginFormat, PluginId, PluginInfo};
    use crate::racks::{self, Rack, SoundRef};
    use crate::route::Source;
    use std::path::Path;
    use std::time::{Duration, Instant};

    const SAMPLER: &str = "aumu Smp7 Fake";
    const PADS: &str = "aumu Pad1 Fake";
    const ORGAN: &str = "aumu Org1 Fake";
    #[cfg(feature = "slow-tests")]
    const DLS: &str = "aumu dls  appl";

    fn info(id: &str, name: &str, manufacturer: &str) -> PluginInfo {
        PluginInfo {
            id: PluginId::parse(id).unwrap(),
            name: name.into(),
            manufacturer: manufacturer.into(),
            version: 0x10000,
            format: PluginFormat::Au2,
            requires_async: false,
            can_load_in_process: false,
            sandbox_safe: true,
            last_load: None,
            in_process: false,
            factory_presets: None,
            user_presets: Vec::new(),
        }
    }

    fn sampler() -> PluginInfo {
        info(SAMPLER, "Sampler Deluxe", "Fake Instruments")
    }
    fn pads() -> PluginInfo {
        info(PADS, "Pad Machine", "Fake Instruments")
    }
    fn organ() -> PluginInfo {
        info(ORGAN, "Organ Deluxe", "Fake Instruments")
    }
    /// Apple's DLSMusicDevice (every Mac has it): the plugin that really plays.
    #[cfg(feature = "slow-tests")]
    fn dls() -> PluginInfo {
        info(DLS, "DLSMusicDevice", "Apple")
    }

    /// A scan whose mock cache lists exactly `plugins` (as after installing or removing
    /// some), pumped until it is in.
    fn scan(s: &Session, d: &Path, plugins: Vec<PluginInfo>) {
        let host = crate::plugin::mock_host(&d.join("cache/plugins.json"), vec![d.join("Presets")], plugins);
        {
            let mut ctl = s.inner.lock();
            ctl.plugins.host = Some(host);
            ctl.start_plugin_scan(false);
        }
        let t0 = Instant::now();
        while s.inner.lock().plugins.scan_rx.is_some() {
            assert!(t0.elapsed() < Duration::from_secs(60), "the plugin scan did not finish");
            s.advance(1_000_000);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn plugins(s: &Session) -> PluginsState {
        s.state().plugins.clone()
    }

    fn entry(st: &PluginsState, id: &str) -> PluginEntry {
        st.list.iter().find(|e| e.id == id).cloned().unwrap_or_else(|| panic!("{id} is not listed"))
    }

    fn missing(id: &str, name: &str, manufacturer: &str, racks: u32, sounds: u32) -> MissingPlugin {
        MissingPlugin { id: id.into(), name: name.into(), manufacturer: manufacturer.into(), racks, sounds }
    }

    #[test]
    fn new_and_missing_plugins_across_scans() {
        let d = data_dir("presence-scans");
        let s = session(&d);
        scan(&s, &d, vec![sampler(), pads()]);
        let st = plugins(&s);
        assert_eq!(st.list.len(), 2);
        assert!(st.list.iter().all(|e| !e.new), "the first scan ever only records what is there");
        assert!(st.missing.is_empty());
        // The organ installed, the pads removed.
        scan(&s, &d, vec![sampler(), organ()]);
        let st = plugins(&s);
        assert!(entry(&st, ORGAN).new && !entry(&st, SAMPLER).new);
        assert_eq!(st.missing, [missing(PADS, "Pad Machine", "Fake Instruments", 0, 0)], "named as it was installed");
        // The next start knows both.
        drop(s);
        let s = session(&d);
        assert!(plugins(&s).missing.is_empty(), "nothing is missing before the scan is in");
        scan(&s, &d, vec![sampler(), organ()]);
        let st = plugins(&s);
        assert!(entry(&st, ORGAN).new, "still new after a restart");
        assert_eq!(st.missing, [missing(PADS, "Pad Machine", "Fake Instruments", 0, 0)]);
        // Opened in the Library: no longer new.
        s.send(PluginCmd::MarkPluginSeen { id: ORGAN.into() }).unwrap();
        assert!(!entry(&plugins(&s), ORGAN).new);
        assert!(s.send(PluginCmd::MarkPluginSeen { id: PADS.into() }).is_err(), "not installed");
        // The pads reinstalled: known before, so not new, and no longer missing.
        drop(s);
        let s = session(&d);
        scan(&s, &d, vec![sampler(), organ(), pads()]);
        let st = plugins(&s);
        assert!(st.list.iter().all(|e| !e.new), "{:?}", st.list);
        assert!(st.missing.is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Save a rack called `name` of what plays now, with `sounds` on its parts.
    fn save_rack(s: &Session, d: &Path, name: &str, sounds: [Option<SoundRef>; 4]) -> Rack {
        let mut r = s.capture_rack(name);
        for (p, sound) in sounds.into_iter().enumerate() {
            if let Some(sound) = sound {
                r.parts[p].sound = sound;
            }
        }
        r.save(&racks::path_for(&racks::dir(d), name)).unwrap();
        r
    }

    #[test]
    fn racks_and_sounds_that_use_each_plugin() {
        let d = data_dir("presence-usage");
        let s = session(&d);
        scan(&s, &d, vec![sampler(), pads()]);
        let plugin_sound = |name: &str, id: &str| add_sound(&s, name, PatchSource::plugin(id, ""));
        let keys = plugin_sound("Deluxe Keys", SAMPLER);
        plugin_sound("Deluxe Choir", SAMPLER);
        let pad = plugin_sound("Warm Pad", PADS);
        let plugin = |id: &str| Some(SoundRef::Plugin { component: id.into() });
        let library = |id: &str| Some(SoundRef::Library { id: id.into() });
        save_rack(&s, &d, "One", [plugin(SAMPLER), library(&keys), None, None]);
        let two = save_rack(&s, &d, "Two", [None, None, library(&pad), None]);
        save_rack(&s, &d, "Three", [None, None, None, None]);
        // The racks are read again after a scan.
        scan(&s, &d, vec![sampler(), pads()]);
        let st = plugins(&s);
        let uses = |e: PluginEntry| (e.racks, e.sounds);
        assert_eq!(uses(entry(&st, SAMPLER)), (1, 2), "one rack (twice), two sounds");
        assert_eq!(uses(entry(&st, PADS)), (1, 1));
        assert!(st.missing.is_empty() && st.needs_attention.is_empty());
        // The pads removed: the rack that plays them needs attention.
        scan(&s, &d, vec![sampler()]);
        let st = plugins(&s);
        assert_eq!(st.missing, [missing(PADS, "Pad Machine", "Fake Instruments", 1, 1)]);
        assert_eq!(st.needs_attention, [RackAttention { id: two.id.clone(), name: "Two".into(), parts: vec![2] }]);
        // A rack saved meanwhile counts within 2 s, no scan needed; a plugin never
        // installed here is missing too, named by its id.
        let never = "aumu Zzz1 Fake";
        let four = save_rack(&s, &d, "Four", [None, plugin(never), None, plugin(PADS)]);
        s.advance(3_000_000_000);
        let st = plugins(&s);
        let attention: Vec<(&str, &[u8])> = st.needs_attention.iter().map(|r| (r.name.as_str(), r.parts.as_slice())).collect();
        assert_eq!(attention, [("Four", &[1u8, 3][..]), ("Two", &[2u8][..])]);
        assert_eq!(st.needs_attention[0].id, four.id);
        assert_eq!(st.missing, [missing(never, never, "", 1, 0), missing(PADS, "Pad Machine", "Fake Instruments", 2, 1)]);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Pump until part `part`'s plugin has finished loading.
    fn wait_loaded(s: &Session, part: usize) -> PluginStatus {
        let t0 = Instant::now();
        loop {
            s.advance(1_000_000);
            match s.state().keyboard_parts[part].plugin.as_ref().map(|p| p.status) {
                Some(PluginStatus::Loading) if t0.elapsed() < Duration::from_secs(20) => std::thread::sleep(Duration::from_millis(1)),
                Some(x) => return x,
                None => panic!("no plugin on the part"),
            }
        }
    }

    /// Part `part`'s plugin state, once read (`savePartPluginState`).
    #[cfg(feature = "slow-tests")]
    fn saved_state(s: &Session, part: usize) -> Option<Vec<u8>> {
        s.inner.lock().saved_parts().parts[part].as_ref().and_then(|v| v.state.clone())
    }

    fn route(s: &Session, part: usize) -> Source {
        s.inner.lock().synth.as_ref().unwrap().control.routes.source(crate::parts::CHANNEL[part])
    }

    #[cfg(feature = "slow-tests")]
    fn energy(l: &[f32], r: &[f32]) -> f64 {
        l.iter().chain(r).map(|x| (*x as f64).powi(2)).sum()
    }

    /// A rack whose parts play a plugin that goes away: they are silent (not on the
    /// SoundFont) and say so, their mix and the rack's reference stay, nothing is written;
    /// once the plugin is back and scanned, they play it again with the state they had.
    #[test]
    #[cfg(feature = "slow-tests")]
    fn a_part_whose_plugin_is_missing_is_silent_and_plays_again_when_it_is_back() {
        let d = data_dir("presence-silent");
        let s = session(&d);
        // A SoundFont (the tiny test font, which sounds on every GM program) makes the
        // silence audible: the parts are not on it.
        let sf2 = d.join("Tiny.sf2");
        std::fs::write(&sf2, crate::patches::sf2::tiny_gm_sound_font()).unwrap();
        s.offline_audio(Some(&sf2), 48_000).unwrap();
        s.fx_returns_off();
        scan(&s, &d, vec![dls(), sampler()]);
        // A real DLS state, read at run time (none is committed).
        s.send(PluginCmd::SetPartPlugin { part: 2, id: DLS.into(), state: None }).unwrap();
        assert_eq!(wait_loaded(&s, 2), PluginStatus::Playing);
        s.send(PluginCmd::SavePartPluginState { part: 2 }).unwrap();
        let t0 = Instant::now();
        let state = loop {
            s.advance(1_000_000);
            if let Some(st) = saved_state(&s, 2) {
                break st;
            }
            assert!(t0.elapsed() < Duration::from_secs(20), "the state was never read");
            std::thread::sleep(Duration::from_millis(1));
        };
        s.send(PluginCmd::ClearPartPlugin { part: 2 }).unwrap();
        // Right 1 plays DLS with that state; Right 2 a library sound on DLS (its default
        // settings). Each with a mix of its own.
        let sound = add_sound(&s, "DLS Keys", PatchSource::plugin(DLS, ""));
        let mut rack = s.capture_rack("Strings");
        rack.parts[0].sound = SoundRef::Plugin { component: DLS.into() };
        rack.parts[0].edited_state = Some(base64_encode(&state));
        (rack.parts[0].on, rack.parts[0].volume, rack.parts[0].pan, rack.parts[0].reverb, rack.parts[0].octave) = (true, 77, 30, 90, 1);
        rack.parts[1].sound = SoundRef::Library { id: sound.clone() };
        (rack.parts[1].on, rack.parts[1].volume, rack.parts[1].chorus) = (true, 55, 66);
        rack.parts[2].on = false;
        let path = racks::path_for(&racks::dir(&d), "Strings");
        rack.save(&path).unwrap();
        let on_disk = std::fs::read(&path).unwrap();
        let library = std::fs::read(d.join("sound-library.json")).unwrap();
        let mix = |s: &Session| {
            let p = &s.state().keyboard_parts;
            ((p[0].volume, p[0].pan, p[0].reverb, p[0].octave), (p[1].volume, p[1].chorus))
        };

        // DLS uninstalled.
        scan(&s, &d, vec![sampler()]);
        let problems = s.apply_rack(&Rack::load(&path).unwrap());
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].starts_with("Right 1: DLSMusicDevice is not installed"), "{problems:?}");
        assert!(problems[1].starts_with("Right 2: DLSMusicDevice is not installed"), "{problems:?}");
        for part in [0, 1] {
            let p = s.state().keyboard_parts[part].plugin.clone().expect("the plugin stays on the part");
            assert!(p.missing && p.status == PluginStatus::Failed, "{p:?}");
            assert_eq!((p.id.as_str(), p.name.as_str(), p.manufacturer.as_str()), (DLS, "DLSMusicDevice", "Apple"), "named as it was installed");
            assert_eq!(route(&s, part), Source::Silent, "silent, not the SoundFont");
        }
        assert_eq!(mix(&s), ((77, 30, 90, 1), (55, 66)), "the rack's mix");
        s.midi_in(crate::session::Port::Keys, &[0x90, 72, 110]);
        let (l, r) = s.render(9600);
        assert_eq!(energy(&l, &r), 0.0, "nothing plays the notes");
        s.midi_in(crate::session::Port::Keys, &[0x80, 72, 0]);
        let st = plugins(&s);
        assert_eq!(st.missing, [missing(DLS, "DLSMusicDevice", "Apple", 1, 1)]);
        assert_eq!(st.needs_attention, [RackAttention { id: rack.id.clone(), name: "Strings".into(), parts: vec![0, 1] }]);
        // The parts still hold the rack's sounds, and nothing was written.
        let again = s.capture_rack("Again");
        assert_eq!((&again.parts[0].sound, &again.parts[0].edited_state), (&rack.parts[0].sound, &rack.parts[0].edited_state));
        assert_eq!(again.parts[1].sound, rack.parts[1].sound);
        assert_eq!(saved_state(&s, 0).as_ref(), Some(&state), "the part keeps the plugin's state");
        assert_eq!(std::fs::read(&path).unwrap(), on_disk, "the rack file is untouched");
        assert_eq!(std::fs::read(d.join("sound-library.json")).unwrap(), library, "the sound library is untouched");

        // Reinstalled and scanned: both play again, as they were.
        scan(&s, &d, vec![dls(), sampler()]);
        for part in [0, 1] {
            assert_eq!(wait_loaded(&s, part), PluginStatus::Playing);
            assert!(!s.state().keyboard_parts[part].plugin.as_ref().unwrap().missing);
            assert_eq!(route(&s, part), Source::Plugin);
        }
        assert_eq!(saved_state(&s, 0).as_ref(), Some(&state), "with the state it had");
        assert_eq!(s.state().keyboard_parts[1].patch.as_deref(), Some(sound.as_str()), "and Right 2 its sound");
        assert_eq!(mix(&s), ((77, 30, 90, 1), (55, 66)));
        s.midi_in(crate::session::Port::Keys, &[0x90, 72, 110]);
        let (l, r) = s.render(9600);
        assert!(energy(&l, &r) > 1e-3, "DLS plays the notes again");
        s.midi_in(crate::session::Port::Keys, &[0x80, 72, 0]);
        let st = plugins(&s);
        assert!(st.missing.is_empty() && st.needs_attention.is_empty());
        assert!(!entry(&st, DLS).new, "known before: not new");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A plugin that is installed but fails to load keeps today's behaviour: the part
    /// plays its SoundFont voice, and isn't missing.
    #[test]
    fn a_plugin_that_fails_to_load_is_not_missing() {
        let d = data_dir("presence-fails");
        let s = session(&d);
        s.offline_audio(None, 48_000).unwrap();
        scan(&s, &d, vec![sampler()]);
        // The made-up sampler is listed but not in the registrar: its load fails.
        s.send(PluginCmd::SetPartPlugin { part: 0, id: SAMPLER.into(), state: None }).unwrap();
        assert_eq!(wait_loaded(&s, 0), PluginStatus::Failed);
        assert!(!s.state().keyboard_parts[0].plugin.as_ref().unwrap().missing);
        assert_eq!(route(&s, 0), Source::SoundFont(0));
        let _ = std::fs::remove_dir_all(&d);
    }
}
