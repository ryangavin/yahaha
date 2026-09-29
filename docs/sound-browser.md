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
Now playing, the "edited" check, Save / Save as… and the sound in Registrations are in
(PR 2). The session and the synth resolve every program through the map with auto-fill,
and the default sound set is gone (PR 3, see "Resolution in the session"). The map's rows
are in the state (`soundLibrary.gmMap`) for the map page. The Sounds tab shows all this
(PR 4, see "The Sounds tab").

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
They are instruments and their presets. A part playing a picked preset names it by its
catalog id in its `sound` tag; picking adds no library record, and Save makes one
(docs/racks.md "Saving": one save makes one record).

Records that name a Sound keep a `SoundTag { id, name }`: the id, plus the name it had
when stored, so recall can show the name even if the sound has since been deleted. It
appears in:

- `PluginVoice.sound`: a part's plugin voice, as plugin-parts.json saves it.
- `VoiceRef::Plugin.sound`: a Registration's (a Snapshot's) plugin voice, filled at
  Memorize, even when the sound is edited (the state stored is the edit, and recall
  shows it edited). Regist Bank Info shows the sound's name.
- `keyboardParts[i].sound`: what the part plays now (O3).

## Now playing and "edited"

A keyboard part's state names the Sound that actually sounds (`KeyboardPart.sound` and
`voiceName`, `api::part_sound`, whose web twin is `app/src/lib/api/part-sound.ts`): a
plugin that is loading, playing or muted names the preset it was given, else its library
sound (by the library's name now, so a rename or delete shows at once), else the bare
plugin; a part with no plugin, or one whose plugin failed, names its SoundFont voice: its
own SoundFont patch, else what the GM map (auto-fill included) resolves its program to.

**Edited** (`KeyboardPart.sound_edited`, `ChannelPlugin::edited`) means the plugin's
state no longer matches the sound it was loaded from:

- The baseline is the plugin's own state read right after it loads (a `plugin-state`
  read, as the factory-preset read always was), not the stored bytes: a plugin may
  serialize the same sound differently from how it was stored.
- Every later read (the 30-second autosave, an explicit save, a Memorize's fill) is
  compared by fingerprint (`state_fingerprint`, hashed on the `plugin-state` thread that
  read it). One that differs marks the part edited. Nothing is added to the audio, engine
  or MIDI threads.
- A voice whose state is not its sound's when it is assigned (a Registration memorized
  with an edit, a plugin-parts.json autosaved after one) is edited from the start: one
  string compare against the library on the control thread.
- The mark stays until Save, Save as… or another sound.

**Save** (`saveSound`) writes the part's state (read afresh), volume and octave over the
sound, but only over the user's own sound. A factory preset, an `.aupreset` file, a
sound of another plugin or no sound at all goes to **Save as…** (`saveSoundAs`), which
adds a new `user` sound that the part then plays. A part playing a library plugin patch
keeps its instance through both (`refresh_part_plugin_voices`). `savePartAsPatch` is
kept as Save as… for older clients; `.aupreset` export (`savePartAsPluginPreset`, with
#307's overwrite confirmation) is unchanged.

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

### Resolution in the session

- **The auto-fill** is built on the control side from the SoundFont folder's fonts (their
  preset lists only) at start and whenever the folder's files change
  (`session/gm_auto.rs`).
- **The synth's main font** is the most GM-complete font (`best_font`: the auto-fill's
  first choice). It is not a setting. It plays any channel that has no route. When the
  folder changes and another font becomes the best, it loads in the background and swaps
  in between two audio buffers.
- **Routes.** `SoundLib::write_bank` resolves the drums and all 128 programs with
  `resolve_gm` for the style's bank of the table (`patches::Routes`). A rule's patch
  routes as before. An auto-fill from another font routes to that font's preset (the
  font joins the rack). An auto-fill from the main font needs no route: the main font
  plays the style's own bank and program, so Yamaha/XG bank variations sound as they
  always did. Style parts, keyboard parts' GM voices and OTS voices (which are GM voices
  on the keyboard parts) all read this table. Nothing new runs on the audio, engine or
  MIDI threads.
- **The map page's data** is `soundLibrary.gmMap` (`gm_map_rows` for the style playing),
  in the engine and in both mocks.
- **Removed:** `setSoundFont`, `setDefaultSoundSet`, `io.defaultSoundSet`,
  `io.autoSoundSet` and Settings › Audio "Default sound set". A `defaultSoundSet` key left
  in `sound-settings.json` is ignored. `--sf2` (and the app's `YAHAHA_SF2`) stays as a
  hidden compatibility pin: that font is the main font and fills every program it has at
  the auto layer (other fonts fill only its gaps); the map's rules still come first.

### The map page (PR 6)

The Sound Library drawer's **GM map** tab (`app/src/panels/sound/GmMapPage.svelte`)
replaces its old Program Map and This style tabs. It draws `soundLibrary.gmMap`:

- The drums, then the 16 families. Each family's header has its rule; each program row
  has its override, the sound it resolves to (the small line), and a badge for the layer
  that decided it (Drums, Override, Family, Auto or Unset). Auto rows and badges are
  tinted, and the tab counts the auto slots, so what nobody chose stands out.
- **Global / This style** switches which map the pickers edit. In the style's map an
  unset rule shows what it falls through to (↳). The resolved sound and its badge are
  always what plays now, for the style playing, so a style's own rule is marked "style"
  in either view.
- Rows a style part uses name the part (what the old This style tab showed).
- Every rule is set by picking from Sounds (the Sound Browser's pick mode, so every
  plugin sound and font preset is eligible) and cleared with ✕. No new commands:
  `setDrumRule`, `setFamilyRule`, `setProgramOverride`, `clearStyleMap`.
- Every control is a native button with a tooltip, reached with Tab; the drawer's tab
  strip takes the arrow keys.

## The Sounds tab

The Sound Browser overlay's list (`app/src/panels/sounds/`). It is all client-side: the
wire is unchanged.

- **All sounds** is the GM map's resolved sounds (one row per program and the kit,
  whichever layer decided it; the row's detail says "GM 5 · …"), every plugin sound and
  everything in My Sounds (the library's patches). About 150 rows, not 1,200.
- **Chips:** All sounds, Favourites, Recent, My Sounds, one per category (within All
  sounds), and **one per instrument**: a SoundFont chip lists every preset of that font; a
  plugin chip lists the plugin, its factory presets (asked for with `listPluginPresets`
  when the chip opens), its `.aupreset` files and the library sounds made with it. The
  filter searches the chip's rows.
- **What plays:** ▶ marks the part's `sound`. When that row isn't in the list, no row is
  active until you move or type a filter (which moves to its first match). The footer reads "<Part> plays <instrument> · <sound>", with an
  **edited** badge while `soundEdited`.
- **One Save flow:** **Save** (`saveSound`) and **Save as…**, which asks for a name
  (`saveSoundAs`) and, on a plugin part, can also write an `.aupreset` with a category
  (`savePartAsPluginPreset`, asking before it replaces a file of that name). The new sound
  shows selected in My Sounds.
- **Your sounds:** selecting a library sound shows its strip under the list: rename,
  category, Details (tags and the defaults a part takes), Duplicate and Delete… (asked
  first). A plugin's category is set on the Instruments tab. A library sound made with a
  plugin can also be exported as an `.aupreset` from the strip (`exportSoundPreset`).
- **Keys** (the filter keeps focus): ↑/↓ PgUp/PgDn Home/End, Enter plays, Shift+Enter
  auditions, Ctrl/⌘+D stars, Ctrl/⌘+S saves, Ctrl/⌘+Shift+S saves as…, F2 renames,
  Ctrl/⌘+Delete deletes (asked), Esc closes. Tab reaches every chip and button.
- The Sound Library drawer's **Patches** tab is gone: its list is My Sounds, its editor
  is the strip, "Save a part's sound" is Save as…, and the library file moved under the
  drawer's other pages. Plugin housekeeping (rescan, retry, hide) lives on the
  Instruments tab, not in this tab's footer.

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
  (`tag_for_state`). Otherwise it plays with no sound: the part shows no sound name
  (an unnamed state), not "edited", since there is no sound it was edited from.
- **OTS.** OTS voices are GM voices from the style file: there is no plugin state or
  sound to store, the record is unchanged, and they resolve through the map as before.

## Export and import (D5, PR 7)

- **A plugin sound as `.aupreset`** (`exportSoundPreset`): its state is written with
  `presets::write_user_preset`, the writer Save as preset uses, into
  `~/Library/Audio/Presets/<Manufacturer>/<Plugin>/<name>.aupreset`, where Logic reads it.
  An existing preset of that name is replaced only with `overwrite` (#307); the app asks
  Replace/Cancel. A factory sound that has not played yet has no state to export.
- **The library and GM map as a bundle** (`exportSoundLibrary`):
  `{ "kind": "yahaha-sound-bundle", "fonts": [...], "library": {...} }`. The library holds
  every sound's metadata and every plugin sound's state, and the global and per-style
  maps. SoundFonts are listed by file name, never copied.
- **Import** (`importSoundLibrary`) reads a bundle or a plain library file. It merges by
  default: sounds are added under new ids where they clash, and with `maps` the map rules
  come too. `replace` swaps the whole library. SoundFonts resolve by file name in the
  SoundFont folder. Any that are missing are reported by name, and their sounds are kept
  so they play once the file is there.
- Rendering a `.sf2` from the map (D6) is issue #322.
## The Instruments tab (PR 5, O2)

The Sound Browser's second tab (`app/src/panels/sounds/Instruments.svelte`, model in
`instruments.ts`). It lists what the scanned folders hold: each SoundFont, then each
instrument plugin. Every row opens (▸/▾, Enter or Space) to its own preset list.

- **A font row** shows only its preset and kit counts and how GM-complete it is
  (`SoundCatalog.fonts`, from `patches::gm::gm_completeness`: GM programs on bank 0, of
  128, and whether it has a kit). Its presets are listed by bank and program.
- **A plugin row** shows its maker, format, the scan's word (how many presets, or the
  last load's ⚠ error) and, for each part playing it, its load status and CPU. Open, it
  has its housekeeping: New sound from <plugin>, Edit… (while the browser's part plays
  it), its default category (`setSoundCategory`) and the in-process override
  (`setPluginInProcess`, where it can run in process). Rescan sits over the plugin list.
  Opening a plugin lists its factory presets (`listPluginPresets`) beside its
  `.aupreset` files.
- **Each preset** has two actions. **Play now** plays it on the browser's part
  (`assignSound`). **Add to my sounds** (`addToMySounds`) adds its library patch once,
  the same patch a part or map rule gets for it, without playing it; a preset already
  in the library shows "✓ In My Sounds".
- **New sound from <plugin>** loads the plugin's default state on the part
  (`setPartPlugin` with no state) and opens its editor once it plays. Save as… keeps it.

None of this housekeeping stays in the Sounds footer. Picking for a map rule shows the
Sounds tab only, because Play now needs a part.

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
  near match is not the same sound. It plays unnamed (PR 2: no sound, not "edited").
- **Decision (PR 2): the edited baseline is the plugin's first read after it loads**,
  compared by fingerprint on the autosave's schedule. It is not the stored bytes, which a
  plugin may serialize differently; a recalled edit is caught by comparing the recalled
  state with the sound's once, at assign.
- **Decision (PR 2): Save never overwrites a factory or `.aupreset` sound**, as Logic
  won't overwrite a factory preset: it becomes Save as…. So does Save with no sound.
- **Decision (PR 2): Save as… makes the part play the new sound** (a plugin part), so its
  name shows at once. A SoundFont part keeps what it played, as before.
- **Decision (PR 2): Memorize stores the sound even when it is edited**, with the edited
  state: recall names it and marks it edited.
- **Decision: a factory preset can now be a map rule.** Its sound is added with an
  empty state, and the state is captured when the rule first plays it. The old refusal
  ("play it on a part and Save as sound") is gone, in the session and in both mocks.

## Decisions made in PR 3 (map resolution)

- **Decision: the old commands are removed, not shimmed.** `setSoundFont` and
  `setDefaultSoundSet` are gone from the wire (an old client's command is refused like any
  unknown command), with `io.defaultSoundSet` and `io.autoSoundSet`. Only `--sf2` stays,
  hidden, because scripts and tests use it to pick a font file.
- **Decision: an auto-fill from the main font has no route.** Routing it would pin every
  part to bank 0 of the program and lose the style's bank variations, which the main font
  has always played. Only fills from other fonts are routed.
- **Decision: the style-then-global order stays as PR 1 kept it**: the style's rule at any
  layer beats a global rule, and auto comes after both. Existing maps play unchanged.
- **Decision: a rule naming a patch that is gone falls through** to the next layer (then
  auto), instead of the old "fallback" to the main font's own voice.

## Decisions made in PR 4 (the Sounds tab)

- **Decision: "every plugin sound" is the library's plugin sounds.** A factory preset or
  `.aupreset` file becomes one when it is saved (not when it is played); until then it is
  under its plugin's chip, like a font's unmapped presets.
- **Decision: a category chip narrows All sounds**, not the whole catalog, so it stays
  short; Favourites and Recent hold whatever you starred or picked, from any chip.
- **Decision: the chips stay in the side column** (as the Genos category tabs), with an
  Instruments group under Categories, rather than a wrapping chip bar above the list.
- **Decision: the `.aupreset` export is an option of Save as…**, so there is one save
  flow; it keeps #307's replace question.
- **Decision: the Patches tab's editor moved whole into the strip** (tags and defaults
  under Details), and its "move up/down" and "play on R1–L" went: the browser is per part
  and sorts by the catalog. The library file (export, import, "port sends mapped") moved
  under the drawer's pages.
- **Decision: Edit… and Rescan stay in the footer for now**; the Instruments tab (PR 5)
  takes them.
