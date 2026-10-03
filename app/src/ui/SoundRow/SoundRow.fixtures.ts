import type { ComponentProps } from 'svelte'
import type SoundRow from './SoundRow.svelte'

/**
 * The dark board's sound row: Sunday drive (modified) on A1; R1 1 Stage Grand, R2 41 Silk Strings
 * (edited), R3 57 Brass Section (off, plugin missing), L 41 Silk Strings.
 */
export const soundRowBoard = {
  rack: 'Sunday drive',
  slot: 'A1',
  modified: true,
  parts: [
    { id: 'right1', part: 'R1', partName: 'Right 1', hue: 'r1', number: '1', sound: 'Stage Grand' },
    { id: 'right2', part: 'R2', partName: 'Right 2', hue: 'r2', number: '41', sound: 'Silk Strings', edited: true },
    {
      id: 'right3',
      part: 'R3',
      partName: 'Right 3',
      hue: 'r3',
      number: '57',
      sound: 'Brass Section',
      off: true,
      missing: true,
    },
    { id: 'left', part: 'L', partName: 'Left', hue: 'l', number: '41', sound: 'Silk Strings' },
  ],
} satisfies ComponentProps<typeof SoundRow>

/** A saved rack with every part on and clean, Left playing the Style's bass, R2's plugin failed. */
export const soundRowClean = {
  rack: 'Ballad night',
  slot: 'B3',
  modified: false,
  parts: [
    { id: 'right1', part: 'R1', partName: 'Right 1', hue: 'r1', number: '3', sound: 'Warm Rhodes' },
    { id: 'right2', part: 'R2', partName: 'Right 2', hue: 'r2', number: '12', sound: 'Sampler Deluxe', failed: true },
    { id: 'right3', part: 'R3', partName: 'Right 3', hue: 'r3', number: '88', sound: 'Soft Pad' },
    { id: 'left', part: 'L', partName: 'Left', hue: 'l', number: '33', sound: 'Finger Bass', bass: true },
  ],
} satisfies ComponentProps<typeof SoundRow>
