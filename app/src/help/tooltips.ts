// The tooltip catalog: the single source of truth for what every control in the app does,
// its Genos name, its keyboard shortcut and where it lives on the Launchkey.
//
// - Every interactive element in the UI carries `data-tip="<key>"` (via `use:tip`), and a
//   test fails if one doesn't or if the key isn't in this catalog.
// - `keys` are written the way README's "Terminal keys" list writes them (`space`,
//   `shift+1`, `F10`, `←`), and a test checks them against README both ways.
// - Pad locations use `padLocation()`'s wording, and a test checks each pad's entry names
//   the pad it's on.
// - `npm run docs:controls` renders this catalog to `app/docs/controls.md`.

export interface Tip {
  /** The control's name, as the app labels it. */
  title: string
  /** What it does, in one to three plain sentences. */
  body: string
  /** The Genos name, when it has one (null: yahaha-only, or the same as the title). */
  genos: string | null
  /** Keyboard shortcuts, README notation. Empty: no shortcut. */
  keys: string[]
  /** Where it is on the Launchkey. null: not on the Launchkey. */
  launchkey: string | null
  /**
   * Keys the app uses instead of `keys`, where the terminal's key would break the app's
   * keyboard navigation (Tab moves focus in the app, so pad pages are PgUp/PgDn here).
   */
  app_keys?: string[]
}

const P1 = 'Pad page 1 (Sections)'
const P2 = 'Pad page 2 (Chord/Setup)'
const P3 = 'Pad page 3 (OTS/Parts)'
const pad = (page: string, row: 'top' | 'bottom', n: number) => `${page}, ${row} row, pad ${n}`

const catalog = {
  // ── Transport ───────────────────────────────────────────────────────────
  'transport.start_stop': {
    title: 'Start / Stop',
    body: 'Starts the band right away, or stops it. Starting from a stop plays the armed Intro first, if you picked one.',
    genos: 'START/STOP',
    keys: ['space'],
    launchkey: `${pad(P1, 'bottom', 8)}; Play button`,
  },
  'transport.stop': {
    title: 'Stop',
    body: 'Stops the band. Unlike Start / Stop it never starts it.',
    genos: 'START/STOP (stop)',
    keys: [],
    launchkey: 'Stop button',
  },
  'transport.sync_start': {
    title: 'Sync Start',
    body: 'Arms the band to start on your first left-hand chord. The lamp pulses while it waits. Pressing it while the band plays stops the band and re-arms.',
    genos: 'SYNC START',
    keys: ['y'],
    launchkey: pad(P1, 'top', 4),
  },
  'transport.sync_stop': {
    title: 'Sync Stop',
    body: 'The band plays only while you hold a chord: let go of every chord key and it stops, play again and it restarts. Not available with the Full Keyboard fingerings.',
    genos: 'SYNC STOP',
    keys: ['j'],
    launchkey: pad(P1, 'bottom', 7),
  },
  'transport.auto_fill': {
    title: 'Auto Fill',
    body: 'When on, switching to another Main plays a fill into it first.',
    genos: 'AUTO FILL IN',
    keys: ['u'],
    launchkey: pad(P1, 'top', 8),
  },
  'transport.stop_acmp': {
    title: 'Stop ACMP',
    body: 'With Sync Start off and the band stopped, a chord you hold sounds on the style\'s bass and pad voices.',
    genos: 'Stop Accompaniment',
    keys: ['h'],
    launchkey: pad(P2, 'bottom', 2),
  },
  'transport.panic': {
    title: 'Panic',
    body: 'Sends all notes off on every part, for a stuck note.',
    genos: null,
    keys: ['\\'],
    launchkey: null,
  },

  // ── Sections ────────────────────────────────────────────────────────────
  'section.intro1': {
    title: 'Intro I',
    body: 'Stopped: arms Intro I to play when the band starts (the lamp pulses). Playing: queues it for the next bar.',
    genos: 'INTRO I',
    keys: ['q'],
    launchkey: pad(P1, 'top', 1),
  },
  'section.intro2': {
    title: 'Intro II',
    body: 'Stopped: arms Intro II to play when the band starts (the lamp pulses). Playing: queues it for the next bar.',
    genos: 'INTRO II',
    keys: ['w'],
    launchkey: pad(P1, 'top', 2),
  },
  'section.intro3': {
    title: 'Intro III',
    body: 'Stopped: arms Intro III to play when the band starts (the lamp pulses). Playing: queues it for the next bar. Dark if the style has none.',
    genos: 'INTRO III',
    keys: ['e'],
    launchkey: pad(P1, 'top', 3),
  },
  'section.main_a': {
    title: 'Main A',
    body: 'Switches to Main A at the next bar; the lamp flashes while queued. Press the Main that is playing to play its fill, which also flashes.',
    genos: 'MAIN VARIATION A',
    keys: ['1'],
    launchkey: pad(P1, 'bottom', 1),
  },
  'section.main_b': {
    title: 'Main B',
    body: 'Switches to Main B at the next bar; the lamp flashes while queued. Press the Main that is playing to play its fill, which also flashes.',
    genos: 'MAIN VARIATION B',
    keys: ['2'],
    launchkey: pad(P1, 'bottom', 2),
  },
  'section.main_c': {
    title: 'Main C',
    body: 'Switches to Main C at the next bar; the lamp flashes while queued. Press the Main that is playing to play its fill, which also flashes.',
    genos: 'MAIN VARIATION C',
    keys: ['3'],
    launchkey: pad(P1, 'bottom', 3),
  },
  'section.main_d': {
    title: 'Main D',
    body: 'Switches to Main D at the next bar; the lamp flashes while queued. Press the Main that is playing to play its fill, which also flashes.',
    genos: 'MAIN VARIATION D',
    keys: ['4'],
    launchkey: pad(P1, 'bottom', 4),
  },
  'section.break': {
    title: 'Break',
    body: 'Plays the break from the next beat to the end of the bar, then goes back to the Main. The lamp flashes while it waits for the beat.',
    genos: 'BREAK',
    keys: ['g'],
    launchkey: pad(P1, 'bottom', 5),
  },
  'section.ending1': {
    title: 'Ending I',
    body: 'Plays Ending I from the next bar, then stops the band.',
    genos: 'ENDING/rit. I',
    keys: ['i'],
    launchkey: pad(P1, 'top', 5),
  },
  'section.ending2': {
    title: 'Ending II',
    body: 'Plays Ending II from the next bar, then stops the band.',
    genos: 'ENDING/rit. II',
    keys: ['o'],
    launchkey: pad(P1, 'top', 6),
  },
  'section.ending3': {
    title: 'Ending III',
    body: 'Plays Ending III from the next bar, then stops the band. Dark if the style has none.',
    genos: 'ENDING/rit. III',
    keys: ['p'],
    launchkey: pad(P1, 'top', 7),
  },
  'section.lamps': {
    title: 'Section lamps',
    body: 'Dim: available. Bright: playing, or on. Flashing: queued for the next bar (fills: the next beat). Pulsing: armed and waiting for you. Dark: this style doesn\'t have it.',
    genos: 'Section lamp states',
    keys: [],
    launchkey: 'The pads light the same way, in the same colours',
  },

  // ── Tempo and display ───────────────────────────────────────────────────
  'tempo.tap': {
    title: 'Tap tempo',
    body: 'Tap two or more times in time to set the tempo from your taps. The pad lights on the downbeat while the band plays.',
    genos: 'TAP TEMPO',
    keys: ['t'],
    launchkey: pad(P1, 'bottom', 6),
  },
  'tempo.down': {
    title: 'Tempo −',
    body: 'Slows the tempo by 1 BPM.',
    genos: 'TEMPO −',
    keys: ['-'],
    launchkey: 'Function button (right of the pads)',
  },
  'tempo.up': {
    title: 'Tempo +',
    body: 'Speeds the tempo up by 1 BPM.',
    genos: 'TEMPO +',
    keys: ['='],
    launchkey: '> (Scene Launch) button (right of the pads)',
  },
  'display.tempo': {
    title: 'Tempo',
    body: 'The current tempo in beats per minute. Loading a style sets the style\'s own tempo.',
    genos: 'Tempo',
    keys: [],
    launchkey: null,
  },
  'display.timesig': {
    title: 'Time signature',
    body: 'The style\'s time signature, from the style file.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'display.position': {
    title: 'Bar and beat',
    body: 'Where the band is: bar, and a light per beat. The section playing now and the one queued next are shown alongside.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'display.chord': {
    title: 'Chord',
    body: 'The chord the style is following. When Keyboard transpose is not zero, the chord as you fingered it is shown small underneath.',
    genos: 'Chord (Home display, Style area)',
    keys: [],
    launchkey: null,
  },
  'display.status': {
    title: 'Status line',
    body: 'The last message: a loaded style, an error, or a Launchkey note or CC nothing is mapped to, which tells you what a button really sends.',
    genos: null,
    keys: [],
    launchkey: null,
  },

  // ── Style ───────────────────────────────────────────────────────────────
  'style.name': {
    title: 'Style',
    body: 'The style that\'s loaded, and its folder. Click to browse the library.',
    genos: 'Style name',
    keys: ['enter'],
    launchkey: null,
  },
  'style.prev': {
    title: 'Previous style',
    body: 'Loads the previous style in the browser\'s order (folder, then name). While the band plays it keeps playing and follows your next chord in the new style.',
    genos: null,
    keys: ['←'],
    launchkey: '< Track button',
  },
  'style.next': {
    title: 'Next style',
    body: 'Loads the next style in the browser\'s order (folder, then name). While the band plays it keeps playing and follows your next chord in the new style.',
    genos: null,
    keys: ['→'],
    launchkey: 'Track > button',
  },

  // ── Style browser ───────────────────────────────────────────────────────
  'browser.open': {
    title: 'Browse styles',
    body: 'Opens the style browser: every style under the library folder, by folder. Your keyboard and the Launchkey keep playing while it\'s open.',
    genos: 'Style selection display',
    keys: ['enter'],
    launchkey: null,
  },
  'browser.filter': {
    title: 'Filter',
    body: 'Type to filter by style name, file name or folder (not case-sensitive). ↑/↓, PgUp/PgDn and Home/End move; Enter loads; Shift+Enter previews, or queues for the next bar while the band plays.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'browser.all': {
    title: 'All styles',
    body: 'Every style in the library, in the order < Track and Track > step through: folder, then name.',
    genos: null,
    keys: [],
    launchkey: '< Track and Track > step through this order',
  },
  'browser.folder': {
    title: 'Folder',
    body: 'Shows only the styles in this folder and its subfolders. The folder a style is in is its category.',
    genos: 'Style category',
    keys: [],
    launchkey: null,
  },
  'browser.favourites': {
    title: 'Favourites',
    body: 'The styles you starred, in library order. Favourites are remembered on this computer.',
    genos: 'Favorite',
    keys: [],
    launchkey: null,
  },
  'browser.recents': {
    title: 'Recent',
    body: 'The styles you loaded lately, newest first, however you loaded them. Remembered on this computer.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'browser.row': {
    title: 'Load style',
    body: 'Click or Enter loads this style and closes the browser. While the band plays it keeps playing and follows your next chord in the new style. ▶ marks the loaded style; ‹ and › mark the ones < Track and Track > would load.',
    genos: null,
    keys: [],
    launchkey: '< Track / Track > load the neighbouring rows',
  },
  'browser.row_error': {
    title: 'Unreadable style',
    body: 'This file couldn\'t be read as a style, so it can\'t be loaded and < Track / Track > skip it. The reason is shown on the row.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'browser.favourite': {
    title: 'Favourite',
    body: 'Stars or unstars this style, adding it to Favourites. Ctrl+D does the same for the highlighted row.',
    genos: 'Favorite',
    keys: [],
    launchkey: null,
  },
  'browser.preview': {
    title: 'Preview',
    body: 'While the band is stopped, plays a few bars of this style over a short default progression without loading it. Shift+Enter previews the highlighted row.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'browser.preview_stop': {
    title: 'Stop preview',
    body: 'Stops the preview at once. Loading a style, starting the band or closing the browser stops it too.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'browser.queue': {
    title: 'Load at next bar',
    body: 'While the band plays, loads this style on the next bar line instead of right away, so the change lands on the beat. Shift+Enter queues the highlighted row.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'browser.auto_preview': {
    title: 'Preview on select',
    body: 'When on and the band is stopped, the highlighted or hovered row previews after a short pause. Remembered on this computer.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'browser.close': {
    title: 'Close browser',
    body: 'Closes the browser without changing the style, and stops any preview.',
    genos: null,
    keys: ['esc'],
    launchkey: null,
  },

  // ── Chord detection ─────────────────────────────────────────────────────
  'fingering.select': {
    title: 'Fingering type',
    body: 'How your left hand\'s chords are read. The pads on page 2 pick one directly.',
    genos: 'Fingering Type',
    keys: ['f'],
    launchkey: `${P2}, top row, pads 1–7`,
  },
  'fingering.next': {
    title: 'Next fingering type',
    body: 'Steps to the next fingering type.',
    genos: 'Fingering Type',
    keys: ['f'],
    launchkey: null,
  },
  'fingering.single_finger': {
    title: 'Single Finger',
    body: 'One key plays a major chord. Add a black key to its left for minor, a white key for 7th, both for m7.',
    genos: 'Single Finger',
    keys: [],
    launchkey: pad(P2, 'top', 1),
  },
  'fingering.fingered': {
    title: 'Fingered',
    body: 'Play the whole chord. The bass is always the chord\'s root.',
    genos: 'Fingered',
    keys: [],
    launchkey: pad(P2, 'top', 2),
  },
  'fingering.fingered_on_bass': {
    title: 'Fingered On Bass',
    body: 'Like Fingered, but the lowest note you play becomes the bass, so you can play slash chords.',
    genos: 'Fingered On Bass',
    keys: [],
    launchkey: pad(P2, 'top', 3),
  },
  'fingering.multi_finger': {
    title: 'Multi Finger',
    body: 'Reads Single Finger and Fingered shapes both, without switching.',
    genos: 'Multi Finger',
    keys: [],
    launchkey: pad(P2, 'top', 4),
  },
  'fingering.ai_fingered': {
    title: 'AI Fingered',
    body: 'Like Fingered, but fewer than three keys can still give a chord, guessed from the chord before.',
    genos: 'AI Fingered',
    keys: [],
    launchkey: pad(P2, 'top', 5),
  },
  'fingering.full_keyboard': {
    title: 'Full Keyboard',
    body: 'Chords are read across the whole keyboard, even split between your hands.',
    genos: 'Full Keyboard',
    keys: [],
    launchkey: pad(P2, 'top', 6),
  },
  'fingering.ai_full_keyboard': {
    title: 'AI Full Keyboard',
    body: 'Full Keyboard with AI Fingered\'s guessing from fewer keys. 9th, 11th and 13th chords can\'t be played.',
    genos: 'AI Full Keyboard',
    keys: [],
    launchkey: pad(P2, 'top', 7),
  },
  'detection.upper': {
    title: 'Chord detection: Upper / Lower',
    body: 'Lower: your left hand plays the chords. Upper: your right hand does (as Fingered*), and your left hand is free for a bass line.',
    genos: 'Chord Detection Area',
    keys: ['d'],
    launchkey: pad(P2, 'top', 8),
  },
  'detection.manual_bass': {
    title: 'Manual Bass',
    body: 'In Upper, mutes the style\'s Bass and gives its voice to Left, so your left hand plays the bass. Left stays on while it\'s on. Dark in Lower, where it isn\'t available.',
    genos: 'Manual Bass',
    keys: ['D'],
    launchkey: pad(P2, 'bottom', 1),
  },
  'split.display': {
    title: 'Split point',
    body: 'The key that divides the chord section from the right hand (C3 = middle C). Right 1–3 play above it, Left and the chord section at and below it.',
    genos: 'Split Point (Style + Left)',
    keys: [],
    launchkey: null,
  },
  'split.down': {
    title: 'Split −',
    body: 'Moves the split point down one key.',
    genos: 'Split Point',
    keys: ['['],
    launchkey: pad(P2, 'bottom', 3),
  },
  'split.up': {
    title: 'Split +',
    body: 'Moves the split point up one key.',
    genos: 'Split Point',
    keys: [']'],
    launchkey: pad(P2, 'bottom', 4),
  },

  // ── Transpose ───────────────────────────────────────────────────────────
  'transpose.display': {
    title: 'Transpose',
    body: 'Keyboard transpose moves your keys and the chord the style follows. Master transpose moves everything that sounds, drums excepted.',
    genos: 'TRANSPOSE',
    keys: [],
    launchkey: null,
  },
  'transpose.keyboard_down': {
    title: 'Keyboard transpose −',
    body: 'Moves your keys and the chord the style follows down a semitone. The pad lights while it\'s below zero.',
    genos: 'TRANSPOSE − (Keyboard)',
    keys: [';'],
    launchkey: pad(P2, 'bottom', 5),
  },
  'transpose.keyboard_up': {
    title: 'Keyboard transpose +',
    body: 'Moves your keys and the chord the style follows up a semitone. The pad lights while it\'s above zero.',
    genos: 'TRANSPOSE + (Keyboard)',
    keys: ["'"],
    launchkey: pad(P2, 'bottom', 6),
  },
  'transpose.master_down': {
    title: 'Master transpose −',
    body: 'Moves everything that sounds down a semitone, the band included (not the drums).',
    genos: 'TRANSPOSE − (Master)',
    keys: [':'],
    launchkey: null,
  },
  'transpose.master_up': {
    title: 'Master transpose +',
    body: 'Moves everything that sounds up a semitone, the band included (not the drums).',
    genos: 'TRANSPOSE + (Master)',
    keys: ['"'],
    launchkey: null,
  },
  'transpose.reset': {
    title: 'Transpose reset',
    body: 'Puts Keyboard and Master transpose back to 0. The pad lights while either isn\'t 0.',
    genos: 'TRANSPOSE − and + together',
    keys: ['/'],
    launchkey: pad(P2, 'bottom', 7),
  },

  // ── One Touch Settings ──────────────────────────────────────────────────
  'ots.1': {
    title: 'OTS 1',
    body: 'A sound setup for your own hands that the style\'s author picked to suit it: the voice, on/off, volume and octave of Right 1–3 and Left. Pressing it swaps your keyboard sounds, and the band doesn\'t change. Dark if the style has none.',
    genos: 'ONE TOUCH SETTING 1',
    keys: ['shift+1'],
    launchkey: pad(P3, 'top', 1),
  },
  'ots.2': {
    title: 'OTS 2',
    body: 'The style\'s second suggested setup for your hands: the voice, on/off, volume and octave of Right 1–3 and Left. Pressing it swaps your keyboard sounds, and the band doesn\'t change.',
    genos: 'ONE TOUCH SETTING 2',
    keys: ['shift+2'],
    launchkey: pad(P3, 'top', 2),
  },
  'ots.3': {
    title: 'OTS 3',
    body: 'The style\'s third suggested setup for your hands: the voice, on/off, volume and octave of Right 1–3 and Left. Pressing it swaps your keyboard sounds, and the band doesn\'t change.',
    genos: 'ONE TOUCH SETTING 3',
    keys: ['shift+3'],
    launchkey: pad(P3, 'top', 3),
  },
  'ots.4': {
    title: 'OTS 4',
    body: 'The style\'s fourth suggested setup for your hands: the voice, on/off, volume and octave of Right 1–3 and Left. Pressing it swaps your keyboard sounds, and the band doesn\'t change.',
    genos: 'ONE TOUCH SETTING 4',
    keys: ['shift+4'],
    launchkey: pad(P3, 'top', 4),
  },
  'ots.link': {
    title: 'OTS Link',
    body: 'When on, your hands\' sounds follow the band: pressing Main A, B, C or D also recalls OTS 1, 2, 3 or 4. Changing style recalls the setting for the Main that\'s playing.',
    genos: 'OTS LINK',
    keys: ['F10'],
    launchkey: `${pad(P3, 'top', 5)}; Shift + Pad Bank ▼`,
  },

  // ── Keyboard parts ──────────────────────────────────────────────────────
  'part.right1.on': {
    title: 'Right 1 on/off',
    body: 'Turns Right 1 on or off. Right parts that are on sound together, which is how you layer voices.',
    genos: 'PART ON/OFF RIGHT 1',
    keys: ['5'],
    launchkey: `${pad(P3, 'bottom', 1)}; Panel fader page: button under fader 1`,
  },
  'part.right2.on': {
    title: 'Right 2 on/off',
    body: 'Turns Right 2 on or off. Turn on Right 1 and Right 2 together to layer, for example piano and strings.',
    genos: 'PART ON/OFF RIGHT 2',
    keys: ['6'],
    launchkey: `${pad(P3, 'bottom', 2)}; Panel fader page: button under fader 2`,
  },
  'part.right3.on': {
    title: 'Right 3 on/off',
    body: 'Turns Right 3 on or off, a third layer for the right hand.',
    genos: 'PART ON/OFF RIGHT 3',
    keys: ['7'],
    launchkey: `${pad(P3, 'bottom', 3)}; Panel fader page: button under fader 3`,
  },
  'part.left.on': {
    title: 'Left on/off',
    body: 'Turns the Left voice on or off: your left hand plays it below the split. It can\'t be turned off while Manual Bass is on.',
    genos: 'PART ON/OFF LEFT',
    keys: ['8', 'l'],
    launchkey: `${pad(P3, 'bottom', 4)}; Panel fader page: button under fader 4; Shift + Pad Bank ▲`,
  },
  'part.right1.select': {
    title: 'Edit Right 1',
    body: 'Picks Right 1 as the part whose voice Voice −/+ changes.',
    genos: 'Part select (Right 1)',
    keys: ['F1'],
    launchkey: `${pad(P3, 'bottom', 5)}; Panel fader page: Shift + button under fader 1`,
  },
  'part.right2.select': {
    title: 'Edit Right 2',
    body: 'Picks Right 2 as the part whose voice Voice −/+ changes.',
    genos: 'Part select (Right 2)',
    keys: ['F2'],
    launchkey: `${pad(P3, 'bottom', 6)}; Panel fader page: Shift + button under fader 2`,
  },
  'part.right3.select': {
    title: 'Edit Right 3',
    body: 'Picks Right 3 as the part whose voice Voice −/+ changes.',
    genos: 'Part select (Right 3)',
    keys: ['F3'],
    launchkey: `${pad(P3, 'bottom', 7)}; Panel fader page: Shift + button under fader 3`,
  },
  'part.left.select': {
    title: 'Edit Left',
    body: 'Picks Left as the part whose voice Voice −/+ changes.',
    genos: 'Part select (Left)',
    keys: ['F4'],
    launchkey: `${pad(P3, 'bottom', 8)}; Panel fader page: Shift + button under fader 4`,
  },
  'part.voice_down': {
    title: 'Voice −',
    body: 'Steps the selected part (the lit Edit pad) to the previous voice.',
    genos: 'Voice select',
    keys: ['9'],
    launchkey: pad(P3, 'top', 7),
  },
  'part.voice_up': {
    title: 'Voice +',
    body: 'Steps the selected part (the lit Edit pad) to the next voice.',
    genos: 'Voice select',
    keys: ['0'],
    launchkey: pad(P3, 'top', 8),
  },
  'part.octave_down': {
    title: 'Octave −',
    body: 'Shifts this part down an octave (down to −2). OTS recalls set it too.',
    genos: 'Voice Setting → Tune → Octave',
    keys: [],
    launchkey: null,
  },
  'part.octave_up': {
    title: 'Octave +',
    body: 'Shifts this part up an octave (up to +2). OTS recalls set it too.',
    genos: 'Voice Setting → Tune → Octave',
    keys: [],
    launchkey: null,
  },

  // ── Keyboard parts drawer ──────────────────────────────────────────────
  'part.voice': {
    title: 'Voice',
    body: 'Picks this part\'s voice. Under Manual Bass, Left plays the style\'s Bass voice instead, and this is the voice it goes back to.',
    genos: 'Voice select (VOICE buttons)',
    keys: [],
    launchkey: 'Pad page 3 (OTS/Parts): the Edit pads (bottom row, pads 5–8) pick the part, Voice −/+ (top row, pads 7–8) step its voice',
  },
  'part.layer': {
    title: 'Layer',
    body: 'The Right parts that are on all sound together on every key above the split: that\'s a layer. Turn on Right 1 and Right 2 to stack, for example, piano and strings.',
    genos: 'PART ON/OFF (Right 1–3 layered)',
    keys: ['5', '6', '7'],
    launchkey: 'Panel fader page: buttons under faders 1–3',
  },
  'part.left_zone': {
    title: 'Left hand',
    body: 'Left plays the keys at and below the split point, with its own voice. Under Manual Bass it plays the style\'s Bass voice there instead, and the band\'s own bass goes quiet.',
    genos: 'Split Point (Left)',
    keys: [],
    launchkey: null,
  },
  'part.written_for': {
    title: 'As written for',
    body: 'The voice the style\'s author wrote this band part for, and its MIDI channel on the yahaha port. Load a matching instrument on that channel in Ableton to hear the style as intended; ≈ marks the nearest General MIDI voice to a Yamaha one.',
    genos: 'Style parts (Mixer › Style)',
    keys: [],
    launchkey: null,
  },
  'ots.link_timing': {
    title: 'OTS Link timing',
    body: 'When OTS Link swaps the setting: as soon as you press a Main button, not when the band reaches the new Main at the bar line. The Genos calls this Real Time.',
    genos: 'OTS Link Timing: Real Time',
    keys: [],
    launchkey: null,
  },

  // ── Mixer ───────────────────────────────────────────────────────────────
  'mixer.page': {
    title: 'Fader page: Panel / Style',
    body: 'Switches what the Launchkey faders control: Panel is your four keyboard parts, Style is the band\'s eight parts. The button lights blue on Panel, green on Style.',
    genos: 'Mixer tabs (Panel / Style)',
    keys: ['F9'],
    launchkey: 'Button under the master fader',
  },
  'mixer.panel.right1': {
    title: 'Right 1 volume',
    body: 'Right 1\'s volume. The fader is channel 1\'s CC 7 itself, with no hidden gain behind it.',
    genos: 'Mixer › Panel › Right 1 Volume',
    keys: [],
    launchkey: 'Panel fader page: fader 1',
  },
  'mixer.panel.right2': {
    title: 'Right 2 volume',
    body: 'Right 2\'s volume. The fader is channel 3\'s CC 7 itself, with no hidden gain behind it.',
    genos: 'Mixer › Panel › Right 2 Volume',
    keys: [],
    launchkey: 'Panel fader page: fader 2',
  },
  'mixer.panel.right3': {
    title: 'Right 3 volume',
    body: 'Right 3\'s volume. The fader is channel 4\'s CC 7 itself, with no hidden gain behind it.',
    genos: 'Mixer › Panel › Right 3 Volume',
    keys: [],
    launchkey: 'Panel fader page: fader 3',
  },
  'mixer.panel.left': {
    title: 'Left volume',
    body: 'Left\'s volume. The fader is channel 2\'s CC 7 itself, with no hidden gain behind it; under Manual Bass it is the bass\'s level too.',
    genos: 'Mixer › Panel › Left Volume',
    keys: [],
    launchkey: 'Panel fader page: fader 4',
  },
  'mixer.style.volume': {
    title: 'Style part volume',
    body: 'This band part\'s volume. The fader is its channel\'s CC 7 itself (channels 9–16), with no hidden gain behind it. Loading a style sets the faders to the style\'s own levels, and a pattern that changes its volume moves the fader too, until you move it yourself.',
    genos: 'Mixer › Style › Volume',
    keys: [],
    launchkey: 'Style fader page: faders 1–8 (Rhythm 1 … Phrase 2)',
  },
  'mixer.style.mute': {
    title: 'Style part on/off',
    body: 'Mutes or unmutes this band part. The Launchkey button is lit while the part plays.',
    genos: 'Channel On/Off',
    keys: ['z', 'x', 'c', 'v', 'b', 'n', 'm', ','],
    launchkey: 'Style fader page: buttons under faders 1–8',
  },
  'mixer.master': {
    title: 'Master volume',
    body: 'The built-in synth\'s output level (100 = unity), the only gain after the channel faders. A safety soft clipper above −1 dBFS keeps loud passages from hard clipping; below that the output is untouched. It does not change the MIDI output.',
    genos: 'MASTER VOLUME',
    keys: [],
    launchkey: 'Master fader (both fader pages)',
  },
  'mixer.solo': {
    title: 'Solo',
    body: 'Would play only this part. Not available yet: the engine has no solo (backlog #30), so the button is disabled.',
    genos: 'Mixer › touch and hold a channel (Solo)',
    keys: [],
    launchkey: null,
  },
  'mixer.channel': {
    title: 'MIDI out channel',
    body: 'The channel this part plays on at yahaha\'s MIDI output: Right 1 = 1, Left = 2, Right 2 = 3, Right 3 = 4, the band 9–16. Map a DAW track (Ableton: MIDI From yahaha, this channel) to record or re-voice it.',
    genos: 'Part / Style channel',
    keys: [],
    launchkey: null,
  },
  'mixer.voice': {
    title: 'Voice',
    body: 'Keyboard parts: the GM voice the part plays. Style parts: the Yamaha voice (bank MSB/LSB/program) the style was written for, and after ≈ the nearest voice the built-in synth plays for it.',
    genos: 'Mixer › Voice',
    keys: [],
    launchkey: null,
  },
  'mixer.info': {
    title: 'A fader is the channel\'s CC 7',
    body: 'Each fader shows and sends exactly its channel\'s CC 7 (0–127), with no hidden gain anywhere, so the MIDI output and the synth hear the same level. Loading a style sets the Style faders to the style\'s own levels.',
    genos: 'Mixer › Volume',
    keys: [],
    launchkey: 'The faders, on both fader pages',
  },
  'mixer.pickup': {
    title: 'Waiting for the fader',
    body: 'This level moved without the Launchkey fader (a style load, an OTS recall, a pattern, a page switch). The hardware fader does nothing until you move it to within 2 of the level, or across it.',
    genos: null,
    keys: [],
    launchkey: 'Soft takeover on every fader',
  },

  // ── Launchkey ───────────────────────────────────────────────────────────
  'padpage.sections': {
    title: 'Pad page 1: Sections',
    body: 'Intros, Mains, Break, Endings, Sync Start/Stop, Auto Fill, Tap and Start/Stop, each in its own colour.',
    genos: null,
    keys: ['tab', 'shift+tab'],
    app_keys: ['PgDn', 'PgUp'],
    launchkey: 'Pad Bank ▲ / ▼ (left of the pads)',
  },
  'padpage.chord_setup': {
    title: 'Pad page 2: Chord/Setup',
    body: 'Fingering types, Upper/Lower, Manual Bass, Stop ACMP, split and transpose. All cyan.',
    genos: null,
    keys: ['tab', 'shift+tab'],
    app_keys: ['PgDn', 'PgUp'],
    launchkey: 'Pad Bank ▲ / ▼ (left of the pads)',
  },
  'padpage.ots_parts': {
    title: 'Pad page 3: OTS/Parts',
    body: 'OTS 1–4 and OTS Link, voice −/+, keyboard parts on/off and which part to edit. All magenta.',
    genos: null,
    keys: ['tab', 'shift+tab'],
    app_keys: ['PgDn', 'PgUp'],
    launchkey: 'Pad Bank ▲ / ▼ (left of the pads)',
  },
  'padpage.prev': {
    title: 'Pad Bank ▲',
    body: 'Goes to the previous pad page, stopping at page 1, so a few presses always take you home. Lit in the page\'s colour when there\'s a page to go to. With Shift: Left on/off.',
    genos: null,
    keys: ['shift+tab'],
    app_keys: ['PgUp'],
    launchkey: 'Pad Bank ▲ (left of the pads)',
  },
  'padpage.next': {
    title: 'Pad Bank ▼',
    body: 'Goes to the next pad page, stopping at page 3. Lit in the page\'s colour when there\'s a page to go to. With Shift: OTS Link on/off.',
    genos: null,
    keys: ['tab'],
    app_keys: ['PgDn'],
    launchkey: 'Pad Bank ▼ (left of the pads)',
  },
  'launchkey.shift': {
    title: 'Shift',
    body: 'Hold for the second functions: Pad Bank ▲ = Left on/off, Pad Bank ▼ = OTS Link, the buttons under faders 1–4 on the Panel page = edit that part. On screen, click it to latch the Shift layer, or hold Shift on your computer keyboard.',
    genos: null,
    keys: [],
    launchkey: 'Shift button',
  },
  'launchkey.status': {
    title: 'Launchkey',
    body: 'Whether the Launchkey is connected in DAW mode, so its pads and buttons are arranger controls.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'launchkey.fader_unused': {
    title: 'Unused fader',
    body: 'On the Panel fader page, faders 5–8 and their buttons do nothing. Switch to the Style page (the button under the master fader) to mix the band.',
    genos: null,
    keys: [],
    launchkey: 'Panel fader page: faders 5–8 and the buttons under them',
  },
  'launchkey.unused': {
    title: 'Unused pad',
    body: 'This pad does nothing on this page and stays dark.',
    genos: null,
    keys: [],
    launchkey: null,
  },

  // ── Settings and audio ──────────────────────────────────────────────────
  'settings.open': {
    title: 'Settings',
    body: 'Chord detection, split, transpose, style behaviour, audio, MIDI and the style library. Changes apply at once.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'settings.close': {
    title: 'Close settings',
    body: 'Closes the settings panel.',
    genos: null,
    keys: ['esc'],
    launchkey: null,
  },
  'audio.output': {
    title: 'Audio output',
    body: 'The output pair the built-in synth plays on. `--audio-out N` sets it at launch.',
    genos: null,
    keys: ['a'],
    launchkey: null,
  },
  'audio.soundfont': {
    title: 'SoundFont',
    body: 'The General MIDI SoundFont (.sf2) the built-in synth plays, from the soundfonts folder. `--sf2 file` sets it at launch.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'audio.synth_mute': {
    title: 'Mute synth',
    body: 'Silences the built-in synth, for when you play Ableton\'s sounds from the yahaha MIDI port instead.',
    genos: null,
    keys: ['k'],
    launchkey: null,
  },
  'midi.input': {
    title: 'MIDI input',
    body: 'Whether this MIDI source plays yahaha. The Launchkey\'s DAW port carries its pads and buttons.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'settings.tab.chord': {
    title: 'Settings: Chord',
    body: 'Fingering type, chord detection area (Lower or Upper) and Manual Bass.',
    genos: 'Menu › Split & Fingering',
    keys: [],
    launchkey: `${P2}, top row (fingering, Upper) and bottom row, pad 1 (Manual Bass)`,
  },
  'settings.tab.split': {
    title: 'Settings: Split point',
    body: 'Where the keyboard divides between the chord section and the right hand.',
    genos: 'Menu › Split & Fingering › Split Point',
    keys: [],
    launchkey: `${P2}, bottom row, pads 3–4`,
  },
  'settings.tab.transpose': {
    title: 'Settings: Transpose',
    body: 'Keyboard and Master transpose, in semitones.',
    genos: 'Menu › Transpose',
    keys: [],
    launchkey: `${P2}, bottom row, pads 5–7`,
  },
  'settings.tab.style': {
    title: 'Settings: Style',
    body: 'How the band starts, stops and fills: Sync Start/Stop, Auto Fill and Stop Accompaniment.',
    genos: 'Menu › Style Setting',
    keys: [],
    launchkey: `${P1} (Sync Start, Sync Stop, Auto Fill); ${P2}, bottom row, pad 2 (Stop ACMP)`,
  },
  'settings.tab.audio': {
    title: 'Settings: Audio',
    body: 'The built-in synth: on or off, which output pair it plays on, its SoundFont and its master volume.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'settings.tab.midi': {
    title: 'Settings: MIDI',
    body: 'Which MIDI inputs play yahaha, the yahaha output port, and the Launchkey connection.',
    genos: 'Menu › MIDI',
    keys: [],
    launchkey: null,
  },
  'settings.tab.library': {
    title: 'Settings: Library',
    body: 'The folders yahaha looks for style files in, and a rescan.',
    genos: 'Style selection (USB / User folders)',
    keys: [],
    launchkey: null,
  },
  'settings.split_strip': {
    title: 'Split point',
    body: 'Drag the marker, or click a key, to set the split (C3 = middle C). Keys at and below it are Left and the chord section, keys above it play Right 1–3. With focus, ←/→ move it a key and PgUp/PgDn an octave.',
    genos: 'Split Point (Style + Left)',
    keys: ['[', ']'],
    launchkey: `${P2}, bottom row, pads 3–4`,
  },
  'settings.section_timing': {
    title: 'Section change timing (coming soon)',
    body: 'When a Main you press takes over: Immediate (at the next beat) or Next Bar. Coming in M5; for now a new Main always waits for the next bar line (with Auto Fill, its fill starts at the next beat).',
    genos: 'Section Change Timing',
    keys: [],
    launchkey: null,
  },
  'settings.ots_link_timing': {
    title: 'OTS Link timing (coming soon)',
    body: 'With OTS Link on, whether the One Touch Setting changes the moment you press a Main (Immediate) or when the new Main actually starts (At Main Section Change). Coming in M5; for now it is always Immediate.',
    genos: 'OTS Link Timing',
    keys: [],
    launchkey: null,
  },
  'settings.synchro_stop_window': {
    title: 'Synchro Stop window (coming soon)',
    body: 'With Sync Stop on: hold a chord longer than this and Sync Stop cancels itself, so the style keeps playing when you let go. A quicker release still stops the style. Coming in M5.',
    genos: 'Synchro Stop Window',
    keys: [],
    launchkey: null,
  },
  'audio.synth_on': {
    title: 'Built-in synth',
    body: 'Turns the built-in SoundFont synth\'s sound on or off. The yahaha MIDI port keeps playing either way, for Ableton or other sounds.',
    genos: null,
    keys: ['k'],
    launchkey: null,
  },
  'midi.merge_all': {
    title: 'Inputs: all or selected',
    body: 'All: every MIDI source plays yahaha, merged. Selected: only the sources you switch on below. `--all-inputs` and `--input name` set it at launch.',
    genos: null,
    keys: [],
    launchkey: null,
  },
  'midi.output': {
    title: 'yahaha MIDI output',
    body: 'The virtual MIDI port yahaha plays the band and your parts on. Pick it as a MIDI input in Ableton to use your own sounds.',
    genos: 'MIDI Transmit',
    keys: [],
    launchkey: null,
  },
  'midi.palette_leds': {
    title: 'Palette LEDs',
    body: 'Lights the Launchkey with its built-in palette colours and hardware flashing instead of exact RGB colours. Try it if the pads look wrong or lag. `--palette-leds` sets it at launch.',
    genos: null,
    keys: [],
    launchkey: 'Every pad and button light',
  },
  'settings.style_folders': {
    title: 'Style folders',
    body: 'The folders yahaha reads style files from (.sty, .prs, .sst and more), with subfolders as categories. Pass them on the command line or set YAHAHA_STYLES.',
    genos: 'Style selection (User / USB)',
    keys: [],
    launchkey: null,
  },
  'settings.rescan': {
    title: 'Rescan styles',
    body: 'Reads the style folders again, picking up files you added, changed or removed. The band keeps playing.',
    genos: null,
    keys: [],
    launchkey: null,
  },

  // ── Drawers around the hardware view ────────────────────────────────────
  'drawer.parts': {
    title: 'Keyboard parts and OTS',
    body: 'Opens the detail of Right 1–3 and Left (voice, volume, octave, on/off) and the style\'s One Touch Settings.',
    genos: 'PART ON/OFF, Voice Setting, ONE TOUCH SETTING',
    keys: [],
    launchkey: 'Pad page 3 (OTS/Parts) has the same controls',
  },
  'drawer.mixer': {
    title: 'Mixer',
    body: 'Opens the full mixer: both fader pages side by side, with each band part\'s voice.',
    genos: 'Mixer (Panel / Style tabs)',
    keys: [],
    launchkey: 'The faders and the buttons under them',
  },
  'drawer.close': {
    title: 'Close',
    body: 'Closes this panel. The band keeps playing.',
    genos: null,
    keys: ['esc'],
    launchkey: null,
  },

  // ── App ─────────────────────────────────────────────────────────────────
  'app.help': {
    title: 'Help mode',
    body: 'Pins help on: hover or tab to any control and its full description stays in the help bar at the bottom. Controls keep working.',
    genos: null,
    keys: ['?'],
    launchkey: null,
  },
  'app.theme': {
    title: 'Light / dark',
    body: 'Switches between the dark stage theme and a light one.',
    genos: null,
    keys: [],
    launchkey: null,
  },
} satisfies Record<string, Tip>

export type TipKey = keyof typeof catalog
export const TIPS: Record<TipKey, Tip> = catalog

export function isTipKey(k: string): k is TipKey {
  return Object.prototype.hasOwnProperty.call(catalog, k)
}

const KEY_NAMES: Record<string, string> = {
  space: 'Space', enter: 'Enter', esc: 'Esc', tab: 'Tab', 'shift+tab': 'Shift+Tab', '\\': '\\',
}

/** A key as a keycap shows it: `shift+1` → `Shift+1`, `D` → `Shift+D`. */
export function keyLabel(k: string): string {
  if (KEY_NAMES[k]) return KEY_NAMES[k]
  if (k.startsWith('shift+')) return 'Shift+' + k.slice(6).toUpperCase()
  if (/^[A-Z]$/.test(k)) return 'Shift+' + k
  return k.length === 1 ? k.toUpperCase() : k
}
