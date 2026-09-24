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
          "chord": { "fingering": "fingeredOnBass", "upper": false, "manualBass": true, "split": 54 },
          "styleControl": { "main": 1, "intro": null, "syncStart": true, "syncStop": false, "stopAcmp": false, "otsLink": false },
          "styleMixer": { "volumes": [100, 100, 96, 80, 76, 70, 88, 84], "on": [true, true, true, true, true, true, true, true] },
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

Today's sections: `style` (early), `tempo`, `chord` (fingering, Upper, Manual Bass, split),
`styleControl` (Main, Intro, Sync Start/Stop, Stop ACMP, OTS Link), `styleMixer` (the 8
Style parts' CC7 and on/off), `parts` (Right 1–3 and Left: on, voice, CC7, octave),
`transpose`. Keyboard Harmony/Arpeggio (#32/#33), Multi Pads (#37), the Chord Looper and
Live Control add theirs when they're wired in (their groups already exist).

The voice is a `VoiceRef` tagged by `kind` (`{"kind":"gm","program":…}`), so plugin
instruments (#35 phase 2) become a new kind without breaking old banks. A voice kind the
build can't read or play (a newer build's `kind`) is reported for that part only: its other
settings and the other parts still recall, and the bank file keeps the voice as it was.

## Recall order and timing

1. The **style** (if it differs). Stopped, it loads at once; playing, it takes over at the
   next bar line, as every style change does (`Engine::change_style`).
2. Everything else waits while any style is still to come (`registration.pending`): this
   recall's, or one chosen just before it (a double tap, or two buttons with the same
   style pressed before the bar line), because a style load resets the tempo, the Style
   mixer and the section. Then, in `REGISTRABLES` order: tempo, chord settings, section
   and Style buttons, Style mixer, keyboard parts, transpose. With no style to wait for
   this all happens at once.
3. The section and the Style buttons go to the engine as **states**
   (`live::Cmd::StyleControls`), which it compares with its own: a Main change is a Main
   press (playing, it changes at the next bar line), and a switch already in the recalled
   state is left alone, however recently the control side last saw a snapshot. Style part
   volumes are states too (`StyleControls.volumes`): only a part whose level differs is
   set, as a fader move (its CC7). A part already at its stored level stays the style's,
   so the style's own pattern CC7 (Intro, Main, Ending levels) still moves it, as it
   would without a recall.

Whatever a recall can't do (a style that's gone, a voice this build can't play) is in the
message, as an error, after the button's name; the rest is still recalled.

While a recall settles, **OTS Link holds still**: the registration's own voices win over
the OTS of the section or style it selects. The hold ends when the engine shows the
registration's style and Main (at most 8 s, in case the player picks another section
first).

## Group membership (Data List "Freeze Group" column)

| Group | Items here |
|---|---|
| Style | the style, section (Main, armed Intro), Sync Start/Stop, Stop ACMP, OTS Link, the Style part mixer, the **Left** part, split point, fingering, Chord Detection Area / Manual Bass |
| Voice | Right 1–3: voice, on/off, volume, octave |
| Tempo | the tempo |
| Transpose | Keyboard and Master transpose |
| Keyboard Harmony/Arpeggio, Multi Pad, Chord Looper, Live Control | reserved for those features |

Not stored (as on the Genos): Auto Fill In, the Style Change Behavior settings, OTS Link
Timing, the synth's master level, the fader page, the pad page, the selected part.

## Parameter Lock

Recall asks `Control::param_locked(LockItem)` before changing a lockable item. Today:
`SplitPoint` (the split) and `FingeringType` (fingering, Upper/Lower, Manual Bass), as the
Data List's lock groups; Parameter Lock itself (M5) owns that function and its state, and
nothing is locked until it lands.

## Launchkey, keys, app

- **Pad page 4 (Registration)**: top row Regist 1–8; bottom row Regist 9, 10, Bank −,
  Bank +, Memory, Freeze, Regist −, Regist +. Lamps in the Genos colours; the other pads
  orange. **Shift + Track ◀/▶** = previous/next Playlist record.
- **Terminal**: Shift + `Q`…`P` = buttons 1–10, `F5` Memory, `F6` Freeze, `F7`/`F8`
  Regist −/+, `F11`/`F12` bank −/+, `<`/`>` playlist. A status line shows the bank, the
  ten lamps, Memory, Freeze, the sequence and the playlist. On macOS, F11 is Show Desktop
  by default: turn that shortcut off (System Settings › Keyboard › Keyboard Shortcuts ›
  Mission Control), or use the app's Registration bar / pad page 4 for Bank −.
- **App**: the Registration bar under the app bar (bank, the ten buttons with their
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
- **Sequence On/Off** is a panel setting, not part of the bank: it stays when the bank
  changes (so End = Next runs through every bank), and is kept in `setup.json`. Because:
  the Genos Data List (Regist Sequence) marks Sequence Data and Sequence End as
  Registration items and Sequence On/Off as not (Setup/Backup only). An `on` field in a
  bank written by an earlier build is ignored.
- **Frozen tempo across a style change**: the tempo is put back after the style load
  (a stopped load takes the style's own tempo). A button that didn't memorize Tempo lets
  the style load set it, as any style change does.
- **Style part volumes**: stored per part (CC7), since yahaha has no Style volume offset;
  the Genos stores the offset. Only the synth master is a non-CC gain, and it's not stored.
- **Sync Start** is recalled only while stopped (pressing it while playing stops the band).
- **Missing style**: if the saved path is gone, a library file of the same name is used;
  otherwise the rest of the registration is still recalled and the message says so.
- **Playlist style records**: yahaha records may point straight at a style file (the
  Genos goes through a bank); handy for a set list of styles.
- **Launchkey**: a fourth pad page rather than a Shift layer on page 1, so the lamps can
  show which buttons are stored and which is in use. Shift + Track was free and is where a
  set list's "next song" belongs.
