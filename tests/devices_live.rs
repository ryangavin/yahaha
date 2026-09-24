//! MIDI devices coming and going while a live session runs (#74), with a device from
//! another process (`yahaha fake-device`): its keyboard is connected when it appears,
//! its Launchkey DAW port becomes the pads and is put into DAW mode, both go when it goes,
//! and a replug puts it back into DAW mode.
//!
//! It starts a live session (CoreMIDI, a virtual "yahaha" port; no synth), so it is not
//! run by default: `cargo test --release --test devices_live -- --ignored`. With a real
//! Launchkey connected, the session leaves the Launchkeys' DAW ports alone (`no_pads`: it
//! would take the real one) and only the keyboard is checked.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use yahaha::{Options, Session};

const NAME: &str = "Launchkey Fake4";

/// The fake device, and the lines it prints (what reaches its DAW In).
fn plug() -> (Child, mpsc::Receiver<String>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_yahaha"))
        .args(["fake-device", NAME, "60"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("yahaha fake-device");
    let out = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });
    assert_eq!(rx.recv_timeout(Duration::from_secs(5)).as_deref(), Ok("ready"));
    (child, rx)
}

fn unplug(mut child: Child) {
    drop(child.stdin.take()); // it removes its endpoints and exits
    let _ = child.wait();
}

fn wait_for(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(secs);
    while !f() {
        assert!(Instant::now() < end, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// ENTER_DAW (9F 0C 7F) reaches the device within `secs`.
fn enters_daw(rx: &mpsc::Receiver<String>, secs: u64) -> bool {
    let end = Instant::now() + Duration::from_secs(secs);
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(l) if l == "rx 9F 0C 7F" => return true,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    false
}

#[test]
#[ignore = "live CoreMIDI; run with --ignored"]
fn a_device_from_another_process_comes_goes_and_comes_back() {
    let style = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !style.exists() {
        eprintln!("corpus missing; skipping");
        return;
    }
    // A CoreMIDI call before the session's own, on this thread (which runs no run loop):
    // the session must still hear of devices coming and going (`midi::init`).
    let real = yahaha::midi::online_sources().iter().any(|(_, n)| n.contains("Launchkey") && n.contains("DAW"));
    if real {
        eprintln!("a Launchkey is connected: checking the keyboard only");
    }
    let pads = !real;
    let opts = Options { paths: vec![style], inputs: vec![NAME.into()], no_pads: !pads, ..Options::default() };
    // A session stopped first: the next one still hears of changes.
    Session::start(opts.clone()).unwrap().stop();
    let s = Session::start(opts).unwrap();
    let keys = format!("{NAME} MIDI Out");
    let has_keys = |s: &Session| s.state().io.inputs.iter().any(|i| *i == keys);
    assert!(!s.state().pads.connected && !has_keys(&s));

    for round in 0..2 {
        let (child, rx) = plug();
        let t = Instant::now();
        wait_for("the pads and the keys", 5, || s.state().pads.connected == pads && has_keys(&s));
        eprintln!("round {round}: connected after {} ms", t.elapsed().as_millis());
        if pads {
            assert!(enters_daw(&rx, 2), "round {round}: the Launchkey went into DAW mode");
            assert!(s.state().io.inputs.iter().any(|i| i.starts_with(NAME) && i.ends_with("(pads)")), "{:?}", s.state().io.inputs);
        }
        unplug(child);
        let t = Instant::now();
        wait_for("the device to go", 5, || !s.state().pads.connected && !has_keys(&s));
        eprintln!("round {round}: gone after {} ms", t.elapsed().as_millis());
    }
    s.stop();
}
