/** One choice in a ChosenTabs run. */
export type TabItem = {
  /** What `onchoose` carries and what `chosen` names. Unique within the run. */
  id: string
  /** The word on the tab ("Vol", "Quick Racks"). */
  label: string
  /** The accessible name when the label is short ("Volume" for "Vol"). Default: the label. */
  name?: string
  /** Shown, not choosable. */
  disabled?: boolean
  /** The tooltip key (`view.stage`, `mixer.layer`), set as `data-tip` and passed to `tipAction`. */
  tip?: string
}
