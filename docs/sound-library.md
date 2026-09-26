# Sound library and program map (#103)

A short list of sounds you choose, about 20. The owner wants every style to play those
same patches, the way a Genos player has favourite voices, instead of the 128 GM sounds
a style asks for. This phase covers SoundFont patches. Plugin patches play through #91's
plugin rack once that is merged; until then they play the SoundFont fallback.

- Code: `src/patches/` (model, map, store, route table, `.sf2` presets, port mapping),
  `src/synth/routing.rs` (the synth's side), `src/session/sound_library.rs` (commands,
  state, style hand-off, SoundFont loading, auditions).
- API: `SoundLibraryCmd` and `state.soundLibrary` (docs/app-api.md, "Sound library").
- App: the Sound Library drawer, and the Library tab of the voice picker.

## Patches

A patch has a name, a category, tags, a favourite flag, a source and defaults.

- **Categories** follow the Genos Voice Selection tabs: Piano, E.Piano, Organ, Guitar,
  Bass, Strings, Brass, Sax/Woodwind, Synth Lead, Pad, Choir, Drums/Perc and SFX.
- **Source**: a SoundFont preset (a file in the SoundFont folder, its bank and program;
  bank 128 holds drum kits), or a plugin (the Audio Unit id and its saved state, as #91
  stores them).
- **Defaults** are MIDI settings sent as CCs, so the mixer shows them and there is no
  hidden gain: volume (CC7), pan (CC10), reverb (CC91), chorus (CC93) and octave.

The library is saved as versioned JSON, `sound-library.json` in the data folder
(`~/Documents/yahaha`, the folder Registration (#99) uses; `--data-dir` changes it). It is
saved after every change. Export writes the same format, and import accepts it or a bare
list of patches. A file written by a newer yahaha is not read and never saved over.

## The program map

When a Style part sends a program change, the map decides which patch it plays. Within
one map, the most specific rule wins:

1. **Drums.** Rhythm 1 and 2 (MIDI channels 9 and 10), and any part on a Yamaha drum or
   SFX kit bank (MSB 126/127), play the drum rule's patch. Individual kit pieces are not
   mapped.
2. **Program override.** One GM program plays a patch.
3. **Family rule.** A GM family (programs in groups of 8: Piano 0–7, Chromatic Perc. 8–15,
   and so on up to Sound FX 120–127) plays a patch.

Bank variations collapse onto their program: the bank select is ignored, so one rule
covers every XG/GS variation. A Yamaha voice outside the GM banks first becomes the GM
program the synth already plays for it (`synth::gm_fallback`). Genos bank 8 voices
(MegaVoice, S.Art!) map to their instrument's GM program through the Data List's table
(`src/voice_gm.rs`, #228): NylonGuitar (8/0/PC#1) is Nylon Guitar, in the Guitar family, not
Acoustic Grand Piano. Bank 9 (the Ensemble parts' S.Art! voices) keeps bank 8's numbering
and has its own table (#270): TenorSax (9/66/PC#81) is Tenor Sax. Bank 104 is GM numbered
and plays its program as it is. Anything no rule covers
plays the SoundFont voice it played before the library existed.

A style can have its own map as well. Its rules win, and whatever it leaves unset falls
through to the global map. It is stored in the library file, keyed by the style's file
name, and never in the style file.

Keyboard parts use the map too. A part can pick a library patch of its own
(`setPartPatch`, the voice picker's Library tab), which then applies its defaults. A
plugin patch picked there plays its plugin with the patch's state, through #91's
`assign_channel_plugin` (the `setPartPlugin` path); leaving the patch takes the plugin
away again (`sync_part_plugins`).
Otherwise the part's GM voice, whether from the GM list or an OTS, resolves through the
map in the same way as a Style part's voice.

## How it plays

The map is resolved off the real-time threads, into a table the audio thread only reads
(`patches::Routes`): a route per GM program and a drum route, in two banks. When a style
is handed to the engine, the control side writes the bank the playing style is not
using, and puts its number in the style (`Prepared::route_bank`). When the engine takes
the style over (`on_style_loaded`, right after the style's setup), it sends that bank to
the synth, in the MIDI stream. So the synth switches maps exactly where the styles
switch. A style chosen while another is still waiting reuses that one's bank; but if the
engine has already taken the waiting one over (its snapshot shows its tag), the control
side promotes it first, so the new style gets the other bank and never rewrites the one
playing. An edit rewrites the table, and the synth routes its channels again from the
programs they have.

The synth holds one synthesizer per SoundFont the library uses: the main font's two, as
before, plus one more per extra font (`Rack::with_fonts`). These load on a thread of
their own, reusing fonts that are already parsed, and swap in like `setSoundFont`.

Where each message goes on the audio thread:

- **To the channel's current synthesizer only:** note-ons, program changes and bank
  selects.
- **To all synthesizers:** everything else, including note-offs, controllers and bend.

So a channel keeps its level, pan and bend on whichever font plays it, and a note never
hangs when its channel moves to another font. The per-channel meters read every
synthesizer.

#91's per-channel route table (`crate::route::ChannelRoutes`, one `AtomicU64`) decides
whether each channel renders on the SoundFont side or on a plugin. The control side
keeps it in line with the map:

- A Style part whose setup voice resolves to a plugin patch is assigned that plugin
  (`assign_channel_plugin`).
- Every other channel stays `Source::SoundFont(0)` (the SoundFont renderer). Which
  SoundFont of the rack plays it is the rack's own per-program routing, so a program
  change inside a section still picks its font on the audio thread.

A SoundFont that fails to load is left out (its patches play the fallback) and not tried
again until the file changes; the others still load. A merge import never saves over a
library file it couldn't read (a newer yahaha's); a replace import keeps that file as
`sound-library.json.bak`. A patch's default left blank (pan, sends) leaves the part's
value as it is.

## Decisions

- **Decision: Rhythm 1 (channel 9) is a drum part, like channel 10**, because the synth
  already plays it on the drum bank, and Genos styles use both Rhythm parts for kits.
- **Decision: a style's own rules beat every global rule**, even a global program
  override. The style map is the more specific choice, made for that style.
- **Decision: keyboard parts' GM voices, including OTS voices, go through the map.** The
  issue says every place that picks a voice uses the library. An explicit library patch
  on a part wins, and picking a GM voice or recalling an OTS that voices the part ends
  it.
- **Decision: the `yahaha` port mirrors the style by default.** Program changes go out
  unchanged, so a DAW recording is faithful. An option (`setPortSendsMapped`) sends the
  mapped bank and program instead: SoundFont bank as MSB (127 for drum kits), LSB 0.
- **Decision: a patch's volume sets a Style part's level only when the style sets no
  CC7 for that part.** It fills the style's level before the style loads, so the mixer
  shows it and it goes out as CC7. Pan, sends and octave apply to keyboard parts only,
  because a style's own pan and sends are part of its arrangement.
- **Decision: plugin routes are per channel, from the setup voice, and made at style
  load.** A program change inside a section cannot move a channel between a plugin and
  the SoundFont, because plugins load asynchronously and must never load on the audio
  thread. Within the SoundFont side, a mid-section program change still picks its font
  from the per-program table: a small real-time read, no allocation.
- **Decision: the synth loads every SoundFont a library patch uses**, not just the ones
  the current map needs. The library is small, so assigning a patch never waits for a
  load. A new font swaps the rack in the same way as `setSoundFont`.
- **Decision: auditions play on channel 16 while the band is stopped**, like the style
  preview. Every channel is taken by a part, and channel 16 (Phrase 2) is silent when the
  band is stopped. Its setup comes back afterwards from the synth's record of it.
- **Decision: a plugin patch auditions on channel 16 too.** Its plugin loads there
  through #91's `assign_channel_plugin`, like a part's, then plays the same phrase
  through the rack, and is cleared when the audition ends (the band starting ends it).
  A plugin the map gives Phrase 2 is assigned again afterwards. The audition sets the
  channel's CC7, CC11 and CC10, because the rack keeps a plugin channel's own.
- **Decision: Multi Pad channels (5–8) are not mapped.** Pads are short phrases written
  for their voices.
- **Decision: a keyboard part plays whatever was picked last.** A plugin patch on a part
  is the part's plugin, as `setPartPlugin` would make it: the Plugins tab shows it, the
  editor edits it and #91 saves it with the part. A plugin picked on the Plugins tab ends
  the part's patch, and a SoundFont patch picked over such a plugin ends the plugin. A GM
  voice picked over a Plugins-tab plugin still leaves the plugin playing (#91's rule).
  The link from the part to its patch lasts for the session, like every part patch.
- **Decision: the per-style map is keyed by the style's file name.** It survives moving
  the style folder. Two styles with the same file name share a map.
- **Decision: there is no user OTS memory yet, so OTS stores no patch id.** OTS voices
  resolve through the map. Registration (#99) is not merged, so it gets no patch item in
  this PR; the hook is `SoundLibraryCmd::SetPartPatch` and `KeyboardPart::patch`.

## Not yet

- A user OTS / Registration item for a part's patch (after #99).
- A keyboard part's GM voice that the map sends to a plugin patch plays the SoundFont
  fallback; only a part's own plugin patch plays its plugin.
- A patch's optional keyboard range.
