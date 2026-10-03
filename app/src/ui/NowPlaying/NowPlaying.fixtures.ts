import type { ComponentProps } from 'svelte'
import type ChordReadout from '../ChordReadout/ChordReadout.svelte'
import type NowPlaying from './NowPlaying.svelte'

/** The board's chord: Am7 (A C E G over R m3 5 m7), Fingered. */
export const chordBoard = {
  chord: 'Am',
  extension: '7',
  notes: [
    { note: 'A', interval: 'R' },
    { note: 'C', interval: 'm3' },
    { note: 'E', interval: '5' },
    { note: 'G', interval: 'm7' },
  ],
  fingering: 'Fingered',
  held: false,
} satisfies ComponentProps<typeof ChordReadout>

/** The dark board's now playing: Am7, Main B playing, Main C next, 104 BPM, running. */
export const nowPlayingBoard = {
  chord: chordBoard,
  playing: 'Main B',
  hue: 'main',
  next: 'Main C',
  nextHue: 'main',
  bpm: 104,
  running: true,
} satisfies ComponentProps<typeof NowPlaying>

/** Stopped on Main A with the last chord (Cmaj7) held, nothing next. */
export const nowPlayingStopped = {
  chord: {
    chord: 'C',
    extension: 'maj7',
    notes: [
      { note: 'C', interval: 'R' },
      { note: 'E', interval: '3' },
      { note: 'G', interval: '5' },
      { note: 'B', interval: '7' },
    ],
    fingering: 'Fingered',
    held: true,
  },
  playing: 'Main A',
  hue: 'main',
  next: '',
  nextHue: 'main',
  bpm: 92,
  running: false,
} satisfies ComponentProps<typeof NowPlaying>
