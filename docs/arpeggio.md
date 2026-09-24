# Arpeggio engine

`src/arp` is the arpeggio core for #33. It takes the held right-hand notes, the style
tick clock and the arpeggio settings, and produces note events. It is pure and
deterministic like `engine.rs`: there are no threads and no wall time, and nothing
allocates after construction. The live wiring is `src/live/kbdfx.rs` (see "Wiring").

The Genos arpeggio *engine* is in scope, but its preset *patterns* are not
(genos-features.md §6 and "Out of scope"). Yamaha's pattern data is copyrighted and
internal. Every pattern in `src/arp/library.rs` was written for yahaha and is not a
transcription of a Yamaha type.

## API

```rust
use yahaha::arp::{library, Arp, ArpEvent, ArpSink, Settings, Quantize, Velocity};

let mut arp = Arp::new(ppq, library::find("Climb 16").unwrap().clone());
arp.set_settings(Settings { quantize: Quantize::Sixteenth, hold: true, ..Settings::default() }, now);

arp.note_on(60, 100, now);          // key down (velocity 0 = key up)
arp.note_off(60, now);              // key up
arp.set_sustain(true, now);         // sustain pedal (only used when `sustain_holds` is on)
arp.set_hold(false, now);           // Arpeggio Hold on/off (the pedal function)
arp.set_pattern(other.clone(), now);

let mut events: Vec<ArpEvent> = Vec::new();   // or any `impl ArpSink`
arp.process(from..to, &mut events);  // every event with from <= tick < to, in time order

arp.stop(now);                      // button off: forget the notes, cut what is sounding
arp.all_off(now, &mut sink);        // stop and send the note-offs right away
```

- **Ticks** are on the caller's clock at `ppq` ticks per quarter note, normally the style
  clock, so the arp stays in phase with the accompaniment.
- **`process`** is called with contiguous ranges. An event due before `range.start` goes
  out at `range.start`. This happens with a quantized start that snapped back to a grid
  line just behind the key press, and with a cut at an earlier tick. Output does not
  depend on how the ranges are chunked (tested with 1-, 7- and 4995-tick chunks).
- **Clock jumps.** If a range starts before the previous one ended (for example, the
  style restarts at tick 0), every sounding note is cut at `range.start`. A running
  pattern then starts again from step 1 at that tick, or on the next grid line when
  Quantize is on. If a range starts after the previous one ended, the steps in the
  gap are skipped and only the latest one plays, at `range.start`. A late quantized
  start works the same way. Empty and reversed ranges do nothing. An unvalidated
  pattern with no steps plays nothing, and a zero step length counts as one tick, so
  neither can panic or hang the real-time thread.
- **No allocation** in `process`, `note_on`, `note_off`, `set_sustain`, `set_settings`,
  `set_hold`, `stop` or `all_off`. `tests/arp_no_alloc.rs` checks this with a counting
  global allocator. `set_pattern` moves the new pattern in and drops the old one. The
  library patterns are borrowed `Cow`s, so swapping between them frees nothing.
- **Limits:** 16 notes held at once (later keys are ignored) and 64 queued note-ons
  (strummed notes waiting for their offset).

### Balanced note-offs

The engine tracks each pitch it starts until that pitch's note-off has gone out:

- A pitch the pattern retriggers while it is still sounding (gate over 100%, or a note
  in two octaves of the walk) is released at the same tick, just before it restarts.
- `set_pattern` and `stop` cut every sounding note at their tick. `all_off` sends the
  cuts at once.
- Releasing every key stops the clock (unless Keep Key On is set). Notes still sounding
  finish their gate and then get their offs.
- When events fall on the same tick, the order is note-offs, then queued note-ons, then
  the next step.

`fuzz_note_offs_always_balance` drives 40 seeded runs of 3000 random operations, mixing
notes, hold, pedal, pattern and settings changes, stop and all-off, over random chunk
sizes. It checks that no pitch starts twice, no off arrives for a silent pitch, time never
goes backwards, and nothing is left sounding after `stop`.

## Pattern format

A `Pattern` is a loop of equally long `Step`s. It is serde-serialisable, so patterns can
also come from TOML or JSON files later.

| Field | Meaning |
|---|---|
| `name`, `category` | Browser name and group: UpDown, Random, AsPlayed, ChordStab, BrokenChord, Guitar, Sequence |
| `step_len` | Step length in ticks at 480 PPQ (`len::SIXTEENTH` = 120, `len::EIGHTH_TRIPLET` = 160, …); scaled to the engine's PPQ |
| `swing` | 50 = straight. Odd steps are delayed by `2 × (swing − 50)%` of a step: 67 ≈ triplet shuffle, 75 = dotted. Clamped to 50–75 |
| `motion` | How `Walk` steps move: Up, Down, UpDown, DownUp (ends not repeated), Random (seeded, no immediate repeat), AsPlayed |
| `octaves` | Octaves a walk spans, 1–4 |
| `sort` | Order that `Idx`/`Top` count in: Pitch (low first) or Played (first pressed first) |
| `seed` | Seed for Random. The sequence restarts with the pattern, so it is repeatable |
| `steps` | The loop |

Each `Step` has `sel` (which notes), `oct` (octave shift, −3 to +3), `vel` (1–127) and
`gate` (percent of the step, over 100 overlaps the next step). `sel` is one of:

| `Sel` | Plays |
|---|---|
| `Rest` | nothing |
| `Walk` | the next note of `motion` |
| `Idx(i)` | the i-th note from the bottom. Past the top it wraps and goes up an octave, so with C E G, `Idx(3)` is the C an octave up |
| `Top(i)` | the i-th note from the top. Past the bottom it wraps and goes down an octave |
| `All` | the whole chord at once |
| `Strum { up, spread }` | the whole chord rolled `spread` ticks (at 480 PPQ) apart. `up: false` is a downstroke (low to high); `up: true` is an upstroke (high to low) |

With these, the pattern follows whatever is held: Up/Down/Random walk any number of
notes, `Idx` figures wrap into higher octaves on small chords, and stabs and strums take
the whole chord. Notes that would fall outside 0–127 are skipped.

## Settings

| Setting | Behaviour |
|---|---|
| `quantize` | Arpeggio Quantize (RM p.41): Off, Eighth or Sixteenth. The pattern starts on the grid line **nearest** the first key. A key pressed slightly late plays at once and the following steps land on the grid; a key pressed slightly early waits for the grid. The grid is counted from tick 0 of the caller's clock (the style's bar line). A pattern change with quantize on waits for the next grid line |
| `hold` | Arpeggio Hold / latch (RM p.41, p.141): the pattern keeps playing after release. The first key after a full release starts a new chord that replaces the latched one; keys added while others are down join the chord. Turning hold off drops every note that is no longer physically down |
| `velocity` | `Original` (the pattern's step velocities), `Thru` (the velocity the source key was played with) or `Fixed(v)`. A yahaha extension: the Genos manuals have no such setting (it is a Motif/MONTAGE idea) |
| `vel_scale` | ArpVel, percent, 0–200. Results are clamped to 1–127 |
| `gate_scale` | ArpGateT, percent, 1–400. The gate is never shorter than one tick |
| `unit_multiply` | ArpUnitM, percent, 25–400: 200 is half speed and 50 is double speed. A change takes effect from the next step, which keeps its time |
| `keep_key_on` | The pattern clock keeps running through a full release, so the next chord picks up mid-phrase, in phase, instead of restarting from step 1. Steps with nothing held are silent. Only `stop` (or turning this off with nothing held) stops the clock. A yahaha extension, not a Genos setting |
| `sustain_holds` | While the sustain pedal is down, released keys stay in the arpeggio, and new keys join them. Pedal up drops the released ones |

Without Hold or Keep Key On, releasing every key stops the pattern, and the next key
restarts it from step 1 and resets the walk and random seed.

Step times are computed as `anchor + k × step_len × ppq × unit_multiply / 48000`, not
accumulated. Fractional step lengths (Unit Multiply 133% gives a 159.6-tick 16th at 480
PPQ) round per step and never drift.

## Starter library

23 patterns. All are original and written for yahaha.

| Category | Patterns |
|---|---|
| Up & Down | Climb 16, Fall 16, Peak 8 (up-down, 2 oct), Valley Triplet (down-up), Sky Ladder 32 (3 oct) |
| Random | Dice 16, Scatter Octaves (2 oct) |
| As Played | Echo Order 8, Shuffle Order 16 (swing 62, 2 oct) |
| Chord stab | Four Stabs, Offbeat Pump, Syncopated Hits, Gated Pad 16 |
| Broken chord | Alberti 16, Waltz Broken (3/4), Rolling Eights, Thumb Pick (alternating bass) |
| Guitar | Strum Quarters, Campfire Strum (down/up), Muted Sixteens |
| Sequence | Octave Pulse, Root Fifth Seq, Pluck Line |

## Wiring (#33)

- **One switch, one type** with Keyboard Harmony (docs/harmony.md): the arpeggio is the
  HARMONY/ARPEGGIO type when an arpeggio pattern is selected.
- **Keys:** the input thread's processor swallows the keys right of the split and hands
  them to the engine thread (a ring), marking them in a held-key mask. Every engine wake
  releases from the arp any key no longer in the mask, so a lost key-up can never leave a
  note in the pattern (`Arp::keys_down`).
- **Clock:** the arp runs on the engine thread in ticks of the style clock, at eight
  sub-ticks per style tick (`live::kbdfx::SUB`, so its ppq is the style's × 8). It plays what
  is due before the current sub-tick: never early, at most 1/8 tick late. The engine wakes
  for `Arp::next_due`.
  - With the band running, the pattern is in phase with the style and Quantize snaps to its
    grid. START resets the style clock to 0: `Arp::jump` cuts and restarts the pattern
    there.
  - With the band stopped, the engine's clock runs on at the tempo from where it stopped:
    that is the arp's own clock (the grid is then arbitrary, but steady).
  - A style change to another resolution calls `Arp::set_ppq` at the bar line: sounding
    notes are cut (their off ticks meant the old clock) and the pattern starts again from
    step 1 there.
- **Output:** on the Right parts Assign picks (Auto: every Right part that is on, as the
  keys would play; Right 1-3: that part). Multi is only offered for the Harmony and Echo
  categories (RM p.46): the app hides it for an arpeggio, and a Multi left over from a
  Harmony type plays as Auto (Decision, #100 review N2), at each part's octave and the
  Keyboard transpose, with velocities scaled by Volume (HrmArpVol, 127 = unchanged).
  Every note is counted per (channel, note) and released where it sounded.
- **Stop:** the switch off or another type calls `Arp::all_off` (the offs go out at once);
  PANIC and shutdown too.
- **Settings in the app:** pattern, Quantize, Hold, Velocity (Original/Thru/Fixed), Keep Key
  On, plus Volume and Assign. The Live Control percentages (ArpVel, ArpGateT, ArpUnitM) stay
  at 100%: the Launchkey has no spare controls for them yet.

Arpeggio Hold and the HARMONY/ARPEGGIO switch are also pedal functions (Settings ›
Pedals: Arpeggio Hold, Kbd Harmony/Arpeggio On/Off; RM p.141), with a Control Type:
Hold A holds the arpeggio while the pedal is down (docs/controllers.md).

## Not done yet

- The Live Control percentages and the sustain-pedal hold (`sustain_holds`) in the app.
- Loading user patterns from TOML files. The format already (de)serialises.
