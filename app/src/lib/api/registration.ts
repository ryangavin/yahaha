// Registration Memory and the Playlist (docs/app-api.md "Registration Memory",
// "Playlist"; docs/registration.md). Mirrors src/api/registration.rs and
// src/api/playlist.rs field for field.

/** A Genos Freeze / Memorize group. */
export type RegistGroup =
  | 'style' | 'voice' | 'harmonyArp' | 'multiPad' | 'tempo' | 'transpose' | 'chordLooper' | 'liveControl' | 'assignable'

/** What the Registration Sequence does past its last step. */
export type SequenceEnd = 'stop' | 'top' | 'next'

export type PlaylistSort = 'normal' | 'aToZ' | 'zToA'

/** A Playlist record: a bank file (and the button to recall after loading it), or a style. */
export type PlaylistRecord =
  | { name: string; kind: 'bank'; path: string; regist?: number | null }
  | { name: string; kind: 'style'; path: string }

/** Registration Memory commands. Buttons are 0-based (0–9 = the panel's [1]–[10]). */
export type RegistrationCmd =
  /** A REGISTRATION MEMORY button: recall, or memorize while MEMORY is armed. */
  | { type: 'pressRegist'; index: number }
  | { type: 'recallRegist'; index: number }
  | { type: 'memorizeRegist'; index: number }
  /** The MEMORY button: the next button press memorizes. */
  | { type: 'toggleRegistMemory' }
  | { type: 'setMemorizeGroup'; group: RegistGroup; on: boolean }
  | { type: 'clearRegist'; index: number }
  | { type: 'renameRegist'; index: number; name: string }
  /** REGIST BANK −/+. */
  | { type: 'stepRegistBank'; delta: number }
  | { type: 'selectRegistBank'; path: string }
  | { type: 'newRegistBank' }
  /** Save to its file (`name` null) or as a file of that name in the folder; another bank's file of that name is only replaced with `overwrite`. */
  | { type: 'saveRegistBank'; name: string | null; overwrite?: boolean }
  | { type: 'setFreeze'; on: boolean }
  | { type: 'toggleFreeze' }
  | { type: 'setFreezeGroup'; group: RegistGroup; on: boolean }
  | { type: 'setRegistSequence'; steps: number[]; end: SequenceEnd }
  | { type: 'setRegistSequenceOn'; on: boolean }
  | { type: 'toggleRegistSequence' }
  /** Regist +/−: the sequence's next/previous step. */
  | { type: 'stepRegistSequence'; delta: number }
  /** Regist +/− from a pedal: the sequence while it is on and programmed, else the bank's
   * next/previous stored button (it stops at either end). */
  | { type: 'stepRegist'; delta: number }

/** Playlist commands. Record `index` is the position in the file (`PlaylistRow.index`). */
export type PlaylistCmd =
  | { type: 'newPlaylist' }
  | { type: 'loadPlaylist'; path: string }
  | { type: 'savePlaylist'; name: string | null; overwrite?: boolean }
  | { type: 'addPlaylistRecord'; record: PlaylistRecord }
  | { type: 'addCurrentBank' }
  | { type: 'addCurrentStyle' }
  | { type: 'appendPlaylist'; path: string }
  | { type: 'setPlaylistRecord'; index: number; record: PlaylistRecord }
  | { type: 'movePlaylistRecord'; index: number; delta: number }
  | { type: 'deletePlaylistRecord'; index: number }
  | { type: 'setPlaylistSort'; sort: PlaylistSort }
  | { type: 'loadPlaylistRecord'; index: number }
  | { type: 'stepPlaylist'; delta: number }

export interface RegistButton {
  index: number
  /** It holds a registration (lamp blue, or red when selected). */
  stored: boolean
  name: string
  /** The groups it memorized. */
  groups: RegistGroup[]
  style: string | null
  tempo: number | null
  /** Right 1, Right 2, Right 3, Left; empty when it stores no parts. */
  voices: { name: string; on: boolean }[]
}

export interface RegistrationState {
  bank: { name: string; path: string | null; dirty: boolean; position: number | null }
  banks: { name: string; path: string }[]
  /** Where banks are saved; null when saving is off. */
  folder: string | null
  /** Always 10. */
  buttons: RegistButton[]
  /** The button last recalled or memorized. */
  selected: number | null
  /** MEMORY is armed. */
  memory: boolean
  memorizeGroups: RegistGroup[]
  freeze: boolean
  freezeGroups: RegistGroup[]
  sequence: { on: boolean; steps: number[]; end: SequenceEnd; position: number | null }
  /** A recall waits for its style to take over (the bar line). */
  pending: boolean
}

export interface PlaylistRow {
  /** Its position in the file (what the commands take). */
  index: number
  record: PlaylistRecord
  /** Its bank or style file is not there. */
  missing: boolean
}

export interface PlaylistState {
  name: string
  path: string | null
  dirty: boolean
  sort: PlaylistSort
  /** In display order. */
  records: PlaylistRow[]
  /** The record last loaded (file position). */
  current: number | null
  playlists: { name: string; path: string }[]
  folder: string | null
}

/** The file name (less its extension) a bank or playlist called `name` is saved under, as
 * the backend makes it (`registration::file_name`): `/`, `\`, `:` and NUL become `_`,
 * leading dots go, and an empty name is Untitled. */
export function fileStem(name: string): string {
  // eslint-disable-next-line no-control-regex
  const clean = name.trim().replace(/[/\\:\u0000]/g, '_').replace(/^\.+/, '')
  return clean || 'Untitled'
}

/** Names `a` and `b` save to the same file: the Mac's file system (APFS) ignores case. */
export const sameFile = (a: string, b: string) => fileStem(a).toLowerCase() === fileStem(b).toLowerCase()

/** The groups in the Genos's order, with the names the Memory and Freeze windows use. */
export const REGIST_GROUPS: { id: RegistGroup; name: string }[] = [
  { id: 'style', name: 'Style' },
  { id: 'voice', name: 'Voice' },
  { id: 'harmonyArp', name: 'Harmony/Arp' },
  { id: 'multiPad', name: 'Multi Pad' },
  { id: 'tempo', name: 'Tempo' },
  { id: 'transpose', name: 'Transpose' },
  { id: 'chordLooper', name: 'Chord Looper' },
  { id: 'liveControl', name: 'Live Control' },
  { id: 'assignable', name: 'Assignable' },
]

export const SEQUENCE_ENDS: { id: SequenceEnd; name: string }[] = [
  { id: 'stop', name: 'Stop' },
  { id: 'top', name: 'Top' },
  { id: 'next', name: 'Next bank' },
]

export function emptyRegistration(): RegistrationState {
  return {
    bank: { name: 'New Bank', path: null, dirty: false, position: null },
    banks: [],
    folder: null,
    buttons: Array.from({ length: 10 }, (_, index) => ({ index, stored: false, name: '', groups: [], style: null, tempo: null, voices: [] })),
    selected: null,
    memory: false,
    memorizeGroups: REGIST_GROUPS.map((g) => g.id),
    freeze: false,
    freezeGroups: [],
    sequence: { on: false, steps: [], end: 'stop', position: null },
    pending: false,
  }
}

export function emptyPlaylist(): PlaylistState {
  return { name: 'New Playlist', path: null, dirty: false, sort: 'normal', records: [], current: null, playlists: [], folder: null }
}
