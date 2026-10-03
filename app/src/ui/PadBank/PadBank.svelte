<!--
  PadBank: the band's Pads section. A GroupHeader with the bank's name, its hue legend and the
  bank counter; the bank ▲ ▼ buttons; and sixteen Pads in two rows of eight. Push group lines (a
  2px line in the family hue) run over each run of one section family's pads; each utility pad
  has its own grey line. Holds no state: the flash phase comes in `lit`, presses go out by index.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import Button from '../Button/Button.svelte'
  import GroupHeader from '../GroupHeader/GroupHeader.svelte'
  import HueLegend from '../HueLegend/HueLegend.svelte'
  import Pad from '../Pad/Pad.svelte'
  import type { LegendItem, PadItem } from './types'

  type Props = {
    /** The sixteen pads, row by row. */
    pads: PadItem[]
    /** The bank's name ("Sections", "Quick Racks"). */
    bankName: string
    /** The bank counter ("1/5"). */
    count: string
    /** The hue legend after the name. */
    legend?: LegendItem[]
    /** The flash phase of queued and armed pads. */
    lit?: boolean
    /** The app's `use:tip` action, passed to every control. */
    tipAction?: Action<HTMLElement, string>
    /** The bank ▲ button. */
    onbankup?: () => void
    /** The bank ▼ button. */
    onbankdown?: () => void
    /** A pad was pressed (0–15). */
    onpress?: (index: number) => void
  }

  let { pads, bankName, count, legend = [], lit = true, tipAction, onbankup, onbankdown, onpress }: Props = $props()

  const COLUMNS = 8
  const isUtil = (pad: PadItem | undefined) => pad === undefined || pad.family === 'util' || pad.family === 'start'
  /** The pad's group line joins the next one: same row, same section family. */
  function joins(i: number) {
    const next = pads[i + 1]
    return (i + 1) % COLUMNS !== 0 && !isUtil(pads[i]) && !isUtil(next) && next.family === pads[i].family
  }
</script>

<section class="bank" aria-label="Pads">
  <GroupHeader title="Pads" count={{ label: 'Bank', value: count }}>
    <span class="bank-name">{bankName}</span>
    {#if legend.length > 0}<span class="legend"><HueLegend items={legend} /></span>{/if}
  </GroupHeader>
  <div class="body">
    <div class="steps">
      <Button symbol="up" size="icon" name="Pad bank up" tip="padpage.prev" {tipAction} onpress={onbankup} />
      <Button symbol="down" size="icon" name="Pad bank down" tip="padpage.next" {tipAction} onpress={onbankdown} />
    </div>
    <div class="pads">
      {#each pads as pad, i (i)}
        <div class="cell">
          <span
            class="line"
            class:join={joins(i)}
            style:--line-hue={isUtil(pad) ? 'var(--util)' : `var(--${pad.family})`}
            aria-hidden="true"
          ></span>
          <Pad
            label={pad.label}
            index={String(i + 1)}
            family={pad.family}
            state={pad.state}
            {lit}
            name={pad.name}
            tip={pad.tip}
            {tipAction}
            onpress={() => onpress?.(i)}
          />
        </div>
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
  .bank-name {
    color: var(--t);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
  }
  .legend {
    display: flex;
    margin-left: var(--space-4);
  }
  .body {
    display: flex;
    gap: var(--space-8);
    margin-top: var(--band-body-gap);
  }
  .steps {
    display: flex;
    flex: none;
    flex-direction: column;
    justify-content: space-between;
    box-sizing: border-box;
    width: var(--control-height);
    padding: var(--pad-step-pad) 0;
  }
  .pads {
    display: grid;
    flex: 1;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    grid-auto-rows: var(--pad-size);
    gap: var(--pad-gap);
    min-width: 0;
  }
  .cell {
    position: relative;
    min-width: 0;
  }
  .line {
    position: absolute;
    top: calc(-1 * var(--pad-line-offset));
    right: 0;
    left: 0;
    height: var(--pad-line-height);
    background: var(--line-hue);
  }
  .line.join {
    right: calc(-1 * var(--pad-gap));
  }
</style>
