<!--
  HealthSlot: the app bar's audio health. Says "Audio" quietly while the sound is fine, and names
  the trouble (a failed plugin, no audio, dropouts, a busy CPU) as a text button that calls onopen
  with where to fix it. Takes the inputs as props; `health()` picks the one text.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import { health, type HealthTarget } from './health'

  type Props = {
    /** The first keyboard part (0–3: R1 R2 R3 L) whose plugin failed and isn't missing (`firstFailedPart`). */
    failedPart?: 0 | 1 | 2 | 3 | null
    /** False when there is no audio (`io.synth` is null). */
    synthOn?: boolean
    /** Audio dropouts in the last 30 s, counted by the wiring. */
    dropouts?: number
    /** The buffer in use, in frames (64 … 1024); null when unknown. */
    bufferFrames?: number | null
    /** The CPU load, 1.0 = the whole buffer; null before the first meters frame. */
    cpu?: number | null
    /** A fixed width in px. Default: fills its container's width. */
    width?: number
    /** The tooltip key, as `data-tip` on the button (or the calm text); null turns it off. */
    tip?: string | null
    /** The app's `use:tip` action, applied with `tip` to the element carrying `data-tip`. */
    tipAction?: Action<HTMLElement, string>
    /** Called with where to fix the trouble on a click, Enter or Space. */
    onopen?: (target: HealthTarget) => void
  }

  let {
    failedPart = null,
    synthOn = true,
    dropouts = 0,
    bufferFrames = null,
    cpu = null,
    width,
    tip = 'app.health',
    tipAction,
    onopen,
  }: Props = $props()

  let shown = $derived(health({ failedPart, synthOn, dropouts, bufferFrames, cpu }))

  const none: Action<HTMLElement, string> = () => {}
  // The action is fixed per element: the button or the calm span mounts fresh when the state flips.
  let act = $derived(tip !== null && tipAction ? tipAction : none)

  function open() {
    if (shown.target) onopen?.(shown.target)
  }
</script>

<span
  class="slot"
  class:fill={width === undefined}
  data-hue={shown.hue}
  style:width={width === undefined ? undefined : `${width}px`}
>
  <span role="status" class="hidden-word">{shown.label}</span>
  {#if shown.target}
    <button type="button" data-tip={tip ?? undefined} use:act={tip ?? ''} onclick={open}>
      <span class="text">{shown.text}</span>
    </button>
  {:else}
    <span class="text" aria-hidden="true" data-tip={tip ?? undefined} use:act={tip ?? ''}>{shown.text}</span>
  {/if}
</span>

<style>
  .slot {
    position: relative;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    height: 35px;
    padding-left: var(--space-8);
    min-width: 0;
    color: var(--m);
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    font-variant-numeric: tabular-nums;
    line-height: normal;
  }
  .fill {
    flex: 1 1 auto;
  }
  .slot[data-hue='ending'] {
    color: var(--ending);
  }
  .hidden-word {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  button {
    display: flex;
    align-items: center;
    max-width: 100%;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  button:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
