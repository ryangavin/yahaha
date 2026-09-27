//! Drawing the performance view: a header with the totals (audio callback, dropouts,
//! voices, stages, engine, MIDI), then a row per part (the 16 MIDI channels: the keyboard
//! parts, the Multi Pads, the Style parts) and per effect block. ANSI escapes only.

use super::{Snapshot, CHANNELS, RINGS, STAGES};
use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

/// How often the view refreshes.
pub const REFRESH: Duration = Duration::from_secs(1);
/// Without a terminal (a log file), a frame this often instead.
pub const LOG_REFRESH: Duration = Duration::from_secs(10);

/// Each MIDI channel's part.
pub const PART_NAMES: [&str; CHANNELS] = [
    "Right 1", "Left", "Right 2", "Right 3", "Multi Pad 1", "Multi Pad 2", "Multi Pad 3", "Multi Pad 4", "Rhythm 1", "Rhythm 2", "Bass",
    "Chord 1", "Chord 2", "Pad", "Phrase 1", "Phrase 2",
];

/// The rows in the order shown: the keyboard parts, the Style parts, then the pads.
const ROW_ORDER: [usize; CHANNELS] = [0, 2, 3, 1, 8, 9, 10, 11, 12, 13, 14, 15, 4, 5, 6, 7];

const BUS_NAMES: [&str; crate::fx::BUSES] = ["Reverb", "Chorus", "Variation"];
/// The insertion effect rows (#269), by Style part.
const INSERT_NAMES: [&str; crate::perf::INSERTS] = ["Ins Rhythm1", "Ins Rhythm2", "Ins Bass", "Ins Chord1", "Ins Chord2", "Ins Pad", "Ins Phrase1", "Ins Phrase2"];

fn us(ns: u64) -> String {
    format!("{:.0}", ns as f64 / 1000.0)
}

fn db(peak: f32) -> String {
    if peak <= 1e-6 { "-".into() } else { format!("{:.1}", 20.0 * peak.log10()) }
}

fn hms(d: Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// One frame, as lines.
pub fn render(s: &Snapshot, uptime: Duration) -> Vec<String> {
    let mut out = Vec::new();
    let rate = |n: u64| if s.secs > 0.0 { n as f64 / s.secs } else { 0.0 };
    let deadline = s.deadline_ns();
    let pct = |ns: u64| if deadline > 0 { ns as f64 * 100.0 / deadline as f64 } else { 0.0 };
    let cb = &s.callback;
    let calls = cb.count.max(1);
    out.push(format!(
        "yahaha top · up {} · RSS {:.1} MB · every {:.0} s",
        hms(uptime),
        s.rss_bytes as f64 / (1 << 20) as f64,
        s.secs.max(0.0)
    ));
    if cb.count == 0 {
        out.push("AUDIO    no audio callbacks (the synth is off, or the device stopped)".into());
    } else {
        out.push(format!(
            "AUDIO    {} frames @ {:.1} kHz · deadline {} µs · {:.0} callbacks/s",
            s.frames,
            s.sample_rate as f64 / 1000.0,
            us(deadline),
            rate(cb.count)
        ));
        out.push(format!(
            "         avg {} µs · p99 {} µs · max {} µs · load {:.0}% (worst {:.0}%)",
            us(cb.mean_ns()),
            us(cb.p99_ns),
            us(cb.max_ns),
            pct(cb.mean_ns()),
            pct(cb.max_ns)
        ));
    }
    let drops = match s.dropouts {
        Some((x, l)) => format!("device {x} · late {l} (since start)"),
        None => "no synth".into(),
    };
    out.push(format!("         dropouts: {drops} · voices {} (peak {})", s.voices, s.voices_peak));
    let stages: Vec<String> =
        STAGES.iter().zip(&s.stages).map(|(n, (sum, max))| format!("{n} {}/{}", us(sum / calls), us(*max))).collect();
    out.push(format!("         stages avg/max µs: {}", stages.join("  ")));
    let rings: Vec<String> = RINGS.iter().zip(&s.rings).map(|(n, d)| format!("{n} {d}")).collect();
    out.push(format!("         synth rings, deepest: {}", rings.join("  ")));
    let e = &s.engine;
    out.push(format!(
        "ENGINE   {:.0} wakes/s · work avg {} µs p99 {} µs max {} µs · woke late p99 {} µs max {} µs · queues input {} ui {}",
        rate(e.count),
        us(e.mean_ns()),
        us(e.p99_ns),
        us(e.max_ns),
        us(s.engine_late.p99_ns),
        us(s.engine_late.max_ns),
        s.engine_queue[0],
        s.engine_queue[1]
    ));
    out.push(format!(
        "MIDI IN  {:.0} packets/s · timestamp -> callback p99 {} µs max {} µs",
        rate(s.midi_packets),
        us(s.midi_in.p99_ns),
        us(s.midi_in.max_ns)
    ));
    out.push(String::new());
    out.push(format!("{:<12} {:>3}  {:<6} {:>6}  {:>7}  {:>8}  {:>8}  {:>6}", "PART", "CH", "SRC", "VOICES", "PEAK dB", "AVG µs", "MAX µs", "%BUF"));
    for ch in ROW_ORDER {
        let r = &s.channels[ch];
        let avg = r.sum_ns / calls;
        out.push(format!(
            "{:<12} {:>3}  {:<6} {:>6}  {:>7}  {:>8}  {:>8}  {:>6.1}",
            PART_NAMES[ch],
            ch + 1,
            if r.plugin { "plugin" } else { "sf2" },
            if r.plugin { "-".to_string() } else { r.voices.to_string() },
            db(r.peak),
            format!("{:.1}", avg as f64 / 1000.0),
            us(r.max_ns),
            pct(avg)
        ));
    }
    for (b, (sum, max, peak)) in s.buses.iter().enumerate() {
        let avg = sum / calls;
        out.push(format!(
            "{:<12} {:>3}  {:<6} {:>6}  {:>7}  {:>8}  {:>8}  {:>6.1}",
            BUS_NAMES[b],
            "fx",
            "bus",
            "-",
            db(*peak),
            format!("{:.1}", avg as f64 / 1000.0),
            us(*max),
            pct(avg)
        ));
    }
    // The insertion effects that ran in the window.
    for (i, (sum, max, peak)) in s.inserts.iter().enumerate() {
        if *sum == 0 && *max == 0 {
            continue;
        }
        let avg = sum / calls;
        out.push(format!(
            "{:<12} {:>3}  {:<6} {:>6}  {:>7}  {:>8}  {:>8}  {:>6.1}",
            INSERT_NAMES[i],
            i + 9,
            "insert",
            "-",
            db(*peak),
            format!("{:.1}", avg as f64 / 1000.0),
            us(*max),
            pct(avg)
        ));
    }
    out
}

/// Draw the view until `stop` says so, collecting as it goes. On a terminal it redraws
/// in place every [`REFRESH`]; otherwise it prints a frame every [`LOG_REFRESH`].
pub fn run(stop: impl Fn() -> bool) {
    super::enable();
    let start = Instant::now();
    let tty = std::io::stdout().is_terminal();
    let every = if tty { REFRESH } else { LOG_REFRESH };
    let mut last = Instant::now();
    // Throw away what was collected before the first refresh started timing it.
    let _ = super::take(0.0);
    while !stop() {
        std::thread::sleep(Duration::from_millis(50));
        if last.elapsed() < every {
            continue;
        }
        let secs = last.elapsed().as_secs_f64();
        last = Instant::now();
        let lines = render(&super::take(secs), start.elapsed());
        let mut o = std::io::stdout().lock();
        if tty {
            // Home, each line cleared to its end, the rest of the screen cleared: no flicker.
            let _ = write!(o, "\x1b[H");
            for l in &lines {
                let _ = write!(o, "{l}\x1b[K\r\n");
            }
            let _ = write!(o, "\x1b[J");
        } else {
            for l in &lines {
                let _ = writeln!(o, "{l}");
            }
            let _ = writeln!(o);
        }
        let _ = o.flush();
    }
}

/// Run the view on a thread of its own for the life of the process (the desktop app).
pub fn spawn() -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new().name("yahaha-top".into()).spawn(|| {
        if std::io::stdout().is_terminal() {
            print!("\x1b[2J");
        }
        run(|| false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf::{ChannelRow, Window};

    #[test]
    fn a_frame_has_the_totals_then_a_row_per_part_and_effect() {
        let mut s = Snapshot {
            secs: 1.0,
            rss_bytes: 300 << 20,
            callback: Window { count: 750, sum_ns: 750 * 92_000, max_ns: 472_000, p99_ns: 288_000 },
            frames: 64,
            sample_rate: 48_000,
            voices: 42,
            voices_peak: 60,
            dropouts: Some((2, 1)),
            ..Snapshot::default()
        };
        s.channels[0] = ChannelRow { sum_ns: 750 * 14_000, max_ns: 40_000, voices: 12, peak: 0.25, plugin: false };
        s.channels[2] = ChannelRow { plugin: true, ..ChannelRow::default() };
        let lines = render(&s, Duration::from_secs(3723));
        let text = lines.join("\n");
        assert!(lines[0].starts_with("yahaha top · up 01:02:03 · RSS 300.0 MB"), "{text}");
        assert!(text.contains("64 frames @ 48.0 kHz · deadline 1333 µs · 750 callbacks/s"), "{text}");
        assert!(text.contains("avg 92 µs · p99 288 µs · max 472 µs · load 7% (worst 35%)"), "{text}");
        assert!(text.contains("dropouts: device 2 · late 1 (since start) · voices 42 (peak 60)"), "{text}");
        let right1 = lines.iter().find(|l| l.starts_with("Right 1")).unwrap();
        assert!(right1.contains("sf2") && right1.contains("12") && right1.contains("-12.0") && right1.contains("14.0"), "{right1}");
        assert!(lines.iter().any(|l| l.starts_with("Right 2") && l.contains("plugin")), "{text}");
        assert_eq!(lines.iter().filter(|l| l.contains(" fx  bus")).count(), 3, "reverb, chorus, variation");
        assert_eq!(lines.len(), 10 + CHANNELS + 3);
        // An insertion effect that ran gets its own row (#269).
        s.inserts[3] = (750 * 8_000, 20_000, 0.5);
        let lines = render(&s, Duration::from_secs(1));
        let ins = lines.iter().find(|l| l.starts_with("Ins Chord1")).expect("the Chord 1 insert row");
        assert!(ins.contains(" 12  insert") && ins.contains("8.0") && ins.contains("-6.0"), "{ins}");
        assert_eq!(lines.len(), 10 + CHANNELS + 4);
    }
}
