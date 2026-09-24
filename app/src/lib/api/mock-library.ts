// A large synthetic style library for the mock (`?styles=60000`), to check the browser
// stays smooth with a big collection. Deterministic: the same `n` always gives the same
// library. Folder names, style names and files are made up; about 1.5% of the files are
// unreadable (error rows) and some have no name marker (the name is the file name).

import { BREAK, ENDINGS, FILLS, INTROS, MAINS } from './types'

export interface SyntheticStyle {
  id: number
  name: string
  folder: string
  file: string
  tempo: number
  timeSignature: number[]
  sections: string[]
  ots: number
  format: 'SFF1' | 'SFF2'
  error?: string
}

/** mulberry32: a tiny seeded PRNG. */
function rng(seed: number): () => number {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

const COLLECTIONS = ['Factory', 'Expansion Packs', 'Downloads', 'Archive 2019', 'Archive 2023', 'Band Library', 'Imports']
const GENRES = [
  'Pop', 'Rock', 'Ballad', 'Dance', 'Swing', 'Jazz', 'R&B', 'Soul', 'Country', 'Latin', 'Ballroom',
  'Entertainer', 'Folk', 'Gospel', 'Funk', 'Blues', 'Reggae', 'World', 'Movie & Show', 'Children',
]
const PACKS = ['Vol 1', 'Vol 2', 'Vol 3', 'Live Set', 'Session', 'Classics', 'Club', 'Unplugged']
const ADJ = [
  'Sunny', 'Midnight', 'Velvet', 'Electric', 'Golden', 'Rolling', 'Silver', 'Urban', 'Smooth', 'Wild',
  'Blue', 'Lazy', 'Happy', 'Dusty', 'Neon', 'Easy', 'Grand', 'Little', 'Modern', 'Classic', 'Deep', 'Bright',
]
const NOUN = [
  'Groove', 'Shuffle', 'Waltz', 'March', 'Ballad', 'Stomp', 'Swing', 'Train', 'Boogie', 'Beat', 'Party',
  'Serenade', 'Drive', 'Strut', 'Bounce', 'Samba', 'Tango', 'Polka', 'Bossa', 'Hustle', 'Anthem', 'Twist',
]
const EXT = ['.sty', '.prs', '.sst', '.bcs', '.pst', '.fps']
const ERRORS = ['not an SFF file: missing MThd header', 'truncated file: track chunk ends early', 'no CASM and no Main section']

/** `n` styles with ids from `firstId` on, in no particular order. */
export function syntheticStyles(n: number, firstId: number): SyntheticStyle[] {
  const r = rng(n * 7919 + 17)
  const pick = <T>(a: T[]): T => a[Math.floor(r() * a.length)]
  const folders: string[] = []
  for (const c of COLLECTIONS) {
    for (const g of GENRES) {
      folders.push(`${c}/${g}`)
      if (r() < 0.35) folders.push(`${c}/${g}/${pick(PACKS)}`)
    }
  }
  folders.push('') // a few files at the top level
  const out: SyntheticStyle[] = []
  for (let i = 0; i < n; i++) {
    const folder = r() < 0.002 ? '' : pick(folders)
    const name = `${pick(ADJ)} ${pick(NOUN)}${r() < 0.3 ? ' ' + (2 + Math.floor(r() * 9)) : ''}`
    const stem = `${name.replace(/[^A-Za-z0-9]/g, '')}${i}`
    const ext = pick(EXT)
    const sections = [
      ...INTROS.slice(0, 1 + Math.floor(r() * 3)),
      ...MAINS.slice(0, 2 + Math.floor(r() * 3)),
      ...FILLS.slice(0, 2 + Math.floor(r() * 3)),
      ...(r() < 0.8 ? [BREAK] : []),
      ...ENDINGS.slice(0, 1 + Math.floor(r() * 3)),
    ]
    const ts = r() < 0.12 ? [3, 4] : r() < 0.06 ? [6, 8] : [4, 4]
    const s: SyntheticStyle = {
      id: firstId + i,
      // No name marker in some files: the engine shows the file name.
      name: r() < 0.05 ? stem : name,
      folder,
      file: stem + ext,
      tempo: 60 + Math.floor(r() * 120),
      timeSignature: ts,
      sections,
      ots: Math.floor(r() * 5),
      format: ext === '.sty' && r() < 0.6 ? 'SFF1' : 'SFF2',
    }
    if (r() < 0.015) s.error = pick(ERRORS)
    out.push(s)
  }
  return out
}
