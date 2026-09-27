# Sound Browser: the Sound model and the GM map

The Sound Browser rethink. This page is the model the whole sprint builds on: what a
Sound is, how things name one, the GM map's layers, and how old files migrate. The
owner's decisions D1–D6 are binding and listed at the end.

- Code: `src/patches/sound.rs` (the model), `src/patches/gm.rs` (the map's layers,
  auto-fill and provenance), `src/patches/store.rs` (the library file and its
  migration), `src/session/sound_library.rs` (`capture_sound_state`,
  `link_voice_sound`, `sound_tag_for_state`).
- Related: docs/sound-library.md (the library and program map as first built),
  docs/plugin-hosting.md ("AU presets").

Status: the model, migrations, first-play capture and the map's data shape are in (PR 1).
The session still resolves through the default sound set; the map with auto-fill
replaces it next (PR 3). No UI uses any of this yet.

## Sounds

A part plays a **Sound**, which is one of two things.

- **Font preset**: a preset of a SoundFont in a scanned folder: its file name, SoundFont
  bank (128 = drum kits) and program (`FontPreset`). It needs no library entry.
- **Plugin sound**: a library patch whose source is a plugin
  (`PatchSource::Plugin { component_id, state, origin }`). It holds the instrument, its
  captured state, and the patch's name, category, CC defaults and favourite flag.

There is exactly **one** kind of plugin sound. Where it came from is its origin
(`PluginOrigin`):

| origin | what | found again by |
|---|---|---|
| `user` (or absent) | made in yahaha (Save as…, New sound), or any sound from a library written before origins | never: two sounds made in yahaha are two sounds |
| `factory { number }` | the plugin's factory preset | instrument + number |
| `file { path }` | an `.aupreset` imported at a scan | instrument + path |

The origin never changes what plays; the state does. It stops a scan or a second pick
of the same preset from adding a duplicate (`SoundLibrary::plugin_sound`,
`add_plugin_preset`).

A library patch whose source is a SoundFont preset is a font preset the user added to
their sounds, with a name, category and defaults of its own.

### First-play capture

A factory preset's state is only known once an instance plays it. Its sound starts
with an empty state (`Patch::awaits_capture`) and loads by number
(`kAudioUnitProperty_PresentPreset`). When the load finishes, the channel's state is read
off the control thread, as it always was for a preset picked by number. The pump hands the
state to `capture_sound_state`, which stores it in the sound once
(`SoundLibrary::capture_state`) and saves the library. Parts already playing the sound
keep their instance, because their voice is updated in place.

A later read never overwrites a captured state. Edits are the user's to save
(Save / Save as…, PR 4).

## Ids

| id | Sound |
|---|---|
| `sf:<file>:<bank>:<program>` | a font preset |
| `saved:<patch id>` | a library patch (every plugin sound, and font presets in My Sounds) |

`SoundId::parse` reads both forms. A bare patch id, which is what map rules and part
patches have always stored, reads as `saved:`. The browser's plugin rows
(`au:<component>`, `au:<component>#f:<n>`, `au:<component>#u:<path>`) are not Sounds.
They are instruments and their presets, and picking one gives you its Sound.

Records that name a Sound keep a `SoundTag { id, name }`: the id, plus the name it had
when stored, so recall can show the name even if the sound has since been deleted. It
appears in:

- `PluginVoice.sound`: a part's plugin voice, as plugin-parts.json saves it.
- `VoiceRef::Plugin.sound`: a Registration's (and an OTS's) plugin voice. PR 2 fills it
  at Memorize.

## The GM map

Each of the 128 GM programs, and the drum parts, resolves through these layers, most
specific first (`resolve_gm`, `Layer`):

1. **Drums**: the drum rule (drum parts only).
2. **Override**: a per-program rule.
3. **Family**: a rule for the program's GM family (16 families of 8).
4. **Auto**: `AutoFill`, the best-matching preset in the scanned fonts (D4).
5. **None**: nothing covers it.

Rules name library patches, so any plugin sound or font preset in the library can be a
rule. Auto names a font preset directly.

A style's map is stored in the library under the style's file name. Its rules win over
the global rules, and anything it leaves unset falls through to the global map, exactly as
`patches::resolve` has always done. Auto comes after both.

**Auto-fill** (`AutoFill::build`) sorts the fonts by GM completeness (`gm_completeness`:
how many GM programs they have on bank 0, then whether they have a kit), breaking ties by
file name. Then:

- Each program takes the best font's bank-0 preset. Failing that, it takes the first
  font that has the program on any melodic bank (the lowest bank).
- The drums take the best font's kit 0. Failing that, they take its lowest kit.

**Provenance (D6).** Every `GmResolution` that ends in a font records its `FontPreset`
(file, bank, program). That covers a rule naming a SoundFont patch as well as an auto
fill, so a `.sf2` writer can render the map later without the data changing. A plugin
sound has none.

**The map page's data** is `gm_map_rows`: 129 rows, the drums first, then programs 0–127.
Each row has its family, the override and family rules that apply (the style's own where
it has one), and the resolution with its deciding layer.

Everything here runs off the real-time threads. The map still resolves into the audio
thread's route table off-thread (O8).

## Migration

Each migration has a test in `src/patches/sound_tests.rs`.

- **Library file.** Format 2 adds `origin` to plugin sources. A format 1 file reads
  unchanged: its plugin patches are `user` sounds, and its family, override and drum
  rules, global and per-style, stay as they were. The file is written back as format 2,
  so a format-1 build refuses it rather than saving over it and dropping the origins.
  The patch limit rises from 256 to 4,096, because every plugin sound lives here now.
- **plugin-parts.json.** A part voice saved with a preset key (`f:`/`u:`) and no `sound`
  loads as before. When it plays, `link_voice_sound` gives it the library's sound for
  that preset (added once), and a factory preset's state is then captured into it.
- **Registrations.** A plugin voice with no `sound` loads as before. On recall it gets
  the library sound with exactly that instrument and state, if there is one
  (`tag_for_state`). Otherwise it plays with no sound (PR 2 shows it as edited).
- **OTS.** OTS voices are GM voices: the record is unchanged, and they resolve through
  the map as before.

## Decisions (binding, from the owner)

- **D1.** The library is the canonical store. `sound-library.json` holds every plugin
  sound (state, name, category, CC defaults, favourite). `.aupreset` files are imported
  on scan and remembered by path; they are not the store.
- **D2.** The browser has two tabs, Sounds and Instruments. Instruments lists SoundFont
  files and plugins, and SoundFonts get special handling only where they really differ.
- **D3.** There is no default sound set. There are only scanned folders, the library's
  metadata and the GM map, whose layers are listed above. Every rule points at any Sound.
  Style parts, keyboard GM voices and OTS voices all resolve through the map, and a
  per-style map falls through to the global one.
- **D4.** A program nobody assigned auto-fills from the best-matching preset in the
  scanned fonts, most GM-complete font first. The map page shows each program's sound
  and the layer that decided it.
- **D5.** Export and import: a plugin sound as `.aupreset`, and the library or map as a
  bundle (metadata JSON plus every plugin sound's state; fonts referenced by file name).
- **D6.** Rendering a real `.sf2` from the map is out of scope, but every font
  resolution records file, bank and program so a writer can be added later.

## Decisions made in this PR

- **Decision: the origin lives on the plugin source**, not on the patch, because it
  only means something for a plugin. A `user` origin is left out of the JSON, so
  libraries without origins look the same as before.
- **Decision: a style's rules still beat every global rule.** D3's "layer by layer" is
  read as the existing fall-through (style, then global), with auto after both. This
  changes no existing map, which the migration requires.
- **Decision: rules still name library patches.** A font preset named by a rule is added
  to the library once, as before. Auto is the only layer that names a font preset
  directly, so rules keep one id space and old maps need no rewrite.
- **Decision: capture happens once.** A factory sound's state is filled only while it
  is empty. Every later change is an edit, saved deliberately.
- **Decision: a recalled state with no sound matches only an identical state.** A
  near match is not the same sound. It plays unnamed, and PR 2 shows it as edited.
- **Decision: a factory preset can now be a map rule.** Its sound is added with an
  empty state, and the state is captured when the rule first plays it. The old refusal
  ("play it on a part and Save as sound") is gone, in the session and in both mocks.
