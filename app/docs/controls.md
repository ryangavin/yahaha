# Controls

<!-- Generated from app/src/help/tooltips.ts by `npm run docs:controls`. Do not edit. -->

Every control in the app, as its tooltip describes it. Hover over any control in the app (or press `?` for help mode) to see the same text.

## Transport

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Start / Stop** | Starts the band right away, or stops it. Starting from a stop plays the armed Intro first, if you picked one. | START/STOP | `Space` | Pad page 1 (Sections), bottom row, pad 8; Play button |
| **Stop** | Stops the band. Unlike Start / Stop it never starts it. | START/STOP (stop) | — | Stop button |
| **Sync Start** | Arms the band to start on your first left-hand chord. The lamp pulses while it waits. Pressing it while the band plays stops the band and re-arms. | SYNC START | `Y` | Pad page 1 (Sections), top row, pad 4 |
| **Sync Stop** | The band plays only while you hold a chord: let go of every chord key and it stops, play again and it restarts. Not available with the Full Keyboard fingerings. | SYNC STOP | `J` | Pad page 1 (Sections), bottom row, pad 7 |
| **Auto Fill** | When on, switching to another Main plays a fill into it first. | AUTO FILL IN | `U` | Pad page 1 (Sections), top row, pad 8 |
| **Stop ACMP** | With Sync Start off and the band stopped, a chord you hold sounds on the style's bass and pad voices. | Stop Accompaniment | `H` | Pad page 2 (Chord/Setup), bottom row, pad 2 |
| **Panic** | Sends all notes off on every part, for a stuck note. | — | `\` | — |

## Sections

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Intro I** | Stopped: arms Intro I to play when the band starts (the lamp pulses). Playing: queues it for the next bar. | INTRO I | `Q` | Pad page 1 (Sections), top row, pad 1 |
| **Intro II** | Stopped: arms Intro II to play when the band starts (the lamp pulses). Playing: queues it for the next bar. | INTRO II | `W` | Pad page 1 (Sections), top row, pad 2 |
| **Intro III** | Stopped: arms Intro III to play when the band starts (the lamp pulses). Playing: queues it for the next bar. Dark if the style has none. | INTRO III | `E` | Pad page 1 (Sections), top row, pad 3 |
| **Main A** | Switches to Main A at the next bar; the lamp flashes while queued. Press the Main that is playing to play its fill, which also flashes. | MAIN VARIATION A | `1` | Pad page 1 (Sections), bottom row, pad 1 |
| **Main B** | Switches to Main B at the next bar; the lamp flashes while queued. Press the Main that is playing to play its fill, which also flashes. | MAIN VARIATION B | `2` | Pad page 1 (Sections), bottom row, pad 2 |
| **Main C** | Switches to Main C at the next bar; the lamp flashes while queued. Press the Main that is playing to play its fill, which also flashes. | MAIN VARIATION C | `3` | Pad page 1 (Sections), bottom row, pad 3 |
| **Main D** | Switches to Main D at the next bar; the lamp flashes while queued. Press the Main that is playing to play its fill, which also flashes. | MAIN VARIATION D | `4` | Pad page 1 (Sections), bottom row, pad 4 |
| **Break** | Plays the break from the next beat to the end of the bar, then goes back to the Main. The lamp flashes while it waits for the beat. | BREAK | `G` | Pad page 1 (Sections), bottom row, pad 5 |
| **Ending I** | Plays Ending I from the next bar, then stops the band. | ENDING/rit. I | `I` | Pad page 1 (Sections), top row, pad 5 |
| **Ending II** | Plays Ending II from the next bar, then stops the band. | ENDING/rit. II | `O` | Pad page 1 (Sections), top row, pad 6 |
| **Ending III** | Plays Ending III from the next bar, then stops the band. Dark if the style has none. | ENDING/rit. III | `P` | Pad page 1 (Sections), top row, pad 7 |
| **Section lamps** | Dim: available. Bright: playing, or on. Flashing: queued for the next bar (fills: the next beat). Pulsing: armed and waiting for you. Dark: this style doesn't have it. | Section lamp states | — | The pads light the same way, in the same colours |

## Tempo

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Tap tempo** | Tap two or more times in time to set the tempo from your taps. The pad lights on the downbeat while the band plays. | TAP TEMPO | `T` | Pad page 1 (Sections), bottom row, pad 6 |
| **Tempo −** | Slows the tempo by 1 BPM. | TEMPO − | `-` | Function button (right of the pads) |
| **Tempo +** | Speeds the tempo up by 1 BPM. | TEMPO + | `=` | > (Scene Launch) button (right of the pads) |

## Displays

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Tempo** | The current tempo in beats per minute. Loading a style sets the style's own tempo. | Tempo | — | — |
| **Time signature** | The style's time signature, from the style file. | — | — | — |
| **Bar and beat** | Where the band is: bar, and a light per beat. The section playing now and the one queued next are shown alongside. | — | — | — |
| **Chord** | The chord the style is following. When Keyboard transpose is not zero, the chord as you fingered it is shown small underneath. | Chord (Home display, Style area) | — | — |
| **Status line** | The last message: a loaded style, an error, or a Launchkey note or CC nothing is mapped to, which tells you what a button really sends. | — | — | — |

## Style

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Style** | The style that's loaded, and its folder. Click to browse the library. | Style name | `Enter` | — |
| **Previous style** | Loads the previous style in the browser's order (folder, then name). While the band plays it keeps playing and follows your next chord in the new style. | — | `←` | < Track button |
| **Next style** | Loads the next style in the browser's order (folder, then name). While the band plays it keeps playing and follows your next chord in the new style. | — | `→` | Track > button |

## Style browser

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Browse styles** | Opens the style browser: every style under the library folder, by folder. Your keyboard and the Launchkey keep playing while it's open. | Style selection display | `Enter` | — |
| **Filter** | Type to filter by style name, file name or folder (not case-sensitive). ↑/↓, PgUp/PgDn and Home/End move; Enter loads; Shift+Enter previews, or queues for the next bar while the band plays. | — | — | — |
| **All styles** | Every style in the library, in the order < Track and Track > step through: folder, then name. | — | — | < Track and Track > step through this order |
| **Folder** | Shows only the styles in this folder and its subfolders. The folder a style is in is its category. | Style category | — | — |
| **Favourites** | The styles you starred, in library order. Favourites are remembered on this computer. | Favorite | — | — |
| **Recent** | The styles you loaded lately, newest first, however you loaded them. Remembered on this computer. | — | — | — |
| **Load style** | Click or Enter loads this style and closes the browser. While the band plays it keeps playing and follows your next chord in the new style. ▶ marks the loaded style; ‹ and › mark the ones < Track and Track > would load. | — | — | < Track / Track > load the neighbouring rows |
| **Unreadable style** | This file couldn't be read as a style, so it can't be loaded and < Track / Track > skip it. The reason is shown on the row. | — | — | — |
| **Favourite** | Stars or unstars this style, adding it to Favourites. Ctrl+D does the same for the highlighted row. | Favorite | — | — |
| **Preview** | While the band is stopped, plays a few bars of this style over a short default progression without loading it. Shift+Enter previews the highlighted row. | — | — | — |
| **Stop preview** | Stops the preview at once. Loading a style, starting the band or closing the browser stops it too. | — | — | — |
| **Load at next bar** | While the band plays, loads this style on the next bar line instead of right away, so the change lands on the beat. Shift+Enter queues the highlighted row. | — | — | — |
| **Preview on select** | When on and the band is stopped, the highlighted or hovered row previews after a short pause. Remembered on this computer. | — | — | — |
| **Close browser** | Closes the browser without changing the style, and stops any preview. | — | `Esc` | — |

## Fingering

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Fingering type** | How your left hand's chords are read. The pads on page 2 pick one directly. | Fingering Type | `F` | Pad page 2 (Chord/Setup), top row, pads 1–7 |
| **Next fingering type** | Steps to the next fingering type. | Fingering Type | `F` | — |
| **Single Finger** | One key plays a major chord. Add a black key to its left for minor, a white key for 7th, both for m7. | Single Finger | — | Pad page 2 (Chord/Setup), top row, pad 1 |
| **Fingered** | Play the whole chord. The bass is always the chord's root. | Fingered | — | Pad page 2 (Chord/Setup), top row, pad 2 |
| **Fingered On Bass** | Like Fingered, but the lowest note you play becomes the bass, so you can play slash chords. | Fingered On Bass | — | Pad page 2 (Chord/Setup), top row, pad 3 |
| **Multi Finger** | Reads Single Finger and Fingered shapes both, without switching. | Multi Finger | — | Pad page 2 (Chord/Setup), top row, pad 4 |
| **AI Fingered** | Like Fingered, but fewer than three keys can still give a chord, guessed from the chord before. | AI Fingered | — | Pad page 2 (Chord/Setup), top row, pad 5 |
| **Full Keyboard** | Chords are read across the whole keyboard, even split between your hands. | Full Keyboard | — | Pad page 2 (Chord/Setup), top row, pad 6 |
| **AI Full Keyboard** | Full Keyboard with AI Fingered's guessing from fewer keys. 9th, 11th and 13th chords can't be played. | AI Full Keyboard | — | Pad page 2 (Chord/Setup), top row, pad 7 |

## Chord detection

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Chord detection: Upper / Lower** | Lower: your left hand plays the chords. Upper: your right hand does (as Fingered*), and your left hand is free for a bass line. | Chord Detection Area | `D` | Pad page 2 (Chord/Setup), top row, pad 8 |
| **Manual Bass** | In Upper, mutes the style's Bass and gives its voice to Left, so your left hand plays the bass. Left stays on while it's on. Dark in Lower, where it isn't available. | Manual Bass | `Shift+D` | Pad page 2 (Chord/Setup), bottom row, pad 1 |

## Split point

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Split point** | The key that divides the chord section from the right hand (C3 = middle C). Right 1–3 play above it, Left and the chord section at and below it. | Split Point (Style + Left) | — | — |
| **Split −** | Moves the split point down one key. | Split Point | `[` | Pad page 2 (Chord/Setup), bottom row, pad 3 |
| **Split +** | Moves the split point up one key. | Split Point | `]` | Pad page 2 (Chord/Setup), bottom row, pad 4 |

## Transpose

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Transpose** | Keyboard transpose moves your keys and the chord the style follows. Master transpose moves everything that sounds, drums excepted. | TRANSPOSE | — | — |
| **Keyboard transpose −** | Moves your keys and the chord the style follows down a semitone. The pad lights while it's below zero. | TRANSPOSE − (Keyboard) | `;` | Pad page 2 (Chord/Setup), bottom row, pad 5 |
| **Keyboard transpose +** | Moves your keys and the chord the style follows up a semitone. The pad lights while it's above zero. | TRANSPOSE + (Keyboard) | `'` | Pad page 2 (Chord/Setup), bottom row, pad 6 |
| **Master transpose −** | Moves everything that sounds down a semitone, the band included (not the drums). | TRANSPOSE − (Master) | `:` | — |
| **Master transpose +** | Moves everything that sounds up a semitone, the band included (not the drums). | TRANSPOSE + (Master) | `"` | — |
| **Transpose reset** | Puts Keyboard and Master transpose back to 0. The pad lights while either isn't 0. | TRANSPOSE − and + together | `/` | Pad page 2 (Chord/Setup), bottom row, pad 7 |

## One Touch Settings

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **OTS 1** | A sound setup for your own hands that the style's author picked to suit it: the voice, on/off, volume and octave of Right 1–3 and Left. Pressing it swaps your keyboard sounds, and the band doesn't change. Dark if the style has none. | ONE TOUCH SETTING 1 | `Shift+1` | Pad page 3 (OTS/Parts), top row, pad 1 |
| **OTS 2** | The style's second suggested setup for your hands: the voice, on/off, volume and octave of Right 1–3 and Left. Pressing it swaps your keyboard sounds, and the band doesn't change. | ONE TOUCH SETTING 2 | `Shift+2` | Pad page 3 (OTS/Parts), top row, pad 2 |
| **OTS 3** | The style's third suggested setup for your hands: the voice, on/off, volume and octave of Right 1–3 and Left. Pressing it swaps your keyboard sounds, and the band doesn't change. | ONE TOUCH SETTING 3 | `Shift+3` | Pad page 3 (OTS/Parts), top row, pad 3 |
| **OTS 4** | The style's fourth suggested setup for your hands: the voice, on/off, volume and octave of Right 1–3 and Left. Pressing it swaps your keyboard sounds, and the band doesn't change. | ONE TOUCH SETTING 4 | `Shift+4` | Pad page 3 (OTS/Parts), top row, pad 4 |
| **OTS Link** | When on, your hands' sounds follow the band: pressing Main A, B, C or D also recalls OTS 1, 2, 3 or 4. Changing style recalls the setting for the Main that's playing. | OTS LINK | `F10` | Pad page 3 (OTS/Parts), top row, pad 5; Shift + Pad Bank ▼ |
| **OTS Link timing** | When OTS Link swaps the setting: as soon as you press a Main button, not when the band reaches the new Main at the bar line. The Genos calls this Real Time. | OTS Link Timing: Real Time | — | — |

## Keyboard parts

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Right 1 on/off** | Turns Right 1 on or off. Right parts that are on sound together, which is how you layer voices. | PART ON/OFF RIGHT 1 | `5` | Pad page 3 (OTS/Parts), bottom row, pad 1; Panel fader page: button under fader 1 |
| **Right 2 on/off** | Turns Right 2 on or off. Turn on Right 1 and Right 2 together to layer, for example piano and strings. | PART ON/OFF RIGHT 2 | `6` | Pad page 3 (OTS/Parts), bottom row, pad 2; Panel fader page: button under fader 2 |
| **Right 3 on/off** | Turns Right 3 on or off, a third layer for the right hand. | PART ON/OFF RIGHT 3 | `7` | Pad page 3 (OTS/Parts), bottom row, pad 3; Panel fader page: button under fader 3 |
| **Left on/off** | Turns the Left voice on or off: your left hand plays it below the split. It can't be turned off while Manual Bass is on. | PART ON/OFF LEFT | `8` `L` | Pad page 3 (OTS/Parts), bottom row, pad 4; Panel fader page: button under fader 4; Shift + Pad Bank ▲ |
| **Edit Right 1** | Picks Right 1 as the part whose voice Voice −/+ changes. | Part select (Right 1) | `F1` | Pad page 3 (OTS/Parts), bottom row, pad 5; Panel fader page: Shift + button under fader 1 |
| **Edit Right 2** | Picks Right 2 as the part whose voice Voice −/+ changes. | Part select (Right 2) | `F2` | Pad page 3 (OTS/Parts), bottom row, pad 6; Panel fader page: Shift + button under fader 2 |
| **Edit Right 3** | Picks Right 3 as the part whose voice Voice −/+ changes. | Part select (Right 3) | `F3` | Pad page 3 (OTS/Parts), bottom row, pad 7; Panel fader page: Shift + button under fader 3 |
| **Edit Left** | Picks Left as the part whose voice Voice −/+ changes. | Part select (Left) | `F4` | Pad page 3 (OTS/Parts), bottom row, pad 8; Panel fader page: Shift + button under fader 4 |
| **Voice −** | Steps the selected part (the lit Edit pad) to the previous voice. | Voice select | `9` | Pad page 3 (OTS/Parts), top row, pad 7 |
| **Voice +** | Steps the selected part (the lit Edit pad) to the next voice. | Voice select | `0` | Pad page 3 (OTS/Parts), top row, pad 8 |
| **Octave −** | Shifts this part down an octave (down to −2). OTS recalls set it too. | Voice Setting → Tune → Octave | — | — |
| **Octave +** | Shifts this part up an octave (up to +2). OTS recalls set it too. | Voice Setting → Tune → Octave | — | — |
| **Voice** | Picks this part's voice. Under Manual Bass, Left plays the style's Bass voice instead, and this is the voice it goes back to. | Voice select (VOICE buttons) | — | Pad page 3 (OTS/Parts): the Edit pads (bottom row, pads 5–8) pick the part, Voice −/+ (top row, pads 7–8) step its voice |
| **Layer** | The Right parts that are on all sound together on every key above the split: that's a layer. Turn on Right 1 and Right 2 to stack, for example, piano and strings. | PART ON/OFF (Right 1–3 layered) | `5` `6` `7` | Panel fader page: buttons under faders 1–3 |
| **Left hand** | Left plays the keys at and below the split point, with its own voice. Under Manual Bass it plays the style's Bass voice there instead, and the band's own bass goes quiet. | Split Point (Left) | — | — |
| **As written for** | The voice the style's author wrote this band part for, and its MIDI channel on the yahaha port. Load a matching instrument on that channel in Ableton to hear the style as intended; ≈ marks the nearest General MIDI voice to a Yamaha one. | Style parts (Mixer › Style) | — | — |

## Mixer

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Fader page: Panel / Style** | Switches what the Launchkey faders control: Panel is your four keyboard parts, Style is the band's eight parts. The button lights blue on Panel, green on Style. | Mixer tabs (Panel / Style) | `F9` | Button under the master fader |
| **Right 1 volume** | Right 1's volume. The fader is channel 1's CC 7 itself, with no hidden gain behind it. | Mixer › Panel › Right 1 Volume | — | Panel fader page: fader 1 |
| **Right 2 volume** | Right 2's volume. The fader is channel 3's CC 7 itself, with no hidden gain behind it. | Mixer › Panel › Right 2 Volume | — | Panel fader page: fader 2 |
| **Right 3 volume** | Right 3's volume. The fader is channel 4's CC 7 itself, with no hidden gain behind it. | Mixer › Panel › Right 3 Volume | — | Panel fader page: fader 3 |
| **Left volume** | Left's volume. The fader is channel 2's CC 7 itself, with no hidden gain behind it; under Manual Bass it is the bass's level too. | Mixer › Panel › Left Volume | — | Panel fader page: fader 4 |
| **Style part volume** | This band part's volume. The fader is its channel's CC 7 itself (channels 9–16), with no hidden gain behind it. Loading a style sets the faders to the style's own levels, and a pattern that changes its volume moves the fader too, until you move it yourself. | Mixer › Style › Volume | — | Style fader page: faders 1–8 (Rhythm 1 … Phrase 2) |
| **Style part on/off** | Mutes or unmutes this band part. The Launchkey button is lit while the part plays. | Channel On/Off | `Z` `X` `C` `V` `B` `N` `M` `,` | Style fader page: buttons under faders 1–8 |
| **Master volume** | The built-in synth's output level (100 = unity), the only gain after the channel faders. A safety soft clipper above −1 dBFS keeps loud passages from hard clipping; below that the output is untouched. It does not change the MIDI output. | MASTER VOLUME | — | Master fader (both fader pages) |
| **Solo** | Would play only this part. Not available yet: the engine has no solo (backlog #30), so the button is disabled. | Mixer › touch and hold a channel (Solo) | — | — |
| **MIDI out channel** | The channel this part plays on at yahaha's MIDI output: Right 1 = 1, Left = 2, Right 2 = 3, Right 3 = 4, the band 9–16. Map a DAW track (Ableton: MIDI From yahaha, this channel) to record or re-voice it. | Part / Style channel | — | — |
| **Voice** | Keyboard parts: the GM voice the part plays. Style parts: the Yamaha voice (bank MSB/LSB/program) the style was written for, and after ≈ the nearest voice the built-in synth plays for it. | Mixer › Voice | — | — |
| **A fader is the channel's CC 7** | Each fader shows and sends exactly its channel's CC 7 (0–127), with no hidden gain anywhere, so the MIDI output and the synth hear the same level. Loading a style sets the Style faders to the style's own levels. | Mixer › Volume | — | The faders, on both fader pages |
| **Waiting for the fader** | This level moved without the Launchkey fader (a style load, an OTS recall, a pattern, a page switch). The hardware fader does nothing until you move it to within 2 of the level, or across it. | — | — | Soft takeover on every fader |

## Launchkey pad pages

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Pad page 1: Sections** | Intros, Mains, Break, Endings, Sync Start/Stop, Auto Fill, Tap and Start/Stop, each in its own colour. | — | `PgDn` `PgUp` (terminal: `Tab` `Shift+Tab`) | Pad Bank ▲ / ▼ (left of the pads) |
| **Pad page 2: Chord/Setup** | Fingering types, Upper/Lower, Manual Bass, Stop ACMP, split and transpose. All cyan. | — | `PgDn` `PgUp` (terminal: `Tab` `Shift+Tab`) | Pad Bank ▲ / ▼ (left of the pads) |
| **Pad page 3: OTS/Parts** | OTS 1–4 and OTS Link, voice −/+, keyboard parts on/off and which part to edit. All magenta. | — | `PgDn` `PgUp` (terminal: `Tab` `Shift+Tab`) | Pad Bank ▲ / ▼ (left of the pads) |
| **Pad Bank ▲** | Goes to the previous pad page, stopping at page 1, so a few presses always take you home. Lit in the page's colour when there's a page to go to. With Shift: Left on/off. | — | `PgUp` (terminal: `Shift+Tab`) | Pad Bank ▲ (left of the pads) |
| **Pad Bank ▼** | Goes to the next pad page, stopping at page 3. Lit in the page's colour when there's a page to go to. With Shift: OTS Link on/off. | — | `PgDn` (terminal: `Tab`) | Pad Bank ▼ (left of the pads) |

## Launchkey

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Shift** | Hold for the second functions: Pad Bank ▲ = Left on/off, Pad Bank ▼ = OTS Link, the buttons under faders 1–4 on the Panel page = edit that part. On screen, click it to latch the Shift layer, or hold Shift on your computer keyboard. | — | — | Shift button |
| **Launchkey** | Whether the Launchkey is connected in DAW mode, so its pads and buttons are arranger controls. | — | — | — |
| **Unused fader** | On the Panel fader page, faders 5–8 and their buttons do nothing. Switch to the Style page (the button under the master fader) to mix the band. | — | — | Panel fader page: faders 5–8 and the buttons under them |
| **Unused pad** | This pad does nothing on this page and stays dark. | — | — | — |

## Lead-sheet band

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Section playing** | The section the band is playing now, and before the start the Main (and any Intro) it will start with. | Section (MAIN VARIATION, INTRO, ENDING) | — | Pad page 1 (Sections) shows it lit |
| **Section progress** | One cell per bar of the section, filling beat by beat, so you can see how far into the pattern the band is. This band will also show the chord chart when one is loaded. | — | — | — |
| **Next section** | The section queued to play next. A Main or Ending takes over at the next bar line, a fill at the next beat. | — | — | Pad page 1 (Sections): the queued pad flashes |
| **Chord chart** | The iReal Pro chart the band is playing, eight bars to a line, with the bar playing ringed. Section letters mark where Main A–D take over; an amber ring means your left hand has taken over until the next bar line. | — | — | — |

## Keyboard strip

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Keyboard** | The keys you are holding, coloured by the part that sounds them: Right 1–3 above the split, Left below it, grey where a key only feeds chord detection. The shaded band is where chord detection listens, and dots mark the tones of the recognised chord, the ringed one its bass. The engine doesn't report held keys or chord tones yet; until it does, the strip shows only the split and the detection area. | Keyboard (Split Point, chord detection area) | — | The Launchkey's keys |
| **Split point** | Where the left-hand section ends (C3 = middle C). Drag the marker, or focus it and use the arrow keys, to move it one key at a time. | Split Point (Style + Left) | `[` `]` | Pad page 2 (Chord/Setup), bottom row, pad 3 and 4 |
| **Keyboard size** | How many keys the strip shows: 49 or 61 like your Launchkey, or a full 88. It matches the connected Launchkey until you pick one; pick the lit one again to go back to matching. | — | — | — |

## iReal Pro chart player

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Chart mode** | While the band plays, it takes its chords and Main sections from the chosen chart instead of your left hand. A chord you play still takes over, until the next bar line. | — | `M` | — |
| **Previous song** | Chooses the song before this one in the playlist. | — | `(` | — |
| **Next song** | Chooses the song after this one in the playlist. | — | `)` | — |
| **Open playlist** | Imports an iReal Pro playlist exported as an .html file (in iReal Pro: Share, then HTML). Its songs are kept until you quit. | — | — | — |
| **iReal Pro link** | Paste an irealb:// link here (a song or a whole playlist, as iReal Pro shares it), then press Import. | — | — | — |
| **Import link** | Imports the songs in the pasted irealb:// link. | — | — | — |
| **Playlist** | Shows this playlist's songs. | — | — | — |
| **Remove playlist** | Forgets this playlist. If the song playing is in it, chart mode turns off. | — | — | — |
| **Song** | Chooses this chart for the band. With Auto style on, the style its iReal label suggests loads too; with the band stopped, the tempo becomes the chart's. | — | — | — |
| **Fewer choruses** | Plays the form one time fewer before the Ending. | — | — | — |
| **More choruses** | Plays the form one more time before the Ending. | — | — | — |
| **Chart Intro** | The Intro the band plays before the chart's first bar, or none. An Intro you arm yourself before starting plays instead. | INTRO | — | — |
| **Chart Ending** | The Ending the band plays after the chart's last bar. With none, the band stops at the end of the last bar. | ENDING/rit. | — | — |
| **Loop** | Plays the whole song, or one section, over and over instead of ending. Stop the band or press an Ending to finish. | — | — | — |
| **Auto style** | Choosing a song loads the library style its iReal style label suggests. Pick any other style in the browser to override it. | — | — | — |
| **Suggested style** | The library style that best matches the chart's iReal style label. Press it to load that style now. | — | — | — |

## Panels around the hardware view

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Keyboard parts and OTS** | Opens the detail of Right 1–3 and Left (voice, volume, octave, on/off) and the style's One Touch Settings. | PART ON/OFF, Voice Setting, ONE TOUCH SETTING | — | Pad page 3 (OTS/Parts) has the same controls |
| **Mixer** | Opens the full mixer: both fader pages side by side, with each band part's voice. | Mixer (Panel / Style tabs) | — | The faders and the buttons under them |
| **Charts** | Opens the iReal Pro chart player: import playlists, pick a song, and set how the band plays it. | — | — | — |
| **Close** | Closes this panel. The band keeps playing. | — | `Esc` | — |

## Settings

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Settings** | Chord detection, split, transpose, style behaviour, audio, MIDI and the style library. Changes apply at once. | — | — | — |
| **Close settings** | Closes the settings panel. | — | `Esc` | — |
| **Settings: Chord** | Fingering type, chord detection area (Lower or Upper) and Manual Bass. | Menu › Split & Fingering | — | Pad page 2 (Chord/Setup), top row (fingering, Upper) and bottom row, pad 1 (Manual Bass) |
| **Settings: Split point** | Where the keyboard divides between the chord section and the right hand. | Menu › Split & Fingering › Split Point | — | Pad page 2 (Chord/Setup), bottom row, pads 3–4 |
| **Settings: Transpose** | Keyboard and Master transpose, in semitones. | Menu › Transpose | — | Pad page 2 (Chord/Setup), bottom row, pads 5–7 |
| **Settings: Style** | How the band starts, stops and fills: Sync Start/Stop, Auto Fill and Stop Accompaniment. | Menu › Style Setting | — | Pad page 1 (Sections) (Sync Start, Sync Stop, Auto Fill); Pad page 2 (Chord/Setup), bottom row, pad 2 (Stop ACMP) |
| **Settings: Audio** | The built-in synth: on or off, which output pair it plays on, its SoundFont and its master volume. | — | — | — |
| **Settings: MIDI** | Which MIDI inputs play yahaha, the yahaha output port, and the Launchkey connection. | Menu › MIDI | — | — |
| **Settings: Library** | The folders yahaha looks for style files in, and a rescan. | Style selection (USB / User folders) | — | — |
| **Split point** | Drag the marker, or click a key, to set the split (C3 = middle C). Keys at and below it are Left and the chord section, keys above it play Right 1–3. With focus, ←/→ move it a key and PgUp/PgDn an octave. | Split Point (Style + Left) | `[` `]` | Pad page 2 (Chord/Setup), bottom row, pads 3–4 |
| **Section change timing (coming soon)** | When a Main you press takes over: Immediate (at the next beat) or Next Bar. Coming in M5; for now a new Main always waits for the next bar line (with Auto Fill, its fill starts at the next beat). | Section Change Timing | — | — |
| **OTS Link timing (coming soon)** | With OTS Link on, whether the One Touch Setting changes the moment you press a Main (Immediate) or when the new Main actually starts (At Main Section Change). Coming in M5; for now it is always Immediate. | OTS Link Timing | — | — |
| **Synchro Stop window (coming soon)** | With Sync Stop on: hold a chord longer than this and Sync Stop cancels itself, so the style keeps playing when you let go. A quicker release still stops the style. Coming in M5. | Synchro Stop Window | — | — |
| **Style folders** | The folders yahaha reads style files from (.sty, .prs, .sst and more), with subfolders as categories. Pass them on the command line or set YAHAHA_STYLES. | Style selection (User / USB) | — | — |
| **Rescan styles** | Reads the style folders again, picking up files you added, changed or removed. The band keeps playing. | — | — | — |

## Audio

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Audio output** | The output pair the built-in synth plays on. `--audio-out N` sets it at launch. | — | `A` | — |
| **SoundFont** | The General MIDI SoundFont (.sf2) the built-in synth plays, from the soundfonts folder. `--sf2 file` sets it at launch. | — | — | — |
| **Mute synth** | Silences the built-in synth, for when you play Ableton's sounds from the yahaha MIDI port instead. | — | `K` | — |
| **Built-in synth** | Turns the built-in SoundFont synth's sound on or off. The yahaha MIDI port keeps playing either way, for Ableton or other sounds. | — | `K` | — |

## MIDI

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **MIDI input** | Whether this MIDI source plays yahaha. The Launchkey's DAW port carries its pads and buttons. | — | — | — |
| **Inputs: all or selected** | All: every MIDI source plays yahaha, merged. Selected: only the sources you switch on below. `--all-inputs` and `--input name` set it at launch. | — | — | — |
| **yahaha MIDI output** | The virtual MIDI port yahaha plays the band and your parts on. Pick it as a MIDI input in Ableton to use your own sounds. | MIDI Transmit | — | — |
| **Palette LEDs** | Lights the Launchkey with its built-in palette colours and hardware flashing instead of exact RGB colours. Try it if the pads look wrong or lag. `--palette-leds` sets it at launch. | — | — | Every pad and button light |

## App

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Help mode** | Grows the help footer to show the whole entry, and keeps the last control you hovered or tabbed to there while you try it. Controls keep working. | — | `?` | — |
| **Pop-up tips** | Also shows each entry in a pop-up next to the control, as well as in the help footer. Off by default, because a pop-up covers the controls while you play. | — | — | — |
| **Light / dark** | Switches between the dark stage theme and a light one. | — | — | — |
