# Racks and sounds

The design for "sounds and racks the owner can actually work with". The owner's
wireframe (v5, private artifact) is the reference for anything on screen. This file is
the reference for the model, the files and the order of work. Where the two differ, the
difference is listed under "Deviations from the wireframe".

## Words

- **Rack**: what's under your hands. It holds the four keyboard parts (Right 1–3 and
  Left), each with its sound and mix, plus the split, Harmony/Arp, transpose and the
  controller map. A rack is saved, recalled and swapped as a whole.
- **Sound**: the raw instrument. It's a SoundFont preset or a plugin state, with a name,
  a category, tags and a favourite flag. It has no mix settings.
- **Quick Racks**: the one-press buttons (bar, Launchkey pad page 4, pedals). They
  replace Registrations. Banks A–H, 8 buttons each.
- **Library**: the one browser. It's a top-level page next to Stage, with Racks, Sounds,
  Instruments and Style map tabs.
- **Live rack**: the rack that's loaded now, including unsaved changes. It autosaves and
  comes back on boot.

In code the model lives in `src/racks/` as `racks::Rack`. The existing `synth::Rack` and
`plugin::PluginRack` keep their names; they are audio plumbing, not user data.

## The model

```
Rack {
  name, id,
  parts: [RackPart; 4],            // R1, R2, R3, L
  split: u8,                       // left split point
  harmony_arp: HarmonyArpReg,      // the same fields Registrations store today
  transpose: i8,
  controls: ControlMap,            // faders 1–4, knobs 1–8 on the Rack knob page
}
RackPart {
  on, sound: SoundRef,             // what plays
  volume, pan, reverb, chorus, variation, octave,
  tone: ToneReg, bend_range,       // as PartReg stores today
  eq: PartEq,                      // channel-strip EQ (#247); absent = flat, flat not written
}
SoundRef = Library(id) | Font { file, bank, program }  // GM voices are font presets
```

- A rack names each sound by reference. Saving a sound you own updates every rack that
  uses it, because they point at it.
- A part's **sound edit** (a plugin state that differs from the saved sound) is held in
  the live rack as `edited_state` until it is saved. It is never written into someone
  else's sound.
- Mix belongs to the rack part, never to the sound. `PatchDefaults` goes away (see
  Migration).
- Style parts, Multi Pads, tempo and the style itself aren't in a rack. The setlist
  (later) holds those.

## Files

The user's things live in the data folder (`~/Documents/yahaha` by default). The live
rack is app state, so it sits next to today's `plugin-parts.json` in
`~/Library/Application Support/yahaha`.

| File | Holds |
|---|---|
| `<data>/Racks/<name>.rack.json` | One user rack, format `yahaha.rack`, version 1. |
| `<data>/quick-racks.json` | Banks A–H of 8 buttons, each a rack id or empty. |
| `<data>/style-racks.json` | Per style file name: which of OTS 1–4 load a user rack instead. |
| `<data>/sound-library.json` | Sounds (version 3: no defaults), the GM maps. |
| `<data>/known-plugins.json` | The plugins yahaha has seen installed (id, name, vendor, whether still new), format `yahaha.known-plugins`, version 1: what makes a plugin New or Missing. |
| `<app support>/live-rack.json` | The live rack, with unsaved edits and plugin states. Replaces `plugin-parts.json` in the same folder. |

Every write is atomic. A file with a newer version than we know is refused and never
saved over (the rule the sound library and Registration banks already follow).

## Saving

- **Modified** shows as soon as anything in the live rack differs from its saved rack:
  a mix change, a sound change, or a plugin edit. Plugin edits are detected within about
  a second while the plugin window is open, not at the 30-second autosave.
- **Save rack** overwrites your own rack and saves any edited sounds in the same step:
  your own sounds are updated in place; an edited factory, file or SoundFont preset asks
  for a name and becomes a new sound of yours. **Save as…** makes a new rack.
- **Style racks and factory sounds are never overwritten.** Saving one makes a copy.
- **One save makes one record.** Picking a factory preset doesn't add a library record;
  the library lists factory presets from the plugin itself. A record is written only when
  you save.
- **Unsaved work is never lost silently.** The live rack autosaves (so quitting keeps it),
  and switching racks with unsaved changes asks: Save first, Discard and switch, or Keep
  editing. This applies from the bar, the Library, the Launchkey and pedals. From the
  hardware, where there's no dialog, the switch goes ahead and the unsaved rack is kept
  as "Recovered: <name>" in My racks, so nothing is lost.

## Styles and OTS

- Loading a style never changes the rack. OTS Link stays off by default; with it on, a
  Main change recalls that OTS, as today.
- The style's four OTS show as style racks (read-only). For each style you can point any
  OTS button at one of your racks instead. That choice lives in `style-racks.json`; the
  `.sty` file isn't touched. "Style's own" puts it back.

## Plugins coming and going

- The startup scan (or Settings › Plugins › Rescan) marks newly found plugins **New** in
  Library › Instruments until they're first opened.
- A plugin that's gone is **Missing**. Racks and sounds that use it are kept untouched.
  Parts that use it are silent and marked ⚠ on the stage and in the rack; the Racks tab
  has a "Needs attention" filter. **Replace…** swaps the sound and keeps the part's mix.
  Nothing is rewritten until you save; reinstalling the plugin brings the original back.

## Screens (the wireframe, in today's app)

- Header: a **Stage | Library** switch. Alt+B toggles it (it opened Sounds before).
- **Library** replaces the Sounds modal and the Sound Library drawer. Browser on the
  left, the Rack panel docked on the right, Quick Racks along the bottom. Clicking a
  sound loads it at once on the target part; ↑ ↓ steps. Badges: Mine, Factory, SoundFont,
  Style. The GM map moves to a **Style map** tab (see Deviations).
- **Rack panel** replaces Parts & OTS: a right-hand drawer on Stage (same button, Alt+O)
  and docked in Library.
- **Quick Racks bar** replaces the Registration bar in the same place: bank ◀ ▶, 1–8,
  Store. Freeze goes. Pad page 4 is renamed Quick Racks. Top row: Quick Racks 1–8
  (were Snapshots 1–8). Bottom row: bank − / + (were snapshot bank − / +), Store, and
  previous / next rack in the bank (were Regist Seq − / +). The bank-file and Freeze
  pads go dark, because there are no bank files and nothing to freeze.
- The Launchkey mirror's part faders show their sound's name; clicking it opens Library
  on that part. The Parts knob page becomes the **Rack** page, driven by the rack's
  controller map.
- Mixer, Effects, Harmony/Arp, Multi Pads, Looper, Charts and Settings are unchanged;
  what they change on keyboard parts is saved with the rack.

## Migration

- **Sounds are kept.** `sound-library.json` v2 → v3 drops each sound's `defaults`. The
  v2 file is copied to `sound-library.v2.json` first, so nothing is discarded.
- **Style part levels don't change.** Today a sound's `defaults.volume` sets a Style
  part's CC7 when the style sets none. That level moves onto the GM map rules: the
  migration copies it onto every map rule (global or per style) that names the sound, and
  a rule's level is used exactly where the sound's default was. Pan, reverb, chorus and
  octave defaults only ever applied to keyboard parts; those parts' current values go
  into the "Restored" rack.
- **The live setup is kept.** On the first boot, `plugin-parts.json` and the current
  parts become the live rack, named "Restored". The old file stays.
- **Registration banks are dropped**, as the goal allows. The files stay on disk,
  unread. They aren't imported. The Playlist and Registration Sequence go with them
  (the setlist replaces them later).

## Deviations from the wireframe

- **Per-part inserts** (the "Inserts" row on each rack part) are left out. Keyboard parts
  have no insert effects in the engine today; the rack saves whatever Effects sets.
- **The GM map** had no home in the wireframe. It's a fourth Library tab, Style map,
  because Library replaces the drawer it lived in.
- **Plugin macros in the controller map** ("learn" a plugin parameter) come last and may
  ship after the journeys if AU parameter learn doesn't fit the sprint.
- The bar's **Sequence** and **Playlist** go, because they stepped through Registrations.
- Pad page 4 has no **FILE − / +** pads: Quick Racks banks live in one file.

## Order of work

Each item is one PR into `develop`.

1. Done (#369). Sounds dialog bugs, on today's code: real preset lists and counts;
   preset listing never hangs; the playing sound is named; one save makes one record;
   edits show as modified at once.
2. Done (#366). `racks::Rack`: the model, its file, capture from and apply to the session.
3. Done (#378). Sound = raw instrument: sound library v3, no defaults.
4. Done (#377). Live rack: autosave and boot restore (replaces `plugin-parts.json`),
   modified state.
5. Done (#381). Rack commands: new, load, save, save as, revert, rename, duplicate,
   delete, with the switching guard.
6. Done (#383). Quick Racks replace Registrations (bar, pads, pedals, Store, banks).
7. Done (#390). Style racks: per-style OTS overrides.
8. Done (#376). Missing and new plugins.
9. Done (#379). Library page shell and the Stage | Library switch; Sounds tab.
10. Done (#379). Library Racks and Instruments tabs; Style map tab.
11. Done (#380). Rack panel (drawer and dock), replacing Parts & OTS.
12. In part (#380; the Rack knob page and controller map in #392). Stage: sound names on
    part faders, the Rack knob page, controller map.
13. Done (this PR). Remove the old Sounds modal, Sound Library drawer and Registration
    code. What is left of the Sounds modal is the map rule's sound picker.
