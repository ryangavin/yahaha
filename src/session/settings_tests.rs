//! `settings.json` as a player's data folder holds it (docs/eyes-free.md): a file written
//! by hand, or by an older build, starts the session with its pad page order and its Setup
//! switches, lit on the Setup page and walked by Pad Bank.

use crate::api::{PadsCmd, StopAcmpMode};
use crate::fingering::Fingering;
use crate::launchkey::{Level, Page, PAD_DOWN_CC, PAD_UP_CC};
use crate::session::testing;
use crate::session::{Options, Port, Session};
use std::path::Path;

const MS: u64 = 1_000_000;

fn start(data: &Path) -> Session {
    let s = Session::offline(Options { paths: vec![testing::style_path()], data_dir: Some(data.to_path_buf()), ..Options::default() }).unwrap();
    // The engine applies a restored Stop ACMP mode on its next wake.
    s.advance(10 * MS);
    s
}

fn write(data: &Path, json: &str) {
    std::fs::create_dir_all(data).unwrap();
    std::fs::write(data.join("settings.json"), json).unwrap();
}

/// Every switch and the order come from the file, and the Setup pads light as they are.
#[test]
fn a_settings_file_starts_the_session_with_its_switches_and_order() {
    let data = testing::data_dir("settings-file");
    write(
        &data,
        r#"{
  "padPages": ["setup", "multiPads", "racks"],
  "fingering": "aiFingered",
  "upper": true,
  "otsLink": true,
  "stopAcmpMode": "style"
}"#,
    );
    let s = start(&data);
    let st = s.state();
    assert_eq!(st.chord.fingering, Fingering::AiFingered);
    assert!(st.chord.upper, "Upper");
    assert!(st.ots.link, "OTS Link");
    assert_eq!(st.transport.stop_acmp_mode, StopAcmpMode::Style);
    assert_eq!(st.settings.pad_pages, [Page::Setup, Page::MultiPads, Page::Racks]);
    let names: Vec<_> = st.pads.pages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Sections", "Setup", "Multi Pads", "Racks"], "Chord trimmed");

    // Pad Bank ▼ walks the saved order from Sections; ▲ walks it back.
    s.midi_in(Port::Pads, &[0xB0, PAD_DOWN_CC, 127]);
    let st = s.state();
    assert_eq!((st.pads.page, st.pads.page_number, st.pads.page_count), (Page::Setup, 2, 4));
    // The Setup page lights the restored switches: AI Fingered (the fifth type), Upper,
    // OTS Link and ACMP STYLE bright; the other types and ACMP FIXED dim.
    let lit: Vec<(&str, Level)> = st.pads.pads.iter().map(|p| (p.label.as_str(), p.level)).collect();
    let bright: Vec<&str> = lit.iter().filter(|(_, l)| *l == Level::Bright).map(|(n, _)| *n).collect();
    assert_eq!(bright, ["AI FING", "UPPER", "OTS LINK", "ACMP STYLE"], "{lit:?}");
    assert_eq!(lit[10], ("ACMP FIXED", Level::Dim));
    s.midi_in(Port::Pads, &[0xB0, PAD_DOWN_CC, 127]);
    assert_eq!(s.state().pads.page, Page::MultiPads);
    s.midi_in(Port::Pads, &[0xB0, PAD_DOWN_CC, 127]);
    assert_eq!(s.state().pads.page, Page::Racks);
    s.midi_in(Port::Pads, &[0xB0, PAD_UP_CC, 127]);
    assert_eq!(s.state().pads.page, Page::MultiPads);
    // Tab (cyclePadPage) walks it too, wrapping past the end to Sections.
    s.send(PadsCmd::CyclePadPage { delta: 1 }).unwrap();
    s.send(PadsCmd::CyclePadPage { delta: 1 }).unwrap();
    assert_eq!(s.state().pads.page, Page::Sections);
    assert!(s.send(PadsCmd::SetPadPage { page: Page::Chord }).is_err(), "Chord was trimmed");

    // Nothing changed, so the file is left as it was written.
    drop(s);
    let text = std::fs::read_to_string(data.join("settings.json")).unwrap();
    assert!(text.contains("\"aiFingered\"") && text.contains("\"multiPads\""), "{text}");
    let _ = std::fs::remove_dir_all(&data);
}

/// Each switch round-trips on its own: set it, restart, and it is back; the others keep
/// their defaults.
#[test]
fn each_setup_switch_round_trips_through_a_restart() {
    use crate::api::{ChordCmd, OtsCmd};
    type Check = fn(&crate::api::AppState) -> bool;
    let cases: [(&str, crate::api::AppCmd, Check); 4] = [
        ("fingering", ChordCmd::SetFingering { fingering: Fingering::FullKeyboard }.into(), |st| st.chord.fingering == Fingering::FullKeyboard),
        ("upper", ChordCmd::SetUpper { on: true }.into(), |st| st.chord.upper),
        ("ots-link", OtsCmd::SetOtsLink { on: true }.into(), |st| st.ots.link),
        (
            "stop-acmp",
            crate::api::AppCmd::Transport(crate::api::TransportCmd::SetStopAcmp { mode: StopAcmpMode::Fixed }),
            |st| st.transport.stop_acmp_mode == StopAcmpMode::Fixed,
        ),
    ];
    for (name, cmd, check) in cases {
        let data = testing::data_dir(&format!("setup-switch-{name}"));
        let s = start(&data);
        assert!(!check(&s.state()), "{name}: not the default");
        s.send(cmd).unwrap();
        s.advance(10 * MS);
        assert!(check(&s.state()), "{name}: set");
        drop(s);
        let s = start(&data);
        let st = s.state();
        assert!(check(&st), "{name}: restored");
        let others = (st.chord.fingering, st.chord.upper, st.ots.link, st.transport.stop_acmp_mode);
        let defaults = (Fingering::FingeredOnBass, false, false, StopAcmpMode::Off);
        let changed = [others.0 != defaults.0, others.1 != defaults.1, others.2 != defaults.2, others.3 != defaults.3];
        assert_eq!(changed.iter().filter(|c| **c).count(), 1, "{name}: only it changed: {others:?}");
        assert_eq!(st.settings.pad_pages, [Page::Racks, Page::Chord, Page::MultiPads, Page::Setup], "{name}: the default order");
        drop(s);
        let _ = std::fs::remove_dir_all(&data);
    }
}

/// The Chord and Setup pages as the app and the Launchkey see them: the pads the design
/// note gives each page, where the contract put them, and no dead pad (a labelled pad
/// always does something; a dark one does nothing).
#[test]
fn chord_and_setup_pages_have_their_switches_and_no_dead_pads() {
    let s = testing::session();
    let labels = |page| {
        s.send(PadsCmd::SetPadPage { page }).unwrap();
        let st = s.state();
        for p in &st.pads.pads {
            assert_eq!(p.label.is_empty(), p.action.is_none(), "{page:?} pad {}: {:?}", p.note, p.label);
            if p.action.is_none() {
                assert_eq!(p.level, Level::Off, "{page:?} pad {}: a pad that does nothing is dark", p.note);
            }
        }
        st.pads.pads.iter().map(|p| p.label.clone()).collect::<Vec<_>>()
    };
    let chord = labels(Page::Chord);
    assert!(chord[..8].iter().all(String::is_empty), "the top row is dark: {chord:?}");
    assert_eq!(chord[8..], ["MAN BASS", "STOP ACMP", "SPLIT -", "SPLIT +", "KBD TR -", "KBD TR +", "TR RESET", "RETRIG"]);
    assert_eq!(s.state().pads.pads[8].level, Level::Off, "Manual Bass is unavailable in Lower");
    s.send(crate::api::ChordCmd::SetUpper { on: true }).unwrap();
    labels(Page::Chord);
    assert_ne!(s.state().pads.pads[8].level, Level::Off, "and available in Upper");
    let setup = labels(Page::Setup);
    assert_eq!(setup[..8], ["SINGLE", "FINGERED", "ON BASS", "MULTI", "AI FING", "FULL KBD", "AI FULL", "UPPER"]);
    assert_eq!(setup[8..], ["OTS LINK", "ACMP STYLE", "ACMP FIXED", "", "", "", "", ""]);
}

/// A file with only some settings (an older build's), or an order the session refuses,
/// keeps the defaults for the rest; an unreadable file is ignored, not fatal.
#[test]
fn a_partial_or_bad_settings_file_keeps_the_defaults() {
    let data = testing::data_dir("settings-partial");
    write(&data, r#"{ "upper": true, "padPages": ["racks", "racks"] }"#);
    let s = start(&data);
    let st = s.state();
    assert!(st.chord.upper);
    assert_eq!(st.chord.fingering, Fingering::FingeredOnBass);
    assert_eq!(st.settings.pad_pages, [Page::Racks, Page::Chord, Page::MultiPads, Page::Setup], "a duplicate order is refused");
    drop(s);

    write(&data, "not json");
    let s = start(&data);
    let st = s.state();
    assert!(!st.chord.upper);
    assert_eq!(st.settings.pad_pages.len(), 4);
    drop(s);
    let _ = std::fs::remove_dir_all(&data);
}
