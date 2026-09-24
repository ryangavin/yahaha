# yahaha reference capture kit

Thank you for helping. [yahaha](https://github.com/ryangavin/yahaha) is an open-source program that plays Yamaha style files from a MIDI keyboard. The aim is for it to follow chords the way a Genos does. Only real hardware can tell us whether it does, so this kit asks a Genos or Genos2 owner to play a fixed chord script into ACMP and record what the style sends back over MIDI. We compare that recording with what yahaha plays for the same script, bar by bar and part by part.

It takes about half an hour per style. You need the instrument, a computer with a DAW or any MIDI player that can record at the same time (Cubase, Logic, Reaper, Ableton, Cakewalk, GarageBand, MIDI-OX and similar all work), and a USB cable.

## What is in the kit

- One `<Style>.capture.mid` per style. The kit's copy of this file ends with a table of them, with the style each one is for, its tempo and its length (`yahaha capture-kit` adds the table). Each file contains only yahaha's chord script: left-hand chords on MIDI channel 1, a Section Control message for each INTRO, MAIN, BREAK and ENDING press, and a marker at every bar. It holds no style data.
- The file sends each chord a little before its beat, and each section button half a beat before. That is on purpose: nothing arrives exactly on a beat, where a millisecond either way would change what the instrument plays.
- `capture.script`: the same script as text, if you want to see what will be played.

The script runs through:

- every chord type in the Genos chord table;
- all twelve roots in major and minor;
- inversions and slash chords;
- changes of two, four and eight chords per bar, some off the beat;
- Cancel, 1+5 and 1+8;
- every Main section, whole-bar fills, a whole-bar Break, a one-beat fill, and an Ending.

## Which styles

Record the styles in the table at the end of the kit's copy of this file, in that order. Each style is one recording, and even one is useful.

- **Free pack styles (first six).** These come from Paul J. Drongowski's free *MOX performance styles V2* pack (sandsoftwaresound.net). Download the pack and load the style from USB. That way your instrument plays exactly the same file we have.
- **Factory styles (last three: 60s8Beat, SoulShuffle and 90sDisco).** These are preset styles on the Genos and Genos2. Load them from the Preset tab.

Do not edit or re-save the styles.

## Setting up the instrument

Start from a clean state: power on, or select the style fresh. Then check the following.

1. **Style:** select the style. Leave the **tempo** at the style's own tempo, which is shown in the table at the end of the kit's copy of this file. The MIDI file runs at that tempo, so if the tempo differs the chords land in the wrong place.
2. **ACMP:** on.
3. **Fingering type:** *Fingered On Bass* ([MENU] → Split & Fingering). The script uses slash chords, and they only reach the bass in On Bass.
4. **Split point (Style):** F#2, the default. Chord Detection Area: *Lower*.
5. **AUTO FILL IN:** on. **Transpose** (Keyboard and Master): 0.
6. **Off:** OTS Link, Style Retrigger, Bass Hold, Synchro Stop, Stop ACMP, Chord Looper, and Keyboard Harmony/Arpeggio.
7. **Style Setting** ([MENU] → Style Setting): *Dynamics Control* off, and *Section Change Timing* at *Next Bar* for both settings (if your firmware has them).
8. **Style parts:** all eight style channels on (Rhythm1 to Phrase2 in the channel on/off display), and none muted on the mixer.
9. **MAIN A** lit.

## MIDI settings ([MENU] → MIDI)

1. Select the template **All Parts**.
2. **Transmit:** check that the style parts go out on channels 9–16 (Rhythm1 = 9, Rhythm2 = 10, Bass = 11, Chord1 = 12, Chord2 = 13, Pad = 14, Phrase1 = 15, Phrase2 = 16).
3. **Receive:** set **USB1 channel 1** to **Keyboard**, so the chords the computer sends act as if they were played on the left of the keyboard.
4. **System:**
   - System Exclusive Message Transmit and Receive: **on**. The section buttons in the file are SysEx.
   - Chord System Exclusive Message Transmit: **on**. This lets us see which chord your instrument read.
   - Clock: **Internal**.

## Recording

1. Connect the instrument to the computer by USB.
2. In the DAW, add two tracks:
   - **Playback track:** holds the kit's `.mid` file, with its output set to the instrument's USB port 1.
   - **Record track:** input set to the instrument's USB port 1, all channels. Turn off MIDI thru/echo on this track, so the instrument's output is not sent back to it.
3. Let the DAW take the tempo from the file (import it with the file, rather than typing it in), so it plays at exactly the style's tempo. Turn off any count-in or metronome that sends MIDI.
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
- Your instrument model (Genos or Genos2) and firmware version.
- Anything you changed from the list above, and anything odd you noticed, such as a section that did not change or notes that hung.

Send the recordings **privately**: a private message on the forum, or ask there for another private way. Please don't post them in a public thread or attach them to a public GitHub issue. A recording is a note-for-note performance of the style, and the style belongs to its author (for the factory styles, Yamaha), so it should not be published. Reports of what you noticed can go anywhere.

We never publish a recording, and we never publish a readable note listing made from it. yahaha keeps only a hash of each bar (see `tests/reference/README.md` in the repository), from which the style cannot be rebuilt.

## For the yahaha developer

```sh
yahaha capture-import OrganCruise.recording.mid corpus/MOX_v2/OrganCruise.S930.STY
```

The importer finds bar 1 by lining up the parts that play as written (drums), fits the tempo, and prints:

- where bar 1 is, the instrument's tempo, and how closely the matched notes line up (the MIDI jitter);
- the chords the instrument read differently (from its Chord SysEx), and how far the chords drifted from where yahaha plays them;
- how fast the instrument's clock runs against the computer's, in ppm;
- every bar and part where the notes differ, then the drums of every fill bar on a line of their own;
- apart from those, the timing-sensitive bars and parts that differ (see below).

**Timing-sensitive bars.** The kit sends each chord where a few ms of timing error cannot change what either side plays. It stays clear of every point where the moment a chord lands decides something: a note start or end, 40 ms after a start (a late chord corrects that note outright), and 40 ms before a start, an end or the end of a section (an early chord leaves a note that is about to end, or to be struck again, as it is). It allows 3 ms of timing error (USB jitter and the instrument's delay in reading a chord). Some styles are too busy for that on every part: a few parts leave no place in the beat 3 ms clear of all those points. There the kit places the chord so that as few parts as possible are affected. Each chord that still lands within 3 ms of such a point is *timing-sensitive* on those parts. Its bars on those parts are marked, from just before the chord to where the part next falls silent, together with the bars where the notes it could cut started. The importer accepts either outcome in the marked bars and parts: it lists their differences on their own and does not count them among the bars that differ, `--golden` leaves them out of the `.known` list, and the reference test ignores them. What the kit compares is chord following (NTT and RTR) against a real Genos, and a timing edge case would only add noise. The chord timing check and the clock tolerance hold each chord to its room on the parts it is not timing-sensitive on. The "Timing-sensitive" column in the table below lists the marked bars per part.

The computer plays the chords on its clock and the instrument plays the style on its own, so the chords drift against the style through the recording. The "Clock tolerance" column in the table below is how much drift each style takes before a chord could meet a note on the other side than in yahaha's take. A pair of clocks that drifts further fails verification, and recording again on the same computer and instrument will not help, because the drift stays the same. Make that owner a kit that runs at their instrument's speed, with the drift the report measured, and import the new recording with the same value:

```sh
yahaha capture-kit kit-for-them --clock-ppm 85
yahaha capture-import rec.mid <style> --clock-ppm 85
```

To turn a verified recording into a reference digest, run:

```sh
yahaha capture-import rec.mid <style> --golden tests/reference
```

Add `--listing <file>` to write a readable listing of the recording for local comparison with `yahaha sim`. Never commit that listing.
