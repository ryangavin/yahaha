# Registration Memory, Freeze, Registration Sequence, Playlist

Issues #36 and #38. The Genos behaviour is in [genos-features.md](genos-features.md) §9
(OM p.96–103, RM p.113–119, the Data List parameter chart); the commands and state are in
[app-api.md](app-api.md) ("Registration Memory", "Playlist").

## What it does

- **Ten buttons per bank.** A button stores the panel: pressing it recalls it. MEMORY
  then a button stores the panel on that button, replacing what it held. Lamps as on the
  Genos: red = the button in use, blue = stored, dark = empty (and all flashing red while
  MEMORY waits for a button).
- **Banks** are files: all ten buttons and the bank's Registration Sequence in one file.
  REGIST BANK −/+ step through the bank files in the folder, in name order. Loading a
  bank recalls nothing; its buttons light up.
- **Memorize groups** (the Memory window's checkboxes) choose what a button stores; a
  recall changes only what the button stored.
- **Freeze** (the FREEZE button, and the ticked Freeze groups) leaves groups unchanged on
  recall.
- **Registration Sequence**: a programmed order of the bank's buttons, stepped with
  Regist +/−, with an end action (Stop, Top, Next bank). The steps and the end action are
  saved in the bank; Sequence On/Off is not (see Decisions).
- **Playlist**: a set list of records; a record loads a bank file (and recalls one of its
  buttons) or a style file. Up to 2,500 records; Normal / A→Z / Z→A display order; Up,
  Down and Delete are off while sorted, and saving keeps the displayed order.

## Files

yahaha's own JSON, never Yamaha's `.rgt`. By default in `~/Documents/yahaha`
(`--data-dir DIR` for the terminal UI; `Options::data_dir`; the app uses the default):

- `Registration/<bank>.regist.json`:

  ```json
  {
    "format": "yahaha.registration-bank",
    "version": 1,
    "name": "Friday Gig",
    "memories": [
      {
        "name": "Verse",
        "groups": ["style", "voice", "tempo", "transpose"],
        "sections": {
          "style": { "path": "/Users/me/Styles/SlowWalker.T552.sty", "name": "SlowWalker" },
          "tempo": { "bpm": 96.0 },
          "chord": { "fingering": "fingeredOnBass", "upper": false, "manualBass": true, "split": 54, "leftHold": true },
          "styleControl": { "main": 1, "intro": null, "syncStart": true, "syncStop": false, "stopAcmp": false, "stopAcmpMode": "off", "otsLink": false },
          "styleMixer": { "volumes": [100, 100, 96, 64, 76, 70, 88, 84], "on": [true, true, true, true, true, true, true, true], "set": [false, false, false, true, false, false, false, false] },
          "parts": { "parts": [
            { "on": true, "voice": { "kind": "gm", "program": 4, "bankMsb": 0, "bankLsb": 0 }, "volume": 100, "octave": 0 },
            null, null,
            { "on": false, "voice": { "kind": "gm", "program": 32, "bankMsb": 0, "bankLsb": 0 }, "volume": 90, "octave": 0 }
          ] },
          "transpose": { "keyboard": 0, "master": 0 }
        }
      },
      null, null, null, null, null, null, null, null, null
    ],
    "sequence": { "steps": [0, 1, 2, 1], "end": "next" }
  }
  ```

- `Playlists/<name>.playlist.json`: `{ "format": "yahaha.playlist", "version": 1,
  "name", "records": [{ "name", "kind": "bank", "path", "regist"? } | { "name", "kind":
  "style", "path" }] }`. A relative path is read from the playlist's own folder.
- `Registration/setup.json`: `{ "sequenceOn": bool }`, the Registration settings that
  are not part of any bank (the Genos keeps them in its Setup/Backup).

Saving under a name that another bank (or playlist) already has is refused, unless the
command says `overwrite: true` (the app shows an Overwrite button then). Names are
compared as the files they save to: `/`, `\`, `:` become `_` (so "A:B" is "A_B"), and on
a case-insensitive file system (the Mac's APFS) "gig" is "Gig". Saving your own bank or
playlist under its name in another case renames its file to that case.

A newer file (higher `version`) is refused; a section or group this build doesn't know is
skipped on recall but kept as it was when the bank is saved again (auto-save included), so
older builds read newer banks for what they understand and don't erase the rest.

## Registrables: how a feature joins Registration

Each feature stores its own **section** of a memory under its own key
(`src/session/registration/sections.rs`):

```rust
Registrable { key: "tempo", early: false, capture: tempo_capture, recall: tempo_recall }
```

- `capture(&Control, Groups) -> Option<Value>`: the section for a Memorize of those
  groups (None when none of its items are in them).
- `recall(&mut Control, &Value, Groups)`: puts it back, changing only items whose group
  is in `Groups` (what the button memorized, less the frozen groups).
- One line in `REGISTRABLES` (the recall order).

Today's sections: `style` (early), `multiPad` (early: the bank file, or null for none),
`tempo`, `chord` (fingering, Upper, Manual Bass, split), `styleControl` (Main, Intro, Sync
Start/Stop, Stop ACMP and its mode `stopAcmpMode` (Data List p.91: group Style; a bank without it
recalls only on/off), OTS Link), `styleMixer` (the 8 Style parts' CC7, on/off, and `set`:
which levels the player had set, and `level`: the Style volume, #199, the Genos's Style volume offset; a bank without it leaves it), `parts` (Right 1–3 and Left: on, voice, CC7, octave, `pan`/`reverb`/`chorus`/`variation` (CC10/91/93/94, #198/#204; a bank without them leaves them as they are), and the part's own sound library
patch `patch: { id, name }` (#109), recalled through `setPartPatch`),
`effects` (#204, group Style: each effect block's `effect` type, `returnLevel` and
`bandSend` (#236; a bank from before it recalls the defaults, reverb 100, chorus 0,
variation 0) and `params` (#236, `{ reverbTime: 24, ... }`; one absent is the type's own
value), under `reverb`, `chorus`, `variation`; a bank without it leaves them), `transpose`, `harmonyArp` (Keyboard Harmony/Arpeggio: the switch, the type and pattern by
name, Volume, Speed, Assign, Chord Note Only, Touch Limit, and the arpeggio's Quantize, Hold
setting, velocity and Keep Key On; not the Arpeggio Hold pedal function, which is the
pedal's), `styleSettings` (#107: Section Change Timing To Main, Style Retrigger on/off and
rate, Synchro Stop Window, Tap Tempo's Style Section Reset, all group Style; the Fade In,
Fade Out and Fade Out Hold times, group Assignable), `chordLooper` (#201, group Chord Looper:
`memory` 0–7 or null, `on` (ON/OFF: armed or looping), and the memory's `sequence` itself
(`bars`, `chords: [{ bar, at, root, type, bass?, name }]`) with its `name`, so a recall works
in a later session even when the looper's memories are gone). Live Control adds its own when
it is wired in (its group already exists).

The voice is a `VoiceRef` tagged by `kind` (`{"kind":"gm","program":…}`). A voice kind the
build can't read or play (a newer build's `kind`) is reported for that part only: its other
settings and the other parts still recall, and the bank file keeps the voice as it was.

**A part's plugin (#104).** A plugin picked on the Plugins tab (`setPartPlugin`) is stored as
`{"kind":"plugin","id":"aumu dls  appl","name":"DLSMusicDevice","state":"<base64>","program":4}`:
the plugin's id, its name (Regist Bank Info shows it), its full state (absent: its default
preset; a state over 64 MB is not stored, and Memorize says so), and the GM voice the part
has underneath. (session/registration/plugin.rs)
- **Memorize** stores the state the part saved last at once, then reads each part's playing
  plugin's state on a `plugin-state` thread (a plugin's state is never read on the control
  thread, #125) and puts it into the button when it lands, so the button keeps the plugin as
  it sounded at Memorize, not as the last 30 s autosave had it.
- **Recall** loads the plugin as `setPartPlugin` does: on a load thread, the part keeping
  what it plays until the plugin is ready; the part's library patch ends. A part that already
  plays that plugin with that state is left alone (nothing reloads). A plugin that isn't
  installed, or a build without the plugin host, leaves the part on the stored GM voice, and
  the recall says so.
- **Warm preload.** Selecting a bank (or changing one) preloads the plugin voices its
  buttons play, with their stored states, on `plugin-load` threads (session/plugin_pool.rs):
  each voice as many times as one button plays it, in button order, at most 8 instances. A
  recall then hands a preloaded instance to the rack at the next pump instead of loading;
  the pool refills after a recall, and lets go of what the bank no longer plays (instances
  are disposed of on `plugin-dispose`). Past 8, a button's plugin loads when pressed and the
  part shows Loading. A preload that fails isn't retried until the bank changes.
- A **GM voice** recalled on a part ends its Plugins-tab plugin. A plugin that the part's own
  library patch plays is not stored as a plugin voice: the `patch` is, and its recall brings
  the plugin back.

## Recall order and timing

1. The **style** (if it differs). Stopped, it loads at once; playing, it takes over at the
   next bar line, as every style change does (`Engine::change_style`).
2. Everything else waits while any style is still to come (`registration.pending`): this
   recall's, or one chosen just before it (a double tap, or two buttons with the same
   style pressed before the bar line), because a style load resets the tempo, the Style
   mixer and the section. Then, in `REGISTRABLES` order: tempo, chord settings, section
   and Style buttons, Style mixer, keyboard parts, transpose. With no style to wait for
   this all happens at once. A style the player chooses while the recall waits (another
   style, or a new load of the one playing) wins: the rest of the recall is dropped, so the
   registration's tempo, mixer and section never land on a style it wasn't meant for.
3. The section and the Style buttons go to the engine as **states**
   (`live::Cmd::StyleControls`), which it compares with its own: a Main change is a Main
   press (playing, it changes at the next bar line), and a switch already in the recalled
   state is left alone, however recently the control side last saw a snapshot. Style part
   volumes are states too (`StyleControls.volumes`, `player_set`): only the parts whose
   level the player had set when the button was memorized (`styleMixer.set`) are set, as
   a fader move (its CC7), and they hold that level against the patterns. Every other
   part goes back to the style (its own level, then its patterns' CC7: Intro, Main,
   Ending levels), even one the player moved since, as it would after a style load. The
   level a pattern had left on the mixer when the button was memorized (say an Ending's)
   is kept in the file but never recalled. A bank from an earlier build (no `set`) sets
   each stored level that differs.

Whatever a recall can't do (a style that's gone, a voice this build can't play) is in the
message, as an error, after the button's name; the rest is still recalled.

While a recall settles, **OTS Link holds still**: the registration's own voices win over
the OTS of the section or style it selects. The hold ends when the engine shows the
registration's style and Main (at most 8 s, in case the player picks another section
first).

## Group membership (Data List "Freeze Group" column)

| Group | Items here |
|---|---|
| Style | the style, section (Main, armed Intro), Sync Start/Stop, Stop ACMP, OTS Link, the Style part mixer and the Style volume, the **Left** part, split point, fingering, Chord Detection Area / Manual Bass, Left Hold, Section Change Timing To Main, Style Retrigger on/off and rate, Synchro Stop Window, Style Section Reset |
| Voice | Right 1–3: voice, on/off, volume, octave, pan, reverb and chorus sends |
| Tempo | the tempo, in whole BPM as on the Genos panel (recalled as SET TEMPO) |
| Transpose | Keyboard and Master transpose |
| Multi Pad | the Multi Pad bank (Data List "Multi Pad File"; a bank already chosen is left playing) and the Multi Pad volume (#196; a bank without it leaves it). Not the pads' Synchro Start standby |
| Assignable | the Fade In, Fade Out and Fade Out Hold times (Data List: Freeze group "Assignable Buttons") |
| Keyboard Harmony/Arpeggio | the `harmonyArp` section |
| Chord Looper | the Chord Looper's memory, its sequence and ON/OFF (a recall arms the loop, from the next bar line or with the style, or stops it at once; a recording under way is left alone) |
| Live Control | reserved |

Not stored (as on the Genos): Auto Fill In, the Style Change Behavior settings, OTS Link
Timing, the synth's master level, the fader page, the pad page, the selected part.

## Parameter Lock

Parameter Lock (RM p.163; `setParamLock`, `paramLocks`, `src/session/param_lock.rs`): a
locked group changes only from the panel. Registration, OTS and Playlist recalls leave it
alone. The groups are the Data List's Parameter Lock column, where yahaha has the items:

| Lock group | Items here |
|---|---|
| Split Point | the split point |
| Fingering Type | the fingering type, the Chord Detection Area (Upper) and Manual Bass |

The Genos's other groups (Master EQ, Reverb Type, the Reverb/Chorus/Variation Return
Levels, Vocal Harmony/Mic Setting) cover things yahaha doesn't have. A recall asks
`Control::param_locked(LockItem)` before it changes an item of a lock group; a new
registrable with such an item (a Left or Right 3 split point, say) must ask too. A One
Touch Setting has no item in any lock group (the Data List's OTS column), so an OTS recall
never needs to ask. The lock state is a setup setting: it is kept in `setup.json` beside
Sequence On/Off, never in a bank.

## Launchkey, keys, app

- **Pad page 4 (Registration)**: top row Regist 1–8; bottom row Regist 9, 10, Bank −,
  Bank +, Memory, Freeze, Regist −, Regist +. Lamps in the Genos colours; the other pads
  orange. **Shift + Track ◀/▶** = previous/next Playlist record.
- **Terminal**: Shift + `Q`…`P` = buttons 1–10, `F5` Memory, `F6` Freeze, `F7`/`F8`
  Regist −/+, `F11`/`F12` bank −/+, `<`/`>` playlist. A status line shows the bank, the
  ten lamps, Memory, Freeze, the sequence and the playlist. On macOS, F11 is Show Desktop
  by default: turn that shortcut off (System Settings › Keyboard › Keyboard Shortcuts ›
  Mission Control), or use the app's Registration bar / pad page 4 for Bank −.
- **Pedals** (Settings › Pedals, docs/controllers.md): Regist +/−, Registration Memory
  1–10, Registration Memory (MEMORY), Registration Bank +/−, Registration Freeze On/Off
  and Registration Sequence On/Off. A Regist +/− pedal steps the sequence while it is on
  and programmed, and otherwise the bank's stored buttons in order (`stepRegist`).
- **App**: the Registration bar under the keyboard strip (bank, the ten buttons with their
  names, Memory, Freeze, the sequence, the playlist) and the Registration panel (Bank,
  Memory & Freeze, Sequence, Playlist pages).

## Decisions (where the manuals are silent)

- **File format**: JSON with a `format`/`version` header and one section per feature.
  Because: readable, diffable, and each feature adds its section without a format change.
- **Saving**: a bank that has a file saves itself on every change (memorize, rename,
  clear, sequence); a new bank is kept in memory until it's saved with a name. Because: a
  performer shouldn't lose a registration to a forgotten Save; the Genos's ten buttons
  also survive power-off.
- **Start-up**: an empty, unsaved "New Bank"; Bank + goes to the first bank file. (The
  Genos keeps the last bank; remembering it is a follow-up.)
- **Default groups**: Memorize ticks every group; Freeze ticks none and is off.
- **Regist − at the first step** (not in the manual): Stop stays, Top wraps to the last
  step, Next goes to the previous bank's last step. Mirrors what + does at the end.
- **Recalling a button by hand** moves the sequence cursor to that button's next
  occurrence, so Regist + carries on from there.
- **Sequence steps while it's off** are refused with a message (the Genos needs it On).
- **A Regist +/− pedal with no sequence** (off, or no steps) steps through the bank's
  stored buttons in order, skipping empty ones, and stops at the first and last (from no
  button: + recalls the first, − the last). Because: the Genos drives a pedal through the
  sequence (RM p.114, Pedal Control), and Genos firmware 1.40 starts a new sequence as
  1–10 in order (a video, not the manual), so a pedal steps a song with no programming;
  skipping empty buttons keeps a press from doing nothing on stage (#200).
- **Sequence On/Off** is a panel setting, not part of the bank: it stays when the bank
  changes (so End = Next runs through every bank), and is kept in `setup.json`. Because:
  the Genos Data List (Regist Sequence) marks Sequence Data and Sequence End as
  Registration items and Sequence On/Off as not (Setup/Backup only). An `on` field in a
  bank written by an earlier build is ignored.
- **Frozen tempo across a style change**: the tempo is put back after the style load
  (a stopped load takes the style's own tempo). A button that didn't memorize Tempo lets
  the style load set it, as any style change does.
- **Style part volumes**: stored per part (CC7), besides the Style volume (`level`, #199:
  one scale on all of them, the Genos's whole-Style offset); the Genos stores the offset ("Volume(Style) Offset" in the Data List's Registration
  items). Decision: only the levels the player set are recalled, because that is what the
  Genos's offset covers: an unset offset leaves the pattern levels alone. Only the synth
  master is a non-CC gain, and it's not stored.
- **Sync Start** is recalled only while stopped (pressing it while playing stops the band).
- **Missing style**: if the saved path is gone, a library file of the same name is used;
  otherwise the rest of the registration is still recalled and the message says so.
- **Playlist style records**: yahaha records may point straight at a style file (the
  Genos goes through a bank); handy for a set list of styles.
- **Style settings** (#107), from the Data List's Registration column: Style Retrigger
  On/Off and Rate, Synchro Stop Window and Tap Tempo's Style Section Reset are group Style;
  Fade In, Fade Out and Fade Out Hold Time are group "Assignable Buttons", which yahaha adds
  as the `assignable` group (the Assignable buttons' own functions can join it later). The
  Data List (Genos) has no Section Change Timing; the Genos2 Reference Manual (p.12) says
  To Main "is also set when you load a Registration Memory", so To Main is stored (group
  Style) and Inside Intro/Ending, which it doesn't mention, is not. The recalled To Main
  applies to the registration's own style: the rest of a recall waits for it anyway.
  Retrigger on/off is sent as a state (`StyleControls.retrigger`). A bank from an
  earlier build (no `styleSettings`) leaves the settings as they are.
- **A part's library patch** (#109): stored with its GM voice underneath. A recall sets
  the GM voice, then the patch (skipped if the part already plays it), then the stored
  level, octave, pan and sends, which win over the patch's defaults.
  A patch deleted from the library leaves the part on the GM voice, and the message says
  so. A memory without a patch (a GM voice, or a bank from an earlier build) clears the
  part's own patch, since the GM voice is what it stored.
- **Launchkey**: a fourth pad page rather than a Shift layer on page 1, so the lamps can
  show which buttons are stored and which is in use. Shift + Track was free and is where a
  set list's "next song" belongs.
