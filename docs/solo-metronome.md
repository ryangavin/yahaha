# Part solo, Style Track Mute, tempo range, metronome

Issue #30. Commands and state: [app-api.md](app-api.md) (`setStyleSolo`, `setPartSolo`,
`styleTrackMute`, `setTempo`, the Metronome commands; `mixer.styleSolo`,
`mixer.partSolo`, `metronome`).

## Solo

The Genos Mixer solos a channel with touch-and-hold; it lights purple, and touching
again cancels it (RM p.10, OM p.91–93).

- **Style parts** (`setStyleSolo`, engine `set_style_solo`): only the soloed part plays.
  The other parts' sounding notes stop at once.
- **Keyboard parts** (`setPartSolo`, `Parts::set_solo`, read by the input thread): only the
  soloed part sounds from the keys. Soloing Left plays the left hand on Left; soloing a
  Right part plays the whole keyboard on it when Left is not sounding, as with Left off.
  Notes already held keep their note-offs.

Decisions:

- **Two solo groups, one part each.** Because the Genos Mixer has a Style tab and a Panel
  tab, and the app's Mixer drawer mirrors them: a style solo and a keyboard solo are
  independent, and soloing another part moves the solo.
- **Solo overrides the part's Off switch and leaves the switches alone.** Because solo is
  for hearing one part: soloing a part that is off would otherwise be silence, and ending
  the solo gives back exactly the mix you had.
- **Solo is not a gain.** It stops notes; every part's CC7 is untouched (mixer principle).

## Style Track Mute

A Genos Live Control knob function (RM p.148). The knob, 0–127: fully left leaves one
Style part on; turning it up brings the others in, one by one, in the order's sequence;
fully right, all eight are on.

- **A:** Rhythm 2, Rhythm 1, Bass, Chord 1, Chord 2, Pad, Phrase 1, Phrase 2.
- **B:** Chord 1, Chord 2, Pad, Bass, Phrase 1, Phrase 2, Rhythm 1, Rhythm 2.

Decision: **the eight steps split the knob evenly** (`TrackMuteOrder::mask`), because the
manual gives only the order. It sets the parts' on/off switches, so the mixer's On
buttons show it. Not on a Launchkey knob yet: the knobs are unmapped in yahaha so far.

## Tempo 5–500

OM p.46, p.133: the tempo range is 5–500 BPM (`engine::MIN_BPM`, `MAX_BPM`); it was
30–300. `setTempo` sets it directly; tempo ±, tap tempo and style tempos clamp to it. Tap
tempo covers the same range (OM p.46: Tap Tempo is part of it): taps up to 12 s apart
count, so a tap after a pause of more than 2.4 s sets a slow tempo; a pause past 12 s
forgets the taps. A tap whose interval jumps by more than half from the last one starts a
fresh average instead of mixing the two tempos.

## Metronome

Genos Menu > Metronome (RM p.39): On/Off, Volume, Bell (on beat 1), Time Signature.

- It clicks on every beat (a quarter note): with the style's beat lines while the band
  plays (the `on_beat` engine hook, woken exactly on the line), and free-running at the
  tempo while stopped, from when it was turned on or the band stopped.
- **The click is a voice of the built-in synth** (`src/click.rs`), not a MIDI part: the
  engine calls `Sink::click`, which `live::Out` sends to the synth's ring only, never the
  MIDI port. It takes no channel from the style or the keyboard parts.
- Its volume is its own (`setMetronomeVolume`, 0–127, a squared curve peaking at
  −6 dBFS); the synth master and mute apply on top.

Decisions:

- **Time signature: the style's.** Because the Genos setting is for the metronome on its
  own (recording without a style); with a style loaded, clicking against its bar would be
  wrong. Beats are the style's quarter notes.
- **Bell on by default**, the Genos default.
- **The click is a short sine blip** (1320 Hz, 1760 Hz for the bell), because any
  SoundFont's percussion would be a MIDI channel.
