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

## Registration Memory

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Registration 1** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [1] | `Shift+Q` | Pad page 4 (Registration), top row, pad 1 |
| **Registration 2** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [2] | `Shift+W` | Pad page 4 (Registration), top row, pad 2 |
| **Registration 3** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [3] | `Shift+E` | Pad page 4 (Registration), top row, pad 3 |
| **Registration 4** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [4] | `Shift+R` | Pad page 4 (Registration), top row, pad 4 |
| **Registration 5** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [5] | `Shift+T` | Pad page 4 (Registration), top row, pad 5 |
| **Registration 6** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [6] | `Shift+Y` | Pad page 4 (Registration), top row, pad 6 |
| **Registration 7** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [7] | `Shift+U` | Pad page 4 (Registration), top row, pad 7 |
| **Registration 8** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [8] | `Shift+I` | Pad page 4 (Registration), top row, pad 8 |
| **Registration 9** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [9] | `Shift+O` | Pad page 4 (Registration), bottom row, pad 1 |
| **Registration 10** | Recalls the panel stored on this button: style, section, tempo, your parts, the mixer, split and more (the groups it memorized, less any you froze). Blue when it holds a setup, red when it is the one in use, dark when empty. With Memory armed, stores the panel here instead. | REGISTRATION MEMORY [10] | `Shift+P` | Pad page 4 (Registration), bottom row, pad 2 |
| **Memory** | Arms Memorize: the next Registration button you press stores the whole panel (the ticked Memorize groups) on it, replacing what it held. The buttons flash while it waits. Press Memory again to cancel. | MEMORY | `F5` | Pad page 4 (Registration), bottom row, pad 5 |
| **Freeze** | While on, recalling a registration leaves the ticked Freeze groups as they are: freeze Style to change voices without changing the band, or Tempo to keep your tempo. | FREEZE | `F6` | Pad page 4 (Registration), bottom row, pad 6 |
| **Regist −** | Steps back through the bank's Registration Sequence and recalls that button. Works only while the sequence is on. | Registration Sequence − (Regist − pedal) | `F7` | Pad page 4 (Registration), bottom row, pad 7 |
| **Regist +** | Steps forward through the bank's Registration Sequence and recalls that button, like a pedal on stage. At the end it stops, starts again, or moves to the next bank, as the sequence says. | Registration Sequence + (Regist + pedal) | `F8` | Pad page 4 (Registration), bottom row, pad 8 |
| **Bank −** | Loads the previous bank file in the Registration folder. Its buttons light up but nothing is recalled until you press one. | REGIST BANK − | `F11` | Pad page 4 (Registration), bottom row, pad 3 |
| **Bank +** | Loads the next bank file in the Registration folder. Its buttons light up but nothing is recalled until you press one. | REGIST BANK + | `F12` | Pad page 4 (Registration), bottom row, pad 4 |
| **Bank** | The bank of ten buttons in use. Pick another bank file from the Registration folder; a star means it has changes that aren't saved yet. | Registration Bank Selection | — | — |
| **Registration and Playlist** | Opens the Registration panel: what each button holds, renaming and clearing, the Memory and Freeze groups, the Registration Sequence and the Playlist. | Regist Bank Info / Edit, Regist Sequence, Regist Freeze, PLAYLIST | — | — |
| **Registration panel page** | Switches between the bank's buttons, the Memory and Freeze groups, the Registration Sequence and the Playlist. | — | — | — |
| **New bank** | Starts a new, empty bank. Give it a name and save it to keep it; unsaved changes to the bank in use are dropped. | Regist Bank: New | — | — |
| **Save bank** | Saves the bank to its file in the Registration folder, or under the name you typed as a new file. If another bank already has that name, nothing is saved: pick another name, or use Overwrite. Once a bank has a file, memorizing, renaming and sequence edits save themselves. | Regist Bank: Save | — | — |
| **Overwrite bank** | Another bank already has the name you typed. Overwrite replaces that bank's file with this bank; what it held is lost. | Regist Bank: Save (overwrite) | — | — |
| **Bank name** | The name to save the bank under. Saving with a new name makes a new file and leaves the old one as it was. A name another bank already has is refused, unless you choose Overwrite. | — | — | — |
| **Button contents** | What this button holds: its style, tempo and the voices of Right 1–3 and Left. Click it to recall it. | Regist Bank Info | — | — |
| **Memorize here** | Stores the panel as it is now on this button (the ticked Memorize groups), replacing what it held. | MEMORY + [1]–[10] | — | — |
| **Rename** | Renames this button. The name shows in the Registration bar and in playlists. | Regist Bank Edit: Rename | — | — |
| **Clear** | Empties this button. Its lamp goes dark. | Regist Bank Edit: Delete | — | — |
| **Memorize group** | Ticked groups are what Memory stores on a button; a recall only changes what the button stored. Untick Tempo, say, for buttons that should keep whatever tempo you are playing. | Registration Memory window (items to register) | — | — |
| **Freeze group** | Ticked groups stay as they are when you recall a registration, while Freeze is on. Style also covers the section, the Style mixer, the split, the fingering and the Left part, as on the Genos. | Regist Freeze display | — | — |
| **Registration Sequence** | Turns the Registration Sequence on, so Regist + and Regist − step through the bank's sequence. As on the Genos this is a panel setting, not part of the bank: it stays as it is when you change banks, and yahaha remembers it between sessions. | Registration Sequence On/Off | — | — |
| **Add step** | Adds this button to the end of the sequence. A button can come more than once. | Registration Sequence: Insert | — | — |
| **Sequence step** | A step of the sequence: the button it recalls. Click to take it out of the sequence; the ringed step is the one last recalled. | Registration Sequence: Delete | — | — |
| **Clear sequence** | Removes every step from the sequence. | Registration Sequence: Clear | — | — |
| **At the end** | What Regist + does after the last step: Stop does nothing more, Top starts again at the first step, Next bank moves on to the next bank file and its first step. | Registration Sequence end (Stop / Top / Next) | — | — |

## Playlist

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Previous song** | Loads the playlist record before the one in use: its bank and button, or its style. | Playlist (previous record) | `<` | Shift + < Track button |
| **Next song** | Loads the next playlist record: its bank and button, or its style. Step through a set list this way without looking at the screen. | Playlist (next record) | `>` | Shift + Track > button |
| **Playlist record** | A song in the set list. Click it to load its bank (and recall its button) or its style; the lit one is the song in use. A struck-out name means its file is gone. | Playlist Record (Load) | — | — |
| **Playlist** | The set list in use. Pick another playlist file from the Playlists folder; a star means it has unsaved changes. | Playlist File Selection | — | — |
| **New playlist** | Starts a new, empty set list. Unsaved changes to the one in use are dropped. | Playlist: New | — | — |
| **Save playlist** | Saves the set list in the order shown (a sorted list is saved sorted, and goes back to Normal), to its file or under the name you typed. A name another playlist already has is refused: pick another, or use Overwrite. | Playlist: Save | — | — |
| **Overwrite playlist** | Another playlist already has the name you typed. Overwrite replaces that playlist's file with this set list. | Playlist: Save (overwrite) | — | — |
| **Playlist name** | The name to save the set list under. A new name makes a new file; another playlist's name needs Overwrite. | — | — | — |
| **Add this bank** | Adds the bank in use to the end of the set list, recalling the button that is lit. The bank must be saved first. | Add Record: Select from Registration Bank | — | — |
| **Add this style** | Adds the loaded style to the end of the set list, for a song that needs only the style. | — | — | — |
| **Append playlist** | Adds every record of another playlist file to the end of this one. | Add Record: Append Playlist | — | — |
| **Button to recall** | Which button of the bank this record recalls after loading it, or none to only load the bank. | Record Edit: Load Regist Memory | — | — |
| **Move up** | Moves the record one place up the set list. Off while the list is sorted. | Playlist: Up | — | — |
| **Move down** | Moves the record one place down the set list. Off while the list is sorted. | Playlist: Down | — | — |
| **Delete record** | Takes the record out of the set list; its bank or style file is not touched. Off while the list is sorted. | Playlist: Delete | — | — |
| **Sort** | Shows the set list in its own order, A to Z or Z to A. Saving while sorted saves that order. | Playlist: Sort (A to Z) | — | — |

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

## Multi Pads

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Multi Pad** | Plays the pad's phrase from the top (keys Shift+Z, X, C, V): at once when the band is stopped, at the next bar line while it plays. With pads in Synchro Start standby, pressing one of them starts them all. Blue: has data; red: playing; flashing red: waiting for Synchro Start; amber: waiting for the bar line. | MULTI PAD CONTROL [1]–[4] | `Shift+Z` `Shift+X` `Shift+C` `Shift+V` | — |
| **Stop all pads** | Stops every Multi Pad at once and cancels Synchro Start standby. The band keeps playing. Key: Shift+B. | MULTI PAD CONTROL [STOP] | `Shift+B` | — |
| **Stop this pad** | Stops only this pad, now. The other pads keep playing. | [STOP] + pad | — | — |
| **Synchro Start** | Puts the pad in standby (flashing red): it starts with your next chord in the chord section, when the band starts, or when you press any pad in standby; while the band plays, at the next bar line. Press again to cancel. | [SELECT] + pad (Synchro Start) | — | — |
| **Repeat** | On: the pad loops until you stop it. Off: it plays once. The bank file sets it; a change here lasts until another bank loads. | Repeat (Multi Pad Edit) | — | — |
| **Chord Match** | On: the pad follows the chord you play, like the band does. Off: it plays exactly as recorded, as drum pads usually do. | Chord Match (Multi Pad Edit) | — | — |
| **Multi Pad bank** | Loads this bank's four pads. Pads playing stop. The list is every .pad file in your style folders. | Multi Pad Bank Selection | — | — |
| **No bank** | Unloads the bank: the pads go dark. | — | — | — |
| **Synchro Stop: Style Stop** | On: looping pads stop when the band stops. Off: they play on until you stop them. | Multi Pad Synchro Stop (Style Stop) | — | — |
| **Synchro Stop: Style Ending** | On: looping pads stop when an Ending starts. Off: they play through the Ending. | Multi Pad Synchro Stop (Style Ending) | — | — |

## Launchkey pad pages

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Pad page 1: Sections** | Intros, Mains, Break, Endings, Sync Start/Stop, Auto Fill, Tap and Start/Stop, each in its own colour. | — | `PgDn` `PgUp` (terminal: `Tab` `Shift+Tab`) | Pad Bank ▲ / ▼ (left of the pads) |
| **Pad page 2: Chord/Setup** | Fingering types, Upper/Lower, Manual Bass, Stop ACMP, split and transpose. All cyan. | — | `PgDn` `PgUp` (terminal: `Tab` `Shift+Tab`) | Pad Bank ▲ / ▼ (left of the pads) |
| **Pad page 3: OTS/Parts** | OTS 1–4 and OTS Link, voice −/+, keyboard parts on/off and which part to edit. All magenta. | — | `PgDn` `PgUp` (terminal: `Tab` `Shift+Tab`) | Pad Bank ▲ / ▼ (left of the pads) |
| **Pad Bank ▲** | Goes to the previous pad page, stopping at page 1, so a few presses always take you home. Lit in the page's colour when there's a page to go to. With Shift: Left on/off. | — | `PgUp` (terminal: `Shift+Tab`) | Pad Bank ▲ (left of the pads) |
| **Pad Bank ▼** | Goes to the next pad page, stopping at page 4. Lit in the page's colour when there's a page to go to. With Shift: OTS Link on/off. | — | `PgDn` (terminal: `Tab`) | Pad Bank ▼ (left of the pads) |
| **Pad page 4: Registration** | Registration buttons 1–10 in the Genos lamp colours (red in use, blue stored, dark empty), Bank −/+, Memory, Freeze and Regist −/+. The other pads are orange. | — | `PgDn` `PgUp` (terminal: `Tab` `Shift+Tab`) | Pad Bank ▲ / ▼ (left of the pads) |

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

## Keyboard strip

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Keyboard** | The keys you are holding, coloured by the part that sounds them: Right 1–3 above the split, Left below it, grey where a key only feeds chord detection. The shaded band is where chord detection listens, and dots mark the tones of the recognised chord, the ringed one its bass. The engine doesn't report held keys or chord tones yet; until it does, the strip shows only the split and the detection area. | Keyboard (Split Point, chord detection area) | — | The Launchkey's keys |
| **Split point** | Where the left-hand section ends (C3 = middle C). Drag the marker, or focus it and use the arrow keys, to move it one key at a time. | Split Point (Style + Left) | `[` `]` | Pad page 2 (Chord/Setup), bottom row, pad 3 and 4 |
| **Keyboard size** | How many keys the strip shows: 49 or 61 like your Launchkey, or a full 88. It matches the connected Launchkey until you pick one; pick the lit one again to go back to matching. | — | — | — |

## Panels around the hardware view

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Keyboard parts and OTS** | Opens the detail of Right 1–3 and Left (voice, volume, octave, on/off) and the style's One Touch Settings. | PART ON/OFF, Voice Setting, ONE TOUCH SETTING | — | Pad page 3 (OTS/Parts) has the same controls |
| **Mixer** | Opens the full mixer: both fader pages side by side, with each band part's voice. | Mixer (Panel / Style tabs) | — | The faders and the buttons under them |
| **Close** | Closes this panel. The band keeps playing. | — | `Esc` | — |
| **Multi Pads** | Opens the Multi Pads: four short phrases from a pad bank that you trigger over the band, and the bank list. | MULTI PAD CONTROL | — | — |

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
| **Pedals and wheels** | The sustain pedal and footswitches, what each pedal does, and which parts the pedal and the wheels reach. | Assignable, Controller | — | — |

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

## Pedals and wheels

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Pedal function** | What this pedal does: Sustain (or Sostenuto, Soft), a style control such as Start/Stop, Fill Up or Break, an OTS, the Registration bank, tempo, transpose or a part on/off. | Assignable › Foot Pedal | — | The pedal plugged into the sustain jack |
| **Pedal CC** | The control change this pedal listens for on the keyboards: the Launchkey's sustain jack sends CC 64. Clear it and the pedal listens to nothing. Bank select (0, 32), the modulation wheel (1), data entry (6, 38), volume (7), (N)RPN (98-101) and the channel mode messages (120-127) can't be used. | — | — | The sustain jack (CC 64) |
| **Learn** | Press this, then the pedal: it takes that pedal's CC. Press again to stop waiting. | — | — | The sustain jack (CC 64) |
| **Try** | Runs the pedal's function now, as a press would. Sustain, Sostenuto and Soft switch on or off, and stay that way until you press Try again. Modulation and Pitch Bend follow the pedal, so there is nothing to try here. | — | — | — |
| **Reverse polarity** | For a pedal that works the wrong way round (nothing when pressed, something when let go). | Polarity | — | — |
| **Hold A** | On while the pedal is held, off when it is let go: how a sustain pedal works. | Control Type: Hold A | — | — |
| **Hold B** | Off while the pedal is held, on when it is up: picking it with the pedal up turns the function on at once. | Control Type: Hold B | — | — |
| **Toggle** | Each press switches it on or off. | Control Type: Toggle | — | — |
| **Bend up** | An expression pedal bends the pitch up: heel down is no bend, toe down the full Pitch Bend Range. | Range: Upper | — | — |
| **Bend down** | An expression pedal bends the pitch down: heel down is no bend, toe down the full range down. | Range: Lower | — | — |
| **Bend both ways** | The pedal sweeps the whole bend: heel down is fully down, the middle no bend, toe down fully up. | Range: Full | — | — |
| **Sustain on this part** | Whether the sustain pedal (and Sostenuto and Soft) reach this part. A part only takes it while it is on. | Sustain (part settings) | — | — |
| **Pitch bend on this part** | Whether the pitch-bend wheel (or a Pitch Bend pedal) bends this part. | Joystick (X): Pitch Bend | — | The pitch wheel |
| **Modulation on this part** | Whether the modulation wheel adds vibrato to this part. By default Right 1–3 take it and Left doesn't. | Joystick (Y): Modulation | — | The modulation wheel |
| **Pitch Bend Range down** | One semitone less bend for this part (0–12; 2 is the default). | Pitch Bend Range | — | — |
| **Pitch Bend Range up** | One semitone more bend for this part (0–12). | Pitch Bend Range | — | — |

## App

| control | what it does | Genos | key | Launchkey |
|---|---|---|---|---|
| **Help mode** | Grows the help footer to show the whole entry, and keeps the last control you hovered or tabbed to there while you try it. Controls keep working. | — | `?` | — |
| **Pop-up tips** | Also shows each entry in a pop-up next to the control, as well as in the help footer. Off by default, because a pop-up covers the controls while you play. | — | — | — |
| **Light / dark** | Switches between the dark stage theme and a light one. | — | — | — |
