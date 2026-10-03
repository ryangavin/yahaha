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

/**
 * The dark board's now playing: Am7, Main B playing, Main C next with the fill after bar 4, bar 3
 * of 4 on beat 3 of 4, 104 BPM, running.
 */
export const nowPlayingBoard = {
  chord: chordBoard,
  playing: 'Main B',
  hue: 'main',
  next: 'Main C',
  fill: 'fill lands after bar 4',
  bar: 3,
  bars: 4,
  beat: 3,
  beats: 4,
  progress: 0.62,
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
  fill: '',
  bar: 1,
  bars: 4,
  beat: 0,
  beats: 4,
  progress: 0,
  bpm: 92,
  running: false,
} satisfies ComponentProps<typeof NowPlaying>

/** A fill on its last beat with Ending II waiting, in 3/4. */
export const nowPlayingFill = {
  chord: chordBoard,
  playing: 'Fill',
  hue: 'fill',
  next: 'Ending II',
  fill: '',
  bar: 1,
  bars: 1,
  beat: 3,
  beats: 3,
  progress: 0.9,
  bpm: 104,
  running: true,
} satisfies ComponentProps<typeof NowPlaying>
