/** One mark PartMarks can draw after a part's sound name. */
export type Mark = 'edited' | 'missing' | 'failed' | 'off' | 'bass'

/** The facts a parent passes, from `keyboardParts[i]`. */
export type MarkFacts = {
  edited?: boolean
  missing?: boolean
  failed?: boolean
  off?: boolean
  bass?: boolean
}

/**
 * The marks drawn, in drawing order: edited, missing or failed, off, bass. ⚠ wins over ✕
 * (`failed` is dropped when `missing`), and "bass" wins over "off" (a bass part sounds).
 */
export function visibleMarks(m: MarkFacts): Mark[] {
  const out: Mark[] = []
  if (m.edited) out.push('edited')
  if (m.missing) out.push('missing')
  else if (m.failed) out.push('failed')
  if (m.off && !m.bass) out.push('off')
  if (m.bass) out.push('bass')
  return out
}

/** The words for each mark in a parent's `aria-label`. */
const WORDS: Record<Mark, string> = {
  edited: 'edited',
  off: 'off',
  missing: 'plugin missing',
  failed: 'plugin failed',
  bass: 'bass',
}

/** Spoken order (Stage's `aria-label` template): edited, off, plugin missing or failed, bass. */
const SPOKEN: Mark[] = ['edited', 'off', 'missing', 'failed', 'bass']

/** The words a parent appends to its `aria-label`, each after ", ": `{ edited: true }` → ", edited". */
export function marksText(m: MarkFacts): string {
  const shown = visibleMarks(m)
  return SPOKEN.filter((mark) => shown.includes(mark))
    .map((mark) => `, ${WORDS[mark]}`)
    .join('')
}
