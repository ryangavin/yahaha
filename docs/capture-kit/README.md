# yahaha reference capture kit

Thank you for helping. [yahaha](https://github.com/ryangavin/yahaha) is an open-source program that plays Yamaha style files from a MIDI keyboard. The aim is for it to follow chords the way a Genos does. Only real hardware can tell us whether it does, so this kit asks a Genos or PSR-SX owner to play a fixed chord script into ACMP and record what the style sends back over MIDI. We compare that recording with what yahaha plays for the same script, bar by bar and part by part.

It takes about half an hour per style. You need the instrument, a computer with a DAW or any MIDI player that can record at the same time (Cubase, Logic, Reaper, Ableton, Cakewalk, GarageBand, MIDI-OX and similar all work), and a USB cable.

## What is in the kit

- One `<Style>.capture.mid` per style, listed at the end of this file. It contains only yahaha's chord script: left-hand chords on MIDI channel 1, a Section Control message for each INTRO, MAIN, BREAK and ENDING press, and a marker at every bar. It holds no style data.
- `capture.script`: the same script as text, if you want to see what will be played.

The script runs through:

- every chord type in the Genos chord table;
- all twelve roots in major and minor;
- inversions and slash chords;
- changes of two, four and eight chords per bar, some off the beat;
- Cancel, 1+5 and 1+8;
- every Main section, a Break, fills, and an Ending.

## Which styles

Record the styles in the table at the end of this file, in that order. Each style is one recording, and even one is useful.

- **Free pack styles (first six).** These come from Paul J. Drongowski's free *MOX performance styles V2* pack (sandsoftwaresound.net). Download the pack and load the style from USB. That way your instrument plays exactly the same file we have.
- **Factory styles (last three).** These are preset styles on the Genos and Genos2. Load them from the Preset tab. If you own a PSR-SX, skip any that your model does not have.

Do not edit or re-save the styles.

## Setting up the instrument

Start from a clean state: power on, or select the style fresh. Then check the following.

1. **Style:** select the style. Leave the **tempo** at the style's own tempo, which is shown in the table below. The MIDI file runs at that tempo, so if the tempo differs the chords land in the wrong place.
2. **ACMP:** on.
3. **Fingering type:** *Fingered On Bass* (Genos: [MENU] → Split & Fingering; PSR-SX: [MENU] → Split Point/Chord Fingering). The script uses slash chords, and they only reach the bass in On Bass.
4. **Split point (Style):** F#2, the default. Chord Detection Area: *Lower*.
5. **AUTO FILL IN:** on. **Transpose** (Keyboard and Master): 0.
6. **Off:** OTS Link, Style Retrigger, Bass Hold, Synchro Stop, Unison & Accent (PSR-SX), Chord Looper, and Keyboard Harmony/Arpeggio.
7. **MAIN A** lit.

## MIDI settings ([MENU] → MIDI)

1. Select the template **All Parts**.
2. **Transmit:** check that the style parts go out on channels 9–16 (Rhythm1 = 9, Rhythm2 = 10, Bass = 11, Chord1 = 12, Chord2 = 13, Pad = 14, Phrase1 = 15, Phrase2 = 16).
3. **Receive:** set **USB1 channel 1** to **Keyboard**, so the chords the computer sends act as if they were played on the left of the keyboard.
   - If your model has no "Keyboard" part, tick channel 1 on the **Chord Detect** and **On Bass Note** pages instead.
4. **System:**
   - System Exclusive Message Transmit and Receive: **on**. The section buttons in the file are SysEx.
   - Chord System Exclusive Message Transmit: **on**. This lets us see which chord your instrument read.
   - Clock: **Internal**.

## Recording

1. Connect the instrument to the computer by USB.
2. In the DAW, add two tracks:
   - **Playback track:** holds the kit's `.mid` file, with its output set to the instrument's USB port 1.
   - **Record track:** input set to the instrument's USB port 1, all channels. Turn off MIDI thru/echo on this track, so the instrument's output is not sent back to it.
3. Set the DAW project tempo to the file's tempo, or let the DAW import the tempo from the file. Turn off any count-in or metronome that sends MIDI.
4. On the instrument, press **[SYNC START]**. The lamp flashes.
5. Start recording in the DAW from the beginning of the project.
   - The file has two empty bars first.
   - Your instrument starts the style on the first chord and plays the intro.
   - The whole script then plays through by itself, with no buttons to press.
   - If a section button in the file is not taken, write down the bar number and carry on.
6. Keep recording until the ending has finished and the style has stopped. The file's last marker says "end".
7. Save the recording as a Standard MIDI File, type 0 or 1, and name it after the style, for example `OrganCruise.recording.mid`.

## What to send back

- The recording(s).
- Your instrument model and firmware version (Genos, Genos2, PSR-SX900 …).
- Anything you changed from the list above, and anything odd you noticed, such as a section that did not change or notes that hung.

Post them in the forum thread, or attach them to an issue at https://github.com/ryangavin/yahaha/issues.

The recording holds only what your instrument played for our script, so it is yours to share. We never publish it, and we never publish a readable note listing made from it. yahaha keeps only hashes of each bar (see `tests/reference/README.md` in the repository).

## For the yahaha developer

```sh
yahaha capture-import OrganCruise.recording.mid corpus/MOX_v2/OrganCruise.S930.STY
```

The importer finds bar 1 by lining up the parts that play as written (drums), fits the tempo, and prints:

- where bar 1 is and how well the drums matched;
- the chords the instrument read differently (from its Chord SysEx);
- every bar and part where the notes differ.

To turn a verified recording into a reference digest, run:

```sh
yahaha capture-import rec.mid <style> --golden tests/reference
```

Add `--listing <file>` to write a readable listing of the recording for local comparison with `yahaha sim`. Never commit that listing.
