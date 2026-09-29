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

/** The file name (less its extension) a bank called `name` is saved under, as the backend
 * makes it: `/`, `\`, `:` and NUL become `_`, leading dots go, and an empty name is
 * Untitled. */
export function fileStem(name: string): string {
  // eslint-disable-next-line no-control-regex
  const clean = name.trim().replace(/[/\\:\u0000]/g, '_').replace(/^\.+/, '')
  return clean || 'Untitled'
}

/** Names `a` and `b` save to the same file: the Mac's file system (APFS) ignores case. */
export const sameFile = (a: string, b: string) => fileStem(a).toLowerCase() === fileStem(b).toLowerCase()
