/**
 * Display formatting shared across panels.
 */

/**
 * A tempo as the Genos shows it: whole BPM (its range is 5–500, in steps of 1). A style
 * stores its tempo in microseconds per quarter note, so its BPM is rarely a whole number
 * (105.00029, 69.99998); round it rather than truncate. The wire value stays precise.
 * `null`, `undefined` and non-finite tempos show as ''.
 */
export function formatTempo(bpm: number | null | undefined): string {
  if (bpm == null || !Number.isFinite(bpm)) return ''
  return String(Math.round(bpm))
}
