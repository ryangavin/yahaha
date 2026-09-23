# yahaha — plan

A software arranger keyboard for jamming. It loads Yamaha Genos style files (`.sty .prs .sst .bcs .pcs .pst .fps`), listens to your connected MIDI keyboards, follows your chords instantly, and plays the band out of a virtual MIDI port. You pick the sounds yourself (Ableton or any synth).

**Scope:** live jamming only. There is no MIDI-file rendering and no built-in synth.

## 1. What a style file is

Every extension uses the same container. It is a **type-0 Standard MIDI File** with extra chunks after the `MTrk`. Each chunk is a 4-byte ASCII ID followed by a 4-byte big-endian length. Parsers must not assume the chunks' order or that any optional chunk is present.

| Chunk | Contents | Needed? |
|---|---|---|
| `MThd`/`MTrk` | All the patterns in one track, cut into sections by **marker** meta events. SFF2 uses 1920 PPQ. | yes |
| `CASM` → `CSEG`* → `Sdec` + `Ctab`/`Ctb2`* (+`Cntt`*) | Per-section, per-source-channel rules for transposing the pattern to follow chords | yes, this is the core |
| `OTSc` | 4 One Touch Settings (right-hand voice setups), stored as small MIDI tracks | later |
| MDB | Music Finder records (song titles, genres) | ignore |
| `MHhd`/`MHtr` | Unknown and rare | preserve, ignore |

**Markers in the MIDI track:**
- Measure 1 holds `SFF1` or `SFF2` (the format flag), the style name, then `SInt`. After `SInt` come bank select, program change, volume, reverb and chorus sends, and XG effect SysEx for each channel.
- The sections follow from measure 2: `Intro A–C`, `Main A–D`, `Fill In AA/BB/CC/DD`, `Fill In BA` (= Break), `Ending A–C`.
- Newer instruments re-apply `SInt` every time the section changes. yahaha should do the same.

**Parts.** Output lands on the 8 accompaniment channels:

| Ch | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|
| Part | Rhythm 1 (sub) | Rhythm 2 (main) | Bass | Chord 1 | Chord 2 | Pad | Phrase 1 | Phrase 2 |

CASM can route any of the 16 *source* channels to these parts, and several sources may share one part (for example, one source for major chords and another for minor chords). Without CASM, the channels map 1:1 and every pattern is written in **CMaj7**.

**`Ctab` fields (SFF1):**
- source channel, name, destination part
- **note mute** (12-bit mask of chord roots)
- **chord mute** (34-bit mask of chord types; bit 2 of byte 13 = autostart)
- **source root and chord type** (default C / Maj7)
- **NTR**: Root Trans / Root Fixed
- **NTT**: Bypass, Melody, Chord, Bass, Melodic minor, Harmonic minor
- **High Key**, **Note Low/High Limit**
- **RTR**: Stop, Pitch shift, Pitch shift to root, Retrigger, Retrigger to root, Note generator
- special feature bytes (the extra break drum voice)

**`Ctb2` fields (SFF2, used by Genos):**
- The same header fields as `Ctab`.
- The note range is split into **low, mid and high zones**. Each zone has its own `{NTR, NTT, HighKey, Lo, Hi, RTR}`.
- NTR gains **Guitar**, which has its own NTT set: All-Purpose / Stroke / Arpeggio.
- NTT gains Natural minor, Dorian and the "5th Var." tables.
- Bit 7 of NTT is **Bass On**, which lets that part follow slash chords.
- 7 trailing bytes are still undocumented. They look drum-related.

Every byte offset and enum table is in the Wierzba/Bedesem spec (see Sources). The parser is well-understood work. The **behaviour** (how NTT actually maps notes) is not documented anywhere, and that is the real reverse-engineering job.

## 2. Keyboard behaviour to reproduce (from the Genos / Genos2 manuals)

**Chord recognition, by fingering type:**
- **Single Finger**: 1 key = major, + white key below = 7th, + black key below = minor, + both = m7.
- **Fingered**: the full chord table.
- **Fingered On Bass**: the lowest note becomes the bass, so slash chords work.
- **Multi Finger**: auto-detects Single Finger or Fingered.
- **AI Fingered**: infers a full chord from fewer than 3 notes, using the previous chord.
- **Full Keyboard / AI Full Keyboard**: detects chords across the whole keyboard.
- **Chord Cancel** (a special fingering) mutes everything except rhythm.
- **Chord types**: the 34 types in the CASM chord-mute table, plus `1+5`, `1+8`, `1+2+5`, `sus4` and cancel. The exact fingerings come from the Genos Data List ("Chord Types Recognized in the Fingered Mode").

**Split points:**
- The Style split point sets the chord zone. The Left split point sets the left-hand voice.
- Chord detection can also be set to *Upper* (right hand).

**Transport:**
- START/STOP, SYNC START (starts on the first chord) and SYNC STOP (stops when you release the keys, with an adjustable hold window).
- **Stop ACMP**: with the style stopped, a chord sounds on the Pad and Bass voices. Settings: Off / Style / Fixed.

**Sections:**
- **Intro I–III**: plays once, then goes to the selected Main.
- **Main A–D**: loops.
- **Fill-ins**:
  - Pressing the current Main again plays its fill.
  - With **AUTO FILL IN**, switching Main plays a fill that leads into the target.
  - A fill is 1 bar, and the style returns to the Main at the bar line.
- **Break**: 1 bar.
- **Ending I–III**: plays once, then stops. Pressing it again during playback adds a ritardando.

**Section-change timing:**
- To a Main: *Immediate* (the new section picks up on the same beat) or *Next Bar*.
- Inside an Intro or Ending: *Next Bar* or *End of Section*.
- Changing Main during a pattern keeps the bar position; it does not restart the pattern.

**Other controls:**
- **Style Section Reset**: the Tap Tempo button jumps back to the top of the current section (a stutter effect).
- **Channel on/off, solo**, and the rules for what happens on a style change (Lock / Hold / Reset) for tempo, part on/off and the default section.
- **OTS / OTS Link**: Main A–D recall One Touch Settings 1–4, either immediately or at the section change.
- **Style Creator "Groove"**: timing templates (swing A–E, beat converter, push/heavy). This is a nice-to-have.

**Style types:**
- **Session** styles depend on the extra NTT tables.
- **DJ** styles lock the chord type and follow the root only.
- **Free Play** styles are rubato.
- **+Audio** styles add time-stretched audio drums, muted above 160% tempo. They are out of scope until later.

## 3. Architecture

Stack: **Rust**. The engine is timing-critical, and the crates it needs already exist.

```
yahaha/
  crates/
    sff/        parse + write: SMF, CASM/Ctab/Ctb2/Cntt, OTSc, MDB; lossless round-trip
    theory/     chord types, chord recognizer (all fingering modes), NTT/NTR/RTR transposer
    engine/     real-time sequencer: section state machine, pattern cursor, chord follow, held-note RTR
    voices/     Genos bank/program → GM/GS fallback table; drum-kit note remap
    io/         midir: merge all connected MIDI inputs; virtual out port "yahaha"; Ableton Link
  apps/
    yahaha/     `yahaha play <style>` (terminal UI) + `yahaha dump <style>`
  corpus/       test styles (git-ignored) + golden outputs
```

**Core model:**
```
Style { name, tempo, timesig, init: SInt events,
        sections: Map<SectionId, Pattern> }
Pattern { length_ticks, tracks: Vec<SourceTrack> }
SourceTrack { src_ch, dest_part, events, rules: Ctb2 (per section, via Sdec) }
Chord { root, type, bass: Option<PitchClass> }
```

**Engine loop.** The transport clock is internal or driven by Ableton Link. On each tick:
1. Advance the pattern cursor.
2. For each source note-on, find the rules for that note's zone. Apply chord mute and note mute. Transpose from the source chord to the current chord with NTT and NTR. Apply High Key, then fold into the note limits.
3. Emit the note on its destination part.

When the chord changes mid-note, apply RTR to the sounding notes: stop them, send pitch-bend, or retrigger. Section changes are queued and applied at the next beat or bar, as §2 describes.

**Threads (real-time design).** The input side and the output side never wait on each other. There are no locks, allocations, or file/terminal I/O on either real-time path.

| Thread | Priority | Owns | Talks to others via |
|---|---|---|---|
| **Input**: CoreMIDI's own receive thread (our callback) | CoreMIDI real-time | held-note state, chord recognition, forwarding the player's own notes | chord packed into one `AtomicU32` (root/type/bass/generation); section/transport commands on a wait-free SPSC ring; one non-blocking semaphore signal |
| **Engine/output**: ours | Mach time-constraint (audio-thread class) | transport clock, pattern cursors, section state machine, transposition, RTR, sending | sleeps with `semaphore_timedwait(until next event deadline)`, so it wakes on time or on a chord change, whichever comes first; state snapshots out to the UI through an SPSC ring |
| **Control/UI** | normal | terminal UI, style loading, logging | new styles are precomputed off the real-time threads and swapped in by atomic pointer at a bar line; old ones are freed here, never on the engine thread |

- Each style track's transposition is precomputed at load into lookup tables: `[chord type][source note] → offset`. At play time it costs one table read plus the root offset.
- Events are sent at their exact time with timestamp "now". There is no lookahead buffer, so a new chord applies to the very next note.
- macOS has no hard core pinning; on Apple Silicon the affinity API is ignored. The time-constraint policy is what keeps the scheduler from preempting the threads. It's the same mechanism CoreAudio uses.
- Verification: `yahaha bench` sends notes through a loopback virtual port and measures input → output latency (p50/p99/max) and scheduling jitter (planned vs actual send time). Target: yahaha adds < 0.5 ms p99. The rest of the 5 ms round trip is USB plus the audio buffer.

**I/O.**
- **Input:** every connected MIDI keyboard is opened and merged, with a per-device role (chord zone / whole keyboard / controller pads).
- **Output:** a CoreMIDI virtual source called `yahaha`. Parts go out on channels 9–16. Put one track per part in Ableton (or any synth) with its input set to `yahaha` on that channel.
- **Tempo:** Ableton Link, so no MIDI-clock hassle.
- **SInt setup:** bank and program changes from the style are forwarded as-is, and the display shows the voice names.

**Voices (labels only).** The synth is yours, so this only affects labels.
- Genos voices are addressed by bank MSB/LSB + program: 0/0-101 XG, 8/9/109 Mega/SArt, 104 "New", 121 GM2, 126/127 drums, and so on.
- Build a table from the Genos **Data List voice list** that maps each voice to its nearest GM2 program and each drum kit to a GM kit.
- Ship it as data (`voices/genos.toml`), not code. It's only used to label parts ("Bass: Finger Bass"), so you know which sound to load.

## 4. The hard part: transposition fidelity

The NTT tables are not published. The one open-source SFF2 tool (sff2-tools) skips CASM for exactly this reason. The plan:

1. **Start from the documented semantics.**
   - Bypass = no change.
   - Melody = scale-degree mapping for the new chord type.
   - Chord = map the "most important" notes of the source chord to the target chord's most important notes. For 4-note chords that means dropping the root first; for 5-note chords, the root and the 5th. No doubled notes.
   - Melodic and Harmonic minor = lower or raise the 3rd, or the 3rd and 6th.
   - Guitar NTR = source notes B, A, G, F, E, D mapped to strings 1–6; C/C# = root/5th; the octave selects the fret position.
   - High Key and Note Limit examples are given explicitly in the spec. Encode them as unit tests.
2. **Read JJazzLab's YamJJazz engine (Java, LGPL-2.1).** It is the most mature open-source Yamaha style player. Use it for approach, not as copied code.
3. **Get ground truth from an oracle**, best first:
   - **A real Genos / PSR-SX / Tyros.** The instrument transmits the style's parts over USB-MIDI. Script the chords via MIDI-in (every root × every chord type × each section) and record what comes out. One afternoon with borrowed hardware gives a complete golden dataset.
   - **Freeware Windows players** (Bedesem's StylePlayer/MidiPlayer, Jososoft's Midi and Style Player). Run them under Wine or on a Windows VM, feed chords over a loopback MIDI port, and record the output. Treat that output as "someone else's approximation", not truth.
4. **Golden tests.** Each case is a `(style, section, chord sequence) → expected MIDI`. Record each case once, diff on every change, and measure % note agreement per NTT type.

## 5. Milestones

**The goal is to jam, not to make backing tracks.** You sit down and play a chord, and the band starts and follows you with no perceptible delay. JJazzLab already covers backing tracks. Its arranger mode lags because it regenerates the backing whenever the chord changes. yahaha instead transposes each pattern note at the moment it plays, so a chord change reaches the very next note, plus the RTR handling for notes that are already sounding.

**Feel targets:**
- Chord in → first note out in under 5 ms.
- Sync Start is on by default: the band waits for your first chord.
- Launchkey pads and buttons are mapped to the arranger controls:
  - Intro I–III, Main A–D (press twice = fill), Break, Ending I–III
  - Start/stop, tap tempo
  - Part mutes
- Launching yahaha puts you straight into play mode with the last style loaded.

| # | Deliverable | Done when |
|---|---|---|
| M0 | **Corpus + `yahaha dump`**: lossless parse/write of all chunks; prints sections, per-channel Ctab/Ctb2, SInt voices | ~100 real styles parse; `write(parse(f)) == f` byte-for-byte |
| M1 | **`theory` crate**: chord recognizer (Fingered, On Bass, Single, AI Fingered) + NTT/NTR/HighKey/limits transposer | Spec examples and hand-checked cases pass |
| M2 | **`yahaha play style.sty`: the target build.** All connected keyboards in → virtual `yahaha` port → Ableton on channels 9–16, Link tempo, Sync Start/Stop, Main A–D + fills + break, intro/ending, RTR on chord changes | You play the Launchkey and the band follows instantly in Ableton. **First useful release.** |
| M3 | **Controller mapping + status display** (small egui window: style browser, section lights, chord display, tempo) | Everything is controllable from the Launchkey; the screen is only for looking |
| M4 | **Fidelity pass** against the oracle data: Ctb2 zones, Guitar NTR, Session NTTs, AI Fingered, Full Keyboard, Stop ACMP, OTS, groove | Note agreement ≥ 95% on golden tests |
| M5 | Stretch goals: Chord Looper, OTS right-hand voice changes, +Audio styles | — |

## 6. Getting style files

The Genos's preset styles live in its firmware, and **Expansion Packs (`.cpi`/`.ppi`) are encrypted, so leave them alone.** Legal sources:
- Yamaha's free style downloads for Genos and PSR-SX
- The large free user-style libraries on PSR Tutorial (psrtutorial.com)
- Styles you already own as unencrypted `.sty` files

Keep the corpus git-ignored and never redistribute it. Reverse engineering the format for interoperability is fine. Shipping Yamaha's content is not.

## 7. Open questions

- The undocumented Ctb2 bytes 40–46. They matter for drum channels. Compare channels that differ only in these bytes using the oracle.
- The "Note generator" RTR. (`Cntt` precedence is settled from corpus evidence in #14. See "SFF1 encoding" in docs/genos-features.md.)
- Whether a Genos-era style carries chunks that the 2015 spec doesn't cover. M0's dump will flag any unknown chunk IDs.
- +Audio style audio storage. See the "Audio Style file format" write-up at sandsoftwaresound.net before M6.

## Sources

- Wierzba & Bedesem, *Style Files – Introduction and Details* v2.1: https://wierzba.hier-im-netz.de/StyleFileDescription_v21.pdf
- Genos Owner's Manual: https://data.yamaha.com/files/download/other_assets/7/1130977/genos_en_om_h0.pdf
- Genos2 Reference Manual: https://data.yamaha.com/files/download/other_assets/1/2318561/Genos2_reference_manual_En_C0.pdf
- Jososoft style articles (CASM): http://www.jososoft.dk/yamaha/articles/style2_2.htm
- sff2-tools (Python, MIT): https://github.com/bures/sff2-tools
- JJazzLab (Java, LGPL): https://github.com/jjazzboss/JJazzLab
- YamahaArranger (Android): https://github.com/pulicarpus/YamahaArranger
- Audio Style format notes: https://sandsoftwaresound.net/audio-style-file-format/
