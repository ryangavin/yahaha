<!--
  PartMarks: the small marks after a part's sound name (sound cell, fader strip name): the edited
  dot, the plugin's ⚠ (missing) or ✕ (failed), and the words "off" or "bass". The parent passes
  the facts; `visibleMarks` applies the precedence. Hidden from assistive tech: the parent's
  aria-label says the same, built with `marksText`. Renders nothing when no mark shows.
-->
<script lang="ts">
  import { visibleMarks } from './marks'

  type Props = {
    /** The part's sound was changed since it was loaded (`soundEdited`): the 5px dot. */
    edited?: boolean
    /** The part's plugin isn't installed (`plugin.missing`): the orange ⚠. Wins over `failed`. */
    missing?: boolean
    /** The part's plugin failed to load or crashed: the red ✕, drawn only when `missing` is false. */
    failed?: boolean
    /** The part is off and silent: the word "off". Not drawn when `bass` is true. */
    off?: boolean
    /** The part plays the Style's bass under Manual Bass (`playsBass`): the word "bass". */
    bass?: boolean
    /** The gap between marks: `cell` 8px (the sound cell), `strip` 4px (the fader strip's name). */
    size?: 'cell' | 'strip'
  }

  let { edited = false, missing = false, failed = false, off = false, bass = false, size = 'cell' }: Props = $props()

  let marks = $derived(visibleMarks({ edited, missing, failed, off, bass }))
</script>

{#if marks.length > 0}
  <span class="marks {size}" aria-hidden="true">
    {#each marks as mark (mark)}
      {#if mark === 'edited'}
        <span class="dot" data-mark="edited" data-hue="t"></span>
      {:else if mark === 'missing'}
        <svg data-mark="missing" data-hue="warn" class="warn" width="12" height="12" viewBox="0 0 12 12">
          <path d="M6 1.5 L11 10.5 H1 Z" /><path d="M6 5 V7.4" /><circle cx="6" cy="8.9" r="0.6" />
        </svg>
      {:else if mark === 'failed'}
        <svg data-mark="failed" data-hue="ending" class="fail" width="12" height="12" viewBox="0 0 12 12">
          <path d="M2.5 2.5 L9.5 9.5 M9.5 2.5 L2.5 9.5" />
        </svg>
      {:else}
        <span class="word" data-mark={mark} data-hue="m">{mark}</span>
      {/if}
    {/each}
  </span>
{/if}

<style>
  .marks {
    display: inline-flex;
    align-items: center;
    flex: none;
    gap: var(--space-8);
    white-space: nowrap;
    font-family: var(--font-sans);
    font-size: var(--text-12);
    font-weight: var(--weight-regular);
    line-height: normal;
    color: var(--m);
  }
  .strip {
    gap: var(--space-4);
  }
  .marks > * {
    flex: none;
  }
  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--t);
  }
  .warn {
    fill: none;
    stroke: var(--warn);
    stroke-width: 1;
    stroke-linejoin: round;
  }
  .warn circle {
    fill: var(--warn);
    stroke: none;
  }
  .fail {
    fill: none;
    stroke: var(--ending);
    stroke-width: 1.5;
    stroke-linecap: round;
  }
</style>
