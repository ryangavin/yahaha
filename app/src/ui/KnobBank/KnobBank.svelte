<!--
  KnobBank: the band's Knobs section. A GroupHeader with the knob page's accent block and its
  page counter; the page ▲ ▼ buttons; and the eight Knobs. Holds no state: every change is a
  callback with the knob's position (0–7).
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import AccentBlock from '../AccentBlock/AccentBlock.svelte'
  import Button from '../Button/Button.svelte'
  import GroupHeader from '../GroupHeader/GroupHeader.svelte'
  import Knob from '../Knob/Knob.svelte'
  import type { KnobItem } from './types'

  type Props = {
    /** The eight knobs, left to right. */
    knobs: KnobItem[]
    /** The knob page's name in the accent block ("Style", "Swap R1"). */
    pageLabel: string
    /** The page counter ("1/6"). */
    count: string
    /** The app's `use:tip` action, passed to every control. */
    tipAction?: Action<HTMLElement, string>
    /** The page ▲ button. */
    onpageup?: () => void
    /** The page ▼ button. */
    onpagedown?: () => void
    /** A knob was clicked. */
    onpress?: (index: number) => void
    /** A knob asked for a step (+1 or −1). */
    onstep?: (index: number, delta: number) => void
  }

  let { knobs, pageLabel, count, tipAction, onpageup, onpagedown, onpress, onstep }: Props = $props()
</script>

<section class="bank" aria-label="Knobs">
  <GroupHeader title="Knobs" count={{ label: 'Page', value: count }}>
    <AccentBlock label={pageLabel} size="knob" />
  </GroupHeader>
  <div class="body">
    <div class="steps">
      <Button symbol="up" size="icon" name="Knob page up" tip="knobs.page" {tipAction} onpress={onpageup} />
      <Button symbol="down" size="icon" name="Knob page down" tip="knobs.page" {tipAction} onpress={onpagedown} />
    </div>
    <div class="knobs">
      {#each knobs as knob, i (i)}
        <Knob
          label={knob.label}
          code={knob.code}
          value={knob.value}
          unit={knob.unit}
          fraction={knob.fraction}
          unused={knob.unused}
          name={`Knob ${i + 1}: ${knob.unused ? 'unused' : `${knob.label} (${knob.code}) ${knob.value}${knob.unit ?? ''}`}`}
          tip="knobs.knob"
          {tipAction}
          onpress={() => onpress?.(i)}
          onstep={(delta) => onstep?.(i, delta)}
        />
      {/each}
    </div>
  </div>
</section>

<style>
  .bank {
    display: flex;
    flex-direction: column;
    width: var(--band-middle-width);
    font-family: var(--font-sans);
  }
  .body {
    display: flex;
    gap: var(--space-8);
    height: var(--knob-height);
    margin-top: var(--band-body-gap);
  }
  .steps {
    display: flex;
    flex: none;
    flex-direction: column;
    justify-content: center;
    gap: var(--space-6);
    width: var(--control-height);
  }
  .knobs {
    display: grid;
    flex: 1;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    column-gap: var(--knob-gap);
    min-width: 0;
  }
</style>
