/** One of the nine strips: its Fader and the name button under it. */
export type FaderStrip = {
  /** What the callbacks carry ("right1", "style", "fader7", "master"). Unique in the bank. */
  id: string
  /** The name under the fader ("Right 1", "Style", "—"). */
  tag: string
  /** The strip's colour token: a part hue, `a` (Style), `t2` (Multi Pad), `t` (Master). */
  hue: 'r1' | 'r2' | 'r3' | 'l' | 'a' | 't2' | 't'
  /** `part`, `group`, `master` live; `off` a part switched off; `parked` an unused fader. */
  kind: 'part' | 'group' | 'master' | 'off' | 'parked'
  /** The text at the top of the fader: the level ("90"), or the layer and value ("Rev 40"). */
  value: string
  /** The set level, 0–127. */
  level: number
  /** The meters and the peak line, 0–1 of the travel. */
  meter: number
  meter2: number
  peak: number
  /** Where the hardware fader is (0–127) while it's away from the level. */
  away?: number
  /** The marks after the name. */
  edited?: boolean
  missing?: boolean
  failed?: boolean
  /** The fader's accessible name ("Right 1 · Stage Grand, level 90"). */
  faderName: string
  /** The name button's accessible name ("Right 1, Stage Grand: open Channel"). Parked: none, not pressable. */
  openName?: string
  /** The fader's tooltip key. */
  tip?: string
}

/** One lamp in the band's lamp row: a part's on/off, or a Launchkey function. */
export type BankLamp = {
  /** What the callbacks carry ("right1", "harmArp"). Unique in the row. */
  id: string
  /** The word on the face ("On", "Harm/Arp"). */
  label: string
  /** Lit. */
  on: boolean
  /** The lamp bar's colour token: the part hue for a part lamp, `m` (grey) for a function. */
  hue?: 't' | 'r1' | 'r2' | 'r3' | 'l' | 'ok' | 'm'
  /** The accessible name ("Right 1 on. Long press: swap mode …"). */
  name?: string
  /** The tooltip key. */
  tip?: string
  /** It has a long press (swap mode, Loop rec, Sound held): `onlamplong` and `onlamprelease` are wired. */
  long?: boolean
}
