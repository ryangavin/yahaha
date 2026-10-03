/** One of the eight band knobs. */
export type KnobItem = {
  /** The plain name ("Dynamics"); "---" when unused. */
  label: string
  /** The Genos code ("DynCtrl"). */
  code: string
  /** The value as shown ("127", "1/8", "Off", "0"). */
  value: string
  /** A small unit after the value ("%"). */
  unit?: string
  /** How far round the arc is, 0–1. */
  fraction: number
  /** Nothing mapped. */
  unused?: boolean
}
