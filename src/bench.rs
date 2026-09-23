//! `yahaha bench <style>`: end-to-end latency through real CoreMIDI.
//!
//! A virtual source ("yahaha-bench-in") plays the role of the keyboard; an input port
//! listens to yahaha's own virtual output. Times are host clock, taken just before our
//! send call and inside the receive callback, so they include CoreMIDI's routing.

use crate::engine::{Engine, Prepared};
use crate::live::{self, Cmd, Input, Shared, TAG_KEYS};
use crate::midi::{Client, InputHandler};
use crate::rt::{self, Histogram, PacketSink, Target};
use crate::sff::Style;
use crate::theory::Recognizer;
use anyhow::Result;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::Duration;

struct Rx {
    tx: Producer<(u64, [u8; 3])>,
}

impl InputHandler for Rx {
    fn packet(&mut self, _tag: usize, _ts: u64, data: &[u8]) {
        let now = rt::now_ns();
        let mut rs = 0;
        crate::midi::for_each_message(data, &mut rs, |m| {
            let mut a = [0u8; 3];
            a[..m.len()].copy_from_slice(m);
            let _ = self.tx.push((now, a));
        });
    }
}

fn wait_for(rx: &mut Consumer<(u64, [u8; 3])>, after: u64, timeout: Duration, f: impl Fn(&[u8; 3]) -> bool) -> Option<u64> {
    let end = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < end {
        while let Ok((t, m)) = rx.pop() {
            if t >= after && f(&m) {
                return Some(t);
            }
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    None
}

fn drain(rx: &mut Consumer<(u64, [u8; 3])>) {
    while rx.pop().is_ok() {}
}

fn report(name: &str, v: &mut [u64]) {
    if v.is_empty() {
        println!("  {name:<34} no samples");
        return;
    }
    v.sort();
    let p = |q: f64| v[((v.len() as f64 - 1.0) * q).round() as usize] as f64 / 1000.0;
    println!("  {name:<34} n={:<4} p50 {:>7.1} µs   p99 {:>7.1} µs   max {:>7.1} µs", v.len(), p(0.5), p(0.99), p(1.0));
}

fn report_hist(name: &str, h: &Histogram) {
    let n = h.count();
    println!(
        "  {name:<34} n={:<4} p50 <{:>5} µs   p99 <{:>5} µs   max {:>7.1} µs",
        n,
        h.percentile_us(0.5),
        h.percentile_us(0.99),
        h.max_ns.load(Relaxed) as f64 / 1000.0
    );
}

pub fn run(path: &std::path::Path, spin_us: Option<u64>) -> Result<()> {
    let style = Style::load(path)?;
    let prep = Box::new(Prepared::new(&style));
    let first_tick = prep.sections[4].as_ref().and_then(|s| s.first_note_tick()).unwrap_or(0);
    let offset_ns = (first_tick as f64 / prep.ppq as f64 * 60e9 / prep.bpm) as u64;
    println!("style: {} ({:.0} bpm), Main A first note at tick {}", prep.name, prep.bpm, first_tick);

    let client = Client::new("yahaha-bench")?;
    let out_src = client.virtual_source("yahaha (bench)")?;
    let kbd_src = client.virtual_source("yahaha-bench-in")?;
    let shared = Arc::new(Shared::new(54));
    if let Some(us) = spin_us {
        shared.spin_ns.store(us * 1000, Relaxed);
    }
    let mut ch = live::channels(live::Out::new(PacketSink::new(Target::Virtual(out_src)), None));
    let input = Input::new(shared.clone(), Recognizer::new(), ch.input_tx, live::Out::new(PacketSink::new(Target::Virtual(out_src)), None));
    let port = client.input_port("in", input)?;
    port.connect(kbd_src, TAG_KEYS)?;

    let (rx_tx, mut rx) = RingBuffer::new(1 << 16);
    let rx_port = client.input_port("bench-rx", Rx { tx: rx_tx })?;
    rx_port.connect(out_src, 0)?;

    let engine = Engine::new(prep);
    let sh = shared.clone();
    let io = ch.io;
    let th = std::thread::Builder::new().name("yahaha-engine".into()).spawn(move || live::run_engine(engine, io, sh))?;
    std::thread::sleep(Duration::from_millis(200));
    println!("engine thread real-time policy: {}", if shared.engine_rt.load(Relaxed) { "yes" } else { "NO" });
    println!("engine spin window: {} µs\n", shared.spin_ns.load(Relaxed) / 1000);

    let mut kbd = PacketSink::new(Target::Virtual(kbd_src));
    let mut send = |msgs: &[&[u8]]| -> u64 {
        for m in msgs {
            kbd.push(m);
        }
        let t = rt::now_ns();
        kbd.flush();
        t
    };

    // 1. Right-hand passthrough.
    let mut pass = Vec::new();
    drain(&mut rx);
    for _ in 0..300 {
        let t0 = send(&[&[0x90, 72, 100]]);
        if let Some(t) = wait_for(&mut rx, t0, Duration::from_millis(100), |m| m[0] == 0x90 && m[1] == 72) {
            pass.push(t - t0);
        }
        send(&[&[0x80, 72, 0]]);
        std::thread::sleep(Duration::from_millis(5));
    }

    // 2. Chord -> first accompaniment note (sync start).
    let mut sync = Vec::new();
    let chords: [[u8; 3]; 2] = [[48, 52, 55], [43, 47, 50]];
    for i in 0..120 {
        let _ = ch.ui_tx.push(Cmd::Arm);
        shared.wake.signal();
        std::thread::sleep(Duration::from_millis(30));
        drain(&mut rx);
        let c = chords[i % 2];
        let t0 = send(&[&[0x90, c[0], 90], &[0x90, c[1], 90], &[0x90, c[2], 90]]);
        if let Some(t) =
            wait_for(&mut rx, t0, Duration::from_millis(200), |m| m[0] & 0xF0 == 0x90 && (8..16).contains(&(m[0] & 0xF)))
        {
            sync.push((t - t0).saturating_sub(offset_ns));
        }
        send(&[&[0x80, c[0], 0], &[0x80, c[1], 0], &[0x80, c[2], 0]]);
    }

    // 3. Keep playing with chord changes; the engine records its own timing.
    let _ = ch.ui_tx.push(Cmd::Arm);
    shared.wake.signal();
    std::thread::sleep(Duration::from_millis(30));
    for h in [&shared.lateness, &shared.chord_lat] {
        for b in &h.buckets {
            b.store(0, Relaxed);
        }
        h.max_ns.store(0, Relaxed);
    }
    let prog: [[u8; 3]; 4] = [[48, 52, 55], [45, 48, 52], [41, 45, 48], [43, 47, 50]];
    let mut n_out = 0usize;
    for i in 0..24 {
        let c = prog[i % 4];
        send(&[&[0x90, c[0], 90], &[0x90, c[1], 90], &[0x90, c[2], 90]]);
        let end = std::time::Instant::now() + Duration::from_millis(480);
        while std::time::Instant::now() < end {
            while rx.pop().is_ok() {
                n_out += 1;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        send(&[&[0x80, c[0], 0], &[0x80, c[1], 0], &[0x80, c[2], 0]]);
    }

    println!("results (host clock, through CoreMIDI):");
    report("keyboard -> passthrough out", &mut pass);
    report("chord -> first band note (sync)*", &mut sync);
    report_hist("chord published -> engine applied", &shared.chord_lat);
    report_hist("engine wake vs. event deadline", &shared.lateness);
    println!("  messages received during 11.5 s play: {n_out}");
    println!("  * minus the style's own {:.1} ms offset before its first note", offset_ns as f64 / 1e6);
    report_hist("engine work per wake (process)", &shared.work_lat);
    report_hist("engine CoreMIDI send per wake", &shared.flush_lat);

    shared.quit.store(true, Relaxed);
    shared.wake.signal();
    let _ = th.join();
    drop((port, rx_port));
    Ok(())
}

/// `yahaha drive`: pretend to be a keyboard ("TestKbd" virtual source) against a running
/// `yahaha play --all-inputs`, and report what comes out of the "yahaha" port.
pub fn drive() -> Result<()> {
    let client = Client::new("yahaha-driver")?;
    let kbd_src = client.virtual_source("TestKbd")?;
    let (rx_tx, mut rx) = RingBuffer::new(1 << 16);
    let rx_port = client.input_port("driver-rx", Rx { tx: rx_tx })?;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let out = loop {
        if let Some((e, _)) = crate::midi::sources().into_iter().find(|(_, n)| n == "yahaha") {
            break e;
        }
        anyhow::ensure!(std::time::Instant::now() < deadline, "no \"yahaha\" source; is `yahaha play` running?");
        // CoreMIDI only learns about endpoints created by other processes via the run loop.
        unsafe {
            core_foundation::runloop::CFRunLoopRunInMode(core_foundation::runloop::kCFRunLoopDefaultMode, 0.1, 0);
        }
    };
    rx_port.connect(out, 0)?;
    // Give `play` a moment to see TestKbd if it was started after us.
    std::thread::sleep(Duration::from_millis(300));
    let mut kbd = PacketSink::new(Target::Virtual(kbd_src));
    let mut send = |msgs: &[[u8; 3]]| -> u64 {
        for m in msgs {
            kbd.push(m);
        }
        let t = rt::now_ns();
        kbd.flush();
        t
    };
    let mut log: Vec<(u64, [u8; 3])> = Vec::new();
    let mut pump = |log: &mut Vec<(u64, [u8; 3])>, ms: u64| {
        let end = std::time::Instant::now() + Duration::from_millis(ms);
        while std::time::Instant::now() < end {
            while let Ok(x) = rx.pop() {
                log.push(x);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    };
    let prog: [(&str, [u8; 3]); 4] = [("C", [48, 52, 55]), ("Am", [45, 48, 52]), ("F", [41, 45, 48]), ("G", [43, 47, 50])];
    let mut marks = Vec::new();
    for (name, c) in prog {
        let t = send(&[[0x90, c[0], 90], [0x90, c[1], 90], [0x90, c[2], 90]]);
        marks.push((name, t));
        let rh = send(&[[0x90, 76, 100]]);
        marks.push(("RH E4", rh));
        pump(&mut log, 1500);
        send(&[[0x80, 76, 0]]);
        send(&[[0x80, c[0], 0], [0x80, c[1], 0], [0x80, c[2], 0]]);
        pump(&mut log, 20);
    }
    pump(&mut log, 300);
    let mut per_ch = [0usize; 16];
    for (_, m) in &log {
        if m[0] & 0xF0 == 0x90 && m[2] > 0 {
            per_ch[(m[0] & 0xF) as usize] += 1;
        }
    }
    println!("note-ons per channel on \"yahaha\":");
    for (i, n) in per_ch.iter().enumerate() {
        if *n > 0 {
            println!("  ch {:>2}: {n}", i + 1);
        }
    }
    for (name, t) in &marks {
        let first = log.iter().find(|(tt, m)| {
            *tt >= *t
                && m[0] & 0xF0 == 0x90
                && if name.starts_with("RH") { m[0] & 0xF == 0 && m[1] == 76 } else { m[0] & 0xF == 1 }
        });
        if let Some((tt, _)) = first {
            println!("  {name:<6} -> passthrough after {:.0} µs", (tt - t) as f64 / 1000.0);
        }
    }
    // Bass part (ch 11) pitch classes heard during each chord.
    for (k, (name, t)) in marks.iter().filter(|m| !m.0.starts_with("RH")).enumerate() {
        let end = t + 1_500_000_000;
        let mut pcs: Vec<&str> = log
            .iter()
            .filter(|(tt, m)| *tt >= *t && *tt < end && m[0] == 0x9A && m[2] > 0)
            .map(|(_, m)| crate::theory::NOTE_NAMES[m[1] as usize % 12])
            .collect();
        pcs.dedup();
        println!("  chord {} {name:<3} bass notes: {}", k + 1, pcs.join(" "));
    }
    Ok(())
}
