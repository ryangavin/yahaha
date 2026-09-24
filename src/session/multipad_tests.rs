//! Multi Pads through an offline session, with synthetic banks.

use crate::api::*;
use crate::multipad::synthetic;
use crate::session::{Options, Port, Session};
use std::path::{Path, PathBuf};

const MS: u64 = 1_000_000;

/// A folder with a style (from the corpus) and the synthetic demo bank in a subfolder.
fn setup(tag: &str) -> Option<(Session, PathBuf)> {
    let style = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !style.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    let dir = std::env::temp_dir().join(format!("yahaha-mp-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Pads")).unwrap();
    std::fs::copy(&style, dir.join("SlowWalker.sty")).unwrap();
    std::fs::write(dir.join("Pads/Demo.pad"), synthetic::demo_bank()).unwrap();
    let s = Session::offline(Options { paths: vec![dir.clone()], ..Options::default() }).unwrap();
    Some((s, dir))
}

/// Note-ons in `out` on MIDI channel `ch` (1-based).
fn ons(out: &[[u8; 3]], ch: u8) -> usize {
    out.iter().filter(|m| m[0] == 0x90 | (ch - 1) && m[2] > 0).count()
}

fn lamps(s: &Session) -> Vec<PadLamp> {
    s.state().multi_pad.pads.iter().map(|p| p.lamp).collect()
}

#[test]
fn the_bank_list_load_and_a_pad_played_while_stopped() {
    let Some((s, dir)) = setup("load") else { return };
    let st = s.state();
    assert_eq!(st.multi_pad.banks.len(), 1);
    let b = &st.multi_pad.banks[0];
    assert_eq!((b.name.as_str(), b.folder.as_str()), ("Demo", "Pads"));
    assert!(st.multi_pad.bank.is_none());
    assert_eq!(lamps(&s), [PadLamp::Empty; 4]);

    s.send(MultiPadCmd::LoadMultiPad { id: b.id }).unwrap();
    let st = s.state();
    assert!(!st.multi_pad.loading);
    assert_eq!(st.multi_pad.bank.as_ref().map(|b| b.name.as_str()), Some("Demo"));
    let names: Vec<_> = st.multi_pad.pads.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Shaker Loop", "Rise Arp", "Bass Riff", "Brass Hit"]);
    let flags: Vec<_> = st.multi_pad.pads.iter().map(|p| (p.repeat, p.chord_match, p.channel)).collect();
    assert_eq!(flags, [(true, false, 5), (false, true, 6), (true, true, 7), (false, true, 8)]);
    assert_eq!(lamps(&s), [PadLamp::Ready; 4]);

    s.take_output();
    s.send(MultiPadCmd::TriggerMultiPad { pad: 0 }).unwrap();
    assert_eq!(lamps(&s)[0], PadLamp::Playing);
    s.advance(3000 * MS);
    let out = s.take_output();
    assert!(ons(&out, 5) >= 10, "the shaker loop plays on channel 5");
    // The drum kit bank goes out with it (the synth maps it to its drum bank).
    assert!(out.contains(&[0xB4, 0, 127]));
    assert_eq!(lamps(&s)[0], PadLamp::Playing, "it repeats");

    s.send(MultiPadCmd::StopMultiPad { pad: 0 }).unwrap();
    assert_eq!(lamps(&s)[0], PadLamp::Ready);
    s.advance(1000 * MS);
    assert_eq!(ons(&s.take_output(), 5), 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn while_the_band_plays_a_pad_waits_for_the_bar_and_synchro_stop_ends_it() {
    let Some((s, dir)) = setup("band") else { return };
    s.send(MultiPadCmd::LoadMultiPad { id: 0 }).unwrap();
    // A chord starts the band (Sync Start is on).
    for n in [36, 40, 43] {
        s.midi_in(Port::Keys, &[0x90, n, 100]);
    }
    assert!(s.state().transport.running);
    s.advance(300 * MS);
    s.send(MultiPadCmd::TriggerMultiPad { pad: 2 }).unwrap();
    assert_eq!(lamps(&s)[2], PadLamp::Queued);
    let bar = 60_000.0 / s.state().transport.tempo * 4.0;
    s.advance(bar as u64 * MS);
    assert_eq!(lamps(&s)[2], PadLamp::Playing);
    s.send(TransportCmd::StartStop).unwrap();
    assert_eq!(lamps(&s)[2], PadLamp::Ready, "Synchro Stop (Style Stop) is on by default");

    s.send(MultiPadCmd::SetMultiPadSynchroStop { style_stop: false, ending: true }).unwrap();
    let st = s.state();
    assert_eq!((st.multi_pad.synchro_stop.style_stop, st.multi_pad.synchro_stop.ending), (false, true));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn synchro_start_arms_and_a_chord_fires() {
    let Some((s, dir)) = setup("sync") else { return };
    s.send(MultiPadCmd::LoadMultiPad { id: 0 }).unwrap();
    s.send(TransportCmd::ToggleSyncStart).unwrap(); // off: the chord only fires the pads
    s.send(MultiPadCmd::ArmMultiPad { pad: 1 }).unwrap();
    s.send(MultiPadCmd::ArmMultiPad { pad: 3 }).unwrap();
    assert_eq!(lamps(&s), [PadLamp::Ready, PadLamp::Armed, PadLamp::Ready, PadLamp::Armed]);
    for n in [41, 45, 48] {
        s.midi_in(Port::Keys, &[0x90, n, 100]);
    }
    assert!(!s.state().transport.running);
    assert_eq!(lamps(&s), [PadLamp::Ready, PadLamp::Playing, PadLamp::Ready, PadLamp::Playing]);
    s.send(MultiPadCmd::StopAllMultiPads).unwrap();
    assert_eq!(lamps(&s), [PadLamp::Ready; 4]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_bank_by_path_overrides_clear_and_errors() {
    let Some((s, dir)) = setup("path") else { return };
    let other = dir.join("Other.PAD");
    let mut p = synthetic::demo_phrases();
    p.truncate(1);
    std::fs::write(&other, synthetic::bank_bytes("One", &p)).unwrap();
    s.send(MultiPadCmd::LoadMultiPadPath { path: other.display().to_string() }).unwrap();
    let st = s.state();
    assert_eq!(st.multi_pad.bank.as_ref().map(|b| b.name.as_str()), Some("Other"));
    assert_eq!(st.multi_pad.banks.len(), 2);
    assert_eq!(lamps(&s), [PadLamp::Ready, PadLamp::Empty, PadLamp::Empty, PadLamp::Empty]);

    s.send(MultiPadCmd::SetMultiPadRepeat { pad: 0, on: false }).unwrap();
    s.send(MultiPadCmd::SetMultiPadChordMatch { pad: 0, on: true }).unwrap();
    let p0 = &s.state().multi_pad.pads[0];
    assert_eq!((p0.repeat, p0.chord_match), (false, true));

    assert!(s.send(MultiPadCmd::TriggerMultiPad { pad: 4 }).is_err());
    assert!(s.state().message.as_ref().is_some_and(|m| m.error));
    let bad = dir.join("bad.pad");
    std::fs::write(&bad, b"not a bank").unwrap();
    assert!(s.send(MultiPadCmd::LoadMultiPadPath { path: bad.display().to_string() }).is_err());
    assert_eq!(s.state().multi_pad.bank.as_ref().map(|b| b.name.as_str()), Some("Other"), "a bad file keeps the bank");
    // Files that do not load (missing, or not a bank) stay out of the bank list.
    assert!(s.send(MultiPadCmd::LoadMultiPadPath { path: dir.join("Nope.pad").display().to_string() }).is_err());
    let listed: Vec<String> = s.state().multi_pad.banks.iter().map(|b| b.name.clone()).collect();
    assert_eq!(listed.len(), 2, "{listed:?}");
    assert!(!listed.iter().any(|n| n == "bad" || n == "Nope"), "{listed:?}");
    assert!(s.send(MultiPadCmd::LoadMultiPad { id: 999 }).is_err());

    s.send(MultiPadCmd::ClearMultiPad).unwrap();
    let st = s.state();
    assert!(st.multi_pad.bank.is_none());
    assert_eq!(lamps(&s), [PadLamp::Empty; 4]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_rescan_finds_new_banks_and_keeps_ids() {
    let Some((s, dir)) = setup("rescan") else { return };
    let id = s.state().multi_pad.banks[0].id;
    std::fs::write(dir.join("Added.pad"), synthetic::demo_bank()).unwrap();
    s.send(LibraryCmd::RescanLibrary).unwrap();
    for _ in 0..200 {
        s.advance(10 * MS);
        if s.state().multi_pad.banks.len() == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let st = s.state();
    assert_eq!(st.multi_pad.banks.len(), 2);
    assert_eq!(st.multi_pad.banks.iter().find(|b| b.name == "Demo").map(|b| b.id), Some(id));
    let _ = std::fs::remove_dir_all(dir);
}

/// Rescan, and wait until the bank list names `marker` (a file just added under the root).
fn rescan_until(s: &Session, marker: &str) -> Vec<(usize, String)> {
    s.send(LibraryCmd::RescanLibrary).unwrap();
    for _ in 0..400 {
        s.advance(10 * MS);
        if s.state().multi_pad.banks.iter().any(|b| b.name == marker) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    s.state().multi_pad.banks.iter().map(|b| (b.id, b.name.clone())).collect()
}

#[test]
fn a_rescan_keeps_banks_loaded_by_path_outside_the_roots() {
    let Some((s, dir)) = setup("outside") else { return };
    let out = std::env::temp_dir().join(format!("yahaha-mp-outside-x-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("Elsewhere.pad"), synthetic::demo_bank()).unwrap();
    std::fs::write(out.join("Spare.pad"), synthetic::demo_bank()).unwrap();
    // Spare is loaded by path, then Elsewhere: Elsewhere is the bank loaded.
    for n in ["Spare", "Elsewhere"] {
        s.send(MultiPadCmd::LoadMultiPadPath { path: out.join(format!("{n}.pad")).display().to_string() }).unwrap();
    }
    let before: Vec<(usize, String)> = s.state().multi_pad.banks.iter().map(|b| (b.id, b.name.clone())).collect();
    let names = |v: &[(usize, String)]| v.iter().map(|b| b.1.clone()).collect::<Vec<_>>();
    assert_eq!(names(&before), ["Demo", "Spare", "Elsewhere"]);
    let elsewhere = before[2].0;

    std::fs::write(dir.join("Pads/Added.pad"), synthetic::demo_bank()).unwrap();
    let after = rescan_until(&s, "Added");
    assert_eq!(names(&after), ["Added", "Demo", "Spare", "Elsewhere"], "by-path banks stay listed");
    for b in &before {
        assert!(after.contains(b), "{b:?} keeps its id: {after:?}");
    }
    assert_eq!(s.state().multi_pad.bank.as_ref().map(|b| b.id), Some(elsewhere));
    // Loading it again by path finds it: same id, no second entry.
    s.send(MultiPadCmd::LoadMultiPadPath { path: out.join("Elsewhere.pad").display().to_string() }).unwrap();
    assert_eq!(s.state().multi_pad.banks.len(), 4);
    assert_eq!(s.state().multi_pad.bank.as_ref().map(|b| b.id), Some(elsewhere));

    // A by-path bank whose file is gone drops out; the loaded one stays even without its file.
    std::fs::remove_file(out.join("Spare.pad")).unwrap();
    std::fs::remove_file(out.join("Elsewhere.pad")).unwrap();
    std::fs::write(dir.join("Pads/Later.pad"), synthetic::demo_bank()).unwrap();
    let last = rescan_until(&s, "Later");
    assert_eq!(names(&last), ["Added", "Demo", "Later", "Elsewhere"], "{last:?}");
    assert!(last.contains(&(elsewhere, "Elsewhere".into())));
    assert_eq!(s.state().multi_pad.bank.as_ref().map(|b| b.id), Some(elsewhere));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}
