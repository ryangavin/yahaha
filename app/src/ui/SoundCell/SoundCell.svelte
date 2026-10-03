<!--
  SoundCell: one part's cell on the display's sound row, Push's track-name row. The part's short
  name in its hue (a button that opens Channel), then its sound (a button that opens the quick
  sound list): the voice number muted, the name in text, and the part's marks after it, passed in
  as `marks`. An off part dims: its name to `--d`, its sound to muted.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { Action } from 'svelte/action'

  type Props = {
    /** The part's short name ("R1", "L"). */
    part: string
    /** The part's full name, for the buttons' names and the label's title ("Right 1"). */
    partName: string
    /** The part's hue. */
    hue?: 'r1' | 'r2' | 'r3' | 'l'
    /** The sound's number in its list ("41"). */
    number?: string
    /** The sound's name; a long one ends in an ellipsis. */
    sound: string
    /** The part is off: the short name dims to `--d`, the sound to muted. */
    off?: boolean
    /** The sound button's accessible name. Default: "Right 1 sound: 1 Stage Grand. Opens the quick sound list". */
    soundName?: string
    /** The tooltip key of the part's short name. */
    partTip?: string
    /** The tooltip key of the sound. */
    soundTip?: string
    /** The app's tooltip action (`use:tip`), applied to each button that has a tip. */
    tipAction?: Action<HTMLElement, string>
    /** The marks after the sound's name (PartMarks). */
    marks?: Snippet
    /** Called when the short name is pressed (opens Channel). */
    onpart?: () => void
    /** Called when the sound is pressed (opens the quick sound list). */
    onsound?: () => void
  }

  let {
    part,
    partName,
    hue = 'r1',
    number = '',
    sound,
    off = false,
    soundName,
    partTip,
    soundTip,
    tipAction,
    marks,
    onpart,
    onsound,
  }: Props = $props()

  const spoken = $derived(
    soundName ?? `${partName} sound: ${[number, sound].filter(Boolean).join(' ')}. Opens the quick sound list`,
  )

  function tipOn(node: HTMLElement, key: string | undefined) {
    if (!tipAction || !key) return
    return tipAction(node, key)
  }
</script>

<div class="cell" class:off data-hue={hue}>
  <button
    type="button"
    class="part"
    style:--hue="var(--{hue})"
    title={partName}
    data-contrast={off ? 'dim' : undefined}
    aria-label="{partName}: open Channel"
    data-tip={partTip}
    use:tipOn={partTip}
    onclick={() => onpart?.()}>{part}</button
  >
  <button
    type="button"
    class="sound"
    aria-label={spoken}
    data-tip={soundTip}
    use:tipOn={soundTip}
    onclick={() => onsound?.()}
  >
    {#if number}<span class="number">{number}</span>{/if}
    <span class="title">{sound}</span>
    {@render marks?.()}
  </button>
</div>

<style>
  .cell {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: var(--sound-cell-width);
    min-width: 0;
    height: var(--sound-row-height);
    border-top: var(--line-width) solid var(--line);
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
  }
  button {
    height: calc(var(--sound-row-height) - var(--line-width));
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    font-family: inherit;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }
  button:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: calc(-1 * var(--line-width));
  }
  .part {
    flex: none;
    width: var(--part-label-width);
    font-size: var(--text-14);
    font-weight: var(--weight-medium);
    color: var(--hue);
  }
  .off .part {
    color: var(--d);
  }
  .sound {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }
  .number {
    flex: none;
    font-size: var(--text-12);
    color: var(--m);
  }
  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: var(--text-14);
    color: var(--t);
  }
  .off .number {
    color: var(--d);
  }
  .off .title {
    color: var(--m);
  }
</style>
