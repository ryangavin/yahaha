<!--
  A labelled read-only display (tempo, bar/beat, chord, split …). It is focusable so its
  tooltip can be reached from the keyboard too, which makes it "interactive" for the
  tooltip coverage test: give it a catalog `tip`.
  Put related Keys in the `controls` snippet; they sit beside the value.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { TipKey } from '../../help/tooltips'
  import { tip as tipAction } from '../tooltip/tip.svelte'

  let {
    label,
    tip,
    children,
    controls,
    class: klass = '',
  }: { label: string; tip: TipKey; children: Snippet; controls?: Snippet; class?: string } = $props()
</script>

<div class="readout {klass}">
  <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
  <div class="value" role="group" aria-label={label} tabindex="0" use:tipAction={tip}>
    <span class="label">{label}</span>
    <span class="content">{@render children()}</span>
  </div>
  {#if controls}<div class="controls">{@render controls()}</div>{/if}
</div>

<style>
  .readout {
    display: flex;
    align-items: stretch;
    gap: 0.4rem;
    min-width: 0;
  }
  .value {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    padding: 0.1rem 0.25rem;
    border-radius: 4px;
  }
  .label {
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.1;
  }
  .content {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.5rem;
    line-height: 1.15;
    white-space: nowrap;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
</style>
