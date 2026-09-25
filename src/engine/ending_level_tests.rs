//! A section that follows an Ending does not inherit the Ending's fade (#122, #129).
//!
//! About half the corpus Endings fade their parts with expression (CC11), and some set a
//! lower level (CC7) at their start. A Main (or a Break, an Intro) pressed during the
//! Ending takes over at a bar line while the band plays on, so no start or style load
//! puts the setup back: it has to be the section change. The test: a Main reached from an
//! Ending starts at the same part levels and expression as the same Main started fresh.

use super::*;

#[derive(Default)]
struct Rec;

impl Sink for Rec {
    fn send(&mut self, _: &[u8]) {}
}

fn corpus(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

const NIGHT_CRUISER: &str = "corpus/MOX_v2/NightCruiser.S930.STY";

/// The style at `p`; None only when the corpus is missing.
fn engine(p: &std::path::Path) -> Option<Engine> {
    if !p.exists() {
        return None;
    }
    let style = Style::load(p).expect("style loads");
    Some(Engine::new(Box::new(Prepared::new(&style))))
}

fn chord(s: &str) -> Chord {
    crate::parse_chord(s).unwrap()
}

/// Each Style part's (level CC7, expression CC11) as the channel has it: a controller
/// never sent is at the receiver's default (100, 127).
fn levels(e: &Engine) -> [(u8, u8); 8] {
    let at = |v: u8, d: u8| if v == UNSENT { d } else { v };
    std::array::from_fn(|p| (at(e.mirror.cc[8 + p][7], 100), at(e.mirror.cc[8 + p][11], 127)))
}

/// Run the engine to `to` at its deadlines (at most 5 ms apart), stopping early (and
/// returning true) once `stop` holds.
fn play_until(e: &mut Engine, from: u64, to: u64, stop: impl Fn(&Engine) -> bool) -> bool {
    let mut now = from;
    let mut rec = Rec;
    while now < to {
        e.process(now, &mut rec);
        if stop(e) {
            return true;
        }
        now = e.next_deadline().unwrap_or(to).clamp(now + 1, now + 5_000_000).min(to);
    }
    e.process(to, &mut rec);
    stop(e)
}

/// Main `main` started fresh: the levels once its first tick has played.
fn fresh_main(p: &std::path::Path, main: u8) -> Option<[(u8, u8); 8]> {
    let mut e = engine(p)?;
    e.button(Button::Main(main), 0, &mut Rec);
    e.set_chord(chord("C"), 0, &mut Rec);
    e.process(0, &mut Rec);
    (e.running && e.cur == slot_of(SectionId::Main(main))).then(|| levels(&e))
}

/// Where the band is after an Ending: the Ending's own levels just before `next` takes
/// over, and the levels once `next`'s first tick has played.
struct AfterEnding {
    ending: [(u8, u8); 8],
    next: [(u8, u8); 8],
    engine: Engine,
    now: u64,
}

/// Main A plays, Ending `ending` is pressed; in the Ending's last bar (past its first
/// beat, so the change waits for the bar line) `press` asks for the next section. Played
/// until the section `next` takes over.
fn after_ending(p: &std::path::Path, ending: u8, press: Button, next: usize) -> Option<AfterEnding> {
    let mut e = engine(p)?;
    let end = slot_of(SectionId::Ending(ending));
    if !e.style.has(end) || !e.style.has(next) || !e.style.has(slot_of(SectionId::Main(0))) {
        return None;
    }
    e.set_chord(chord("C"), 0, &mut Rec);
    let (ppq, tpb) = (e.style.ppq as f64, e.style.tpb as f64);
    // A bar of Main A, then the Ending at the next bar line.
    let t = e.ns_at(tpb + 1.5 * ppq);
    play_until(&mut e, 0, t, |_| false);
    e.button(Button::Ending(ending), t, &mut Rec);
    let limit = t + 120_000_000_000;
    if !play_until(&mut e, t, limit, |e| e.cur == end) {
        return None;
    }
    let len = e.style.sections[end].as_ref()?.len as f64;
    let last_bar = ((len / tpb).ceil() - 1.0).max(0.0) * tpb;
    let at = e.ns_at(e.sec_start + last_bar + 1.5 * ppq.min(tpb / 2.0));
    let now = e.ns_at(e.sec_start);
    play_until(&mut e, now, at, |_| false);
    if !e.running || e.cur != end {
        return None;
    }
    let before = levels(&e);
    e.button(press, at, &mut Rec);
    let mut now = at;
    let mut rec = Rec;
    let mut ending_levels = before;
    // Step to the change, keeping the Ending's levels up to the step before it.
    while now < limit {
        e.process(now, &mut rec);
        if e.cur == next || !e.running {
            break;
        }
        ending_levels = levels(&e);
        now = e.next_deadline().unwrap_or(limit).clamp(now + 1, now + 5_000_000).min(limit);
    }
    (e.running && e.cur == next).then(|| AfterEnding { ending: ending_levels, next: levels(&e), engine: e, now })
}

const BASS: usize = 2;

/// NightCruiser's Ending II fades the Bass (CC11 down to 55 by its last bar, CC7 100 to 51
/// at its start). Main A pressed during it plays the Bass as Main A started fresh does.
#[test]
fn main_after_night_cruiser_ending_ii_has_main_a_levels() {
    let p = corpus(NIGHT_CRUISER);
    let Some(fresh) = fresh_main(&p, 0) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let a = after_ending(&p, 1, Button::Main(0), slot_of(SectionId::Main(0))).expect("Ending II then Main A");
    assert!(a.ending[BASS].1 < 127 || a.ending[BASS].0 < fresh[BASS].0, "the Ending lowers the Bass: {:?}", a.ending[BASS]);
    assert_eq!(a.next[BASS], fresh[BASS], "Bass (CC7, CC11) in Main A after Ending II");
    assert_eq!(a.next, fresh, "every part in Main A after Ending II");
}

/// The player's own fader is not the Ending's: a Bass level set during the Ending stays
/// through the change to the Main (only expression goes back to full).
#[test]
fn a_fader_moved_during_the_ending_keeps_its_level_into_the_main() {
    let p = corpus(NIGHT_CRUISER);
    let Some(mut e) = engine(&p) else { return };
    e.set_chord(chord("C"), 0, &mut Rec);
    let (ppq, tpb) = (e.style.ppq as f64, e.style.tpb as f64);
    let t = e.ns_at(tpb + 1.5 * ppq);
    play_until(&mut e, 0, t, |_| false);
    let end = slot_of(SectionId::Ending(1));
    e.button(Button::Ending(1), t, &mut Rec);
    assert!(play_until(&mut e, t, t + 60_000_000_000, |e| e.cur == end));
    let (start, now) = (e.ns_at(e.sec_start), e.ns_at(e.sec_start + 1.5 * ppq));
    play_until(&mut e, start, now, |_| false);
    e.set_volume(BASS as u8, 33, &mut Rec);
    e.button(Button::Main(0), now, &mut Rec);
    let main = slot_of(SectionId::Main(0));
    assert!(play_until(&mut e, now, now + 60_000_000_000, |e| e.cur == main));
    assert_eq!(levels(&e)[BASS], (33, 127));
}

/// A Break pressed during the Ending plays after it at full expression, where the Break's
/// own pattern doesn't set one.
#[test]
fn break_after_night_cruiser_ending_ii_is_at_full_expression() {
    let p = corpus(NIGHT_CRUISER);
    let Some(a) = after_ending(&p, 1, Button::Break, slot_of(SectionId::Break)) else { return };
    let sec = a.engine.style.sections[slot_of(SectionId::Break)].as_ref().unwrap();
    for part in 0..8 {
        let ch = 8 + part as u8;
        let own = sec.events.iter().take_while(|e| e.tick == 0).any(|e| {
            matches!(e.kind, PKind::Cc { cc: 11, .. }) && sec.rules[e.src as usize & 15].as_ref().is_some_and(|r| r.dest_ch == ch)
        });
        if !own && a.engine.style.setup(a.engine.cur).init.iter().all(|m| !(m.len() == 3 && m[0] == 0xB0 | ch && m[1] == 11)) {
            assert_eq!(a.next[part].1, 127, "part {part}: expression after the Ending (it had {:?})", a.ending[part]);
        }
    }
    let _ = a.now;
}

/// The Ending ends the song and Synchro Start starts it again: the band starts at Main A's
/// levels (the start sends the setup; #131).
#[test]
fn synchro_restart_after_an_ending_has_main_a_levels() {
    let p = corpus(NIGHT_CRUISER);
    let Some(fresh) = fresh_main(&p, 0) else { return };
    let Some(mut e) = engine(&p) else { return };
    e.set_chord(chord("C"), 0, &mut Rec);
    let (ppq, tpb) = (e.style.ppq as f64, e.style.tpb as f64);
    let t = e.ns_at(tpb + 1.5 * ppq);
    play_until(&mut e, 0, t, |_| false);
    e.button(Button::Ending(1), t, &mut Rec);
    let limit = t + 120_000_000_000;
    let mut now = t;
    let mut rec = Rec;
    while e.running && now < limit {
        e.process(now, &mut rec);
        now = e.next_deadline().unwrap_or(limit).clamp(now + 1, now + 5_000_000).min(limit);
    }
    assert!(!e.running && e.sync_armed, "the Ending stopped the band, Synchro Start armed");
    assert!(levels(&e)[BASS] != fresh[BASS], "the Ending left the Bass lower");
    e.set_chord(chord("F"), now + 1, &mut Rec);
    e.process(now + 1, &mut Rec);
    assert!(e.running && e.cur == slot_of(SectionId::Main(0)));
    assert_eq!(levels(&e), fresh);
}

/// Every corpus style, both Endings: Main A pressed in the Ending's last bar starts at
/// the levels Main A starts at fresh (expression and the style's part levels).
#[test]
fn corpus_main_after_an_ending_has_the_main_levels() {
    let mut paths = Vec::new();
    for dir in ["corpus/MOX_v2", "corpus/SX900Style for Genos", "corpus/T5Style"] {
        let Ok(rd) = std::fs::read_dir(corpus(dir)) else { continue };
        paths.extend(rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sty"))));
    }
    if paths.is_empty() {
        eprintln!("corpus missing; skipping");
        return;
    }
    paths.sort();
    let (mut checked, mut lowered, mut failures) = (0, 0, Vec::new());
    for p in &paths {
        let Some(fresh) = fresh_main(p, 0) else { continue };
        for ending in 0..2 {
            let Some(a) = after_ending(p, ending, Button::Main(0), slot_of(SectionId::Main(0))) else { continue };
            checked += 1;
            if a.ending.iter().zip(&fresh).any(|(x, f)| x.1 < 127 || x.0 != f.0) {
                lowered += 1;
            }
            if a.next != fresh {
                let name = p.file_name().unwrap().to_string_lossy();
                let parts: Vec<String> = (0..8).filter(|&i| a.next[i] != fresh[i]).map(|i| format!("part {i}: {:?} (Ending {:?}) want {:?}", a.next[i], a.ending[i], fresh[i])).collect();
                failures.push(format!("{name} Ending {}: {}", ending + 1, parts.join(", ")));
            }
        }
    }
    eprintln!("{checked} Ending -> Main A changes, {lowered} with an Ending that lowered a part");
    assert!(checked > 20 && lowered > 5, "the corpus exercises the path ({checked}, {lowered})");
    assert!(failures.is_empty(), "{} of {checked}:\n{}", failures.len(), failures.join("\n"));
}
