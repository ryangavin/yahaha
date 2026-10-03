<!--
  FaderBank: the band's Faders section. A GroupHeader with the fader page and layer tabs; nine
  strips, each a Fader over its name button with PartMarks; the "Part on/off" and "Functions"
  caption row; and one 32px lamp row: the part lamps under faders 1–4 (their bar in the part's
  hue), a hairline, the Launchkey function lamps under 5–8 (a grey bar), and the Panel page button
  under 9 (no bar). The layer tabs use the secondary (grey) chosen block. Holds no state: every change is a
  callback with the strip's or lamp's id.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import Button from '../Button/Button.svelte'
  import ChosenTabs from '../ChosenTabs/ChosenTabs.svelte'
  import type { TabItem } from '../ChosenTabs/types'
  import Fader from '../Fader/Fader.svelte'
  import GroupHeader from '../GroupHeader/GroupHeader.svelte'
  import LampButton from '../LampButton/LampButton.svelte'
  import PartMarks from '../PartMarks/PartMarks.svelte'
  import Separator from '../Separator/Separator.svelte'
  import type { BankLamp, FaderStrip } from './types'

  type Props = {
    /** The nine strips, left to right. */
    strips: FaderStrip[]
    /** The fader page tabs (Panel, Style). */
    pageTabs: TabItem[]
    /** The chosen page's id. */
    page: string
    /** The fader layer tabs (Vol, Pan, Reverb, Chorus, Delay). */
    layerTabs: TabItem[]
    /** The chosen layer's id; any but the first gives strips 1–4 the layer look and the header its word. */
    layer: string
    /** The part lamps under faders 1–4. */
    partLamps: BankLamp[]
    /** The Launchkey function lamps under faders 5–8. */
    functionLamps: BankLamp[]
    /** The app's `use:tip` action, passed to every control. */
    tipAction?: Action<HTMLElement, string>
    /** A page tab was chosen. */
    onchoosePage?: (id: string) => void
    /** A layer tab was chosen. */
    onchooseLayer?: (id: string) => void
    /** A fader asked for a level (0–127). */
    onlevel?: (id: string, level: number) => void
    /** A strip's name button was pressed (open Channel, Effects, …). */
    onopen?: (id: string) => void
    /** A lamp asked to be switched to `on`. */
    onlamp?: (id: string, on: boolean) => void
    /** A lamp with `long` was held. */
    onlamplong?: (id: string) => void
    /** The hold on a `long` lamp ended. */
    onlamprelease?: (id: string) => void
    /** The Panel page button under fader 9 (flips the page). */
    onpagebutton?: () => void
  }

  let {
    strips,
    pageTabs,
    page,
    layerTabs,
    layer,
    partLamps,
    functionLamps,
    tipAction,
    onchoosePage,
    onchooseLayer,
    onlevel,
    onopen,
    onlamp,
    onlamplong,
    onlamprelease,
    onpagebutton,
  }: Props = $props()

  let layered = $derived(layerTabs.length > 0 && layer !== layerTabs[0].id)
  let layerWord = $derived(layered ? layerTabs.find((tab) => tab.id === layer)?.label : undefined)
  let pageLabel = $derived(pageTabs.find((tab) => tab.id === page)?.label ?? '')
  let otherLabel = $derived(pageTabs.find((tab) => tab.id !== page)?.label ?? '')

  const isPart = (strip: FaderStrip) => strip.kind === 'part' || strip.kind === 'off'
  const isLive = (strip: FaderStrip) => strip.kind !== 'off' && strip.kind !== 'parked'
</script>

{#snippet lamp(item: BankLamp)}
  <LampButton
    label={item.label}
    on={item.on}
    hue={item.hue}
    size="cell"
    name={item.name}
    tip={item.tip}
    {tipAction}
    ontoggle={(on) => onlamp?.(item.id, on)}
    onlongpress={item.long ? () => onlamplong?.(item.id) : undefined}
    onlongrelease={item.long ? () => onlamprelease?.(item.id) : undefined}
  />
{/snippet}

<section class="bank" aria-label="Faders">
  <GroupHeader title="Faders" detail={layerWord}>
    <ChosenTabs size="header" label="Fader page (master button)" tabs={pageTabs} chosen={page} {tipAction} onchoose={onchoosePage} />
    <Separator />
    <span class="layer">
      <span class="layer-word">Layer</span>
      <ChosenTabs size="header" tone="secondary" label="Fader layer" tabs={layerTabs} chosen={layer} {tipAction} onchoose={onchooseLayer} />
    </span>
  </GroupHeader>

  <div class="body">
    <div class="strips">
      {#each strips as strip (strip.id)}
        <div class="strip">
          <Fader
            name={strip.faderName}
            value={strip.value}
            level={strip.level}
            meter={strip.meter}
            meter2={strip.meter2}
            peak={strip.peak}
            away={strip.away}
            kind={strip.kind}
            hue={strip.hue}
            layered={layered && isPart(strip)}
            tip={strip.tip}
            {tipAction}
            onlevel={(level) => onlevel?.(strip.id, level)}
          />
          <button
            type="button"
            class="name"
            style:--hue={isLive(strip) ? `var(--${strip.hue})` : 'var(--d)'}
            aria-label={strip.openName ?? strip.tag}
            onclick={() => onopen?.(strip.id)}
          >
            <span class="tag">{strip.tag}</span>
            <PartMarks size="strip" edited={strip.edited} missing={strip.missing} failed={strip.failed} />
          </button>
        </div>
      {/each}
    </div>

    <div class="captions">
      <span class="caption parts">Part on/off</span>
      <span class="caption functions">Functions<span class="hint">Launchkey fader buttons 5–9</span></span>
    </div>

    <div class="lamps">
      <span class="divider" aria-hidden="true"></span>
      {#each partLamps as item (item.id)}{@render lamp(item)}{/each}
      {#each functionLamps as item (item.id)}{@render lamp(item)}{/each}
      <Button
        label={pageLabel}
        size="cell"
        name={`Fader page is ${pageLabel}: click for ${otherLabel}`}
        tip="mixer.page"
        {tipAction}
        onpress={onpagebutton}
      />
    </div>
  </div>
</section>

<style>
  .bank {
    display: flex;
    flex-direction: column;
    width: var(--band-faders-width);
    height: var(--band-height);
    font-family: var(--font-sans);
  }
  .layer {
    display: flex;
    align-items: baseline;
  }
  .layer-word {
    margin-right: var(--space-4);
    color: var(--m);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
  }
  .body {
    display: flex;
    flex-direction: column;
    margin-top: var(--band-body-gap);
  }
  .strips,
  .captions,
  .lamps {
    display: grid;
    grid-template-columns: repeat(9, minmax(0, 1fr));
    column-gap: var(--space-8);
  }
  .strip {
    display: flex;
    flex-direction: column;
    min-width: 0;
    height: var(--fader-strip-height);
  }
  .name {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    box-sizing: border-box;
    width: 100%;
    height: var(--fader-name-height);
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    white-space: nowrap;
    cursor: pointer;
  }
  .tag {
    color: var(--hue);
    font-family: var(--font-sans);
    font-size: var(--text-13);
    font-weight: var(--weight-medium);
    line-height: var(--space-16);
  }
  .captions {
    height: var(--band-caption-height);
    white-space: nowrap;
  }
  .caption {
    box-sizing: border-box;
    border-bottom: var(--line-width) solid var(--line);
    color: var(--m);
    font-size: var(--text-12);
    line-height: var(--space-16);
  }
  .parts {
    grid-column: 1 / 5;
  }
  .functions {
    display: flex;
    grid-column: 5 / 10;
  }
  .hint {
    margin-left: auto;
  }
  .lamps {
    position: relative;
    height: var(--control-height);
    margin-top: var(--band-lamp-gap);
  }
  /* The hairline between the part lamps and the functions, centred in the gap after column 4. */
  .divider {
    position: absolute;
    top: 0;
    bottom: 0;
    left: calc((100% - 8 * var(--space-8)) * 4 / 9 + 3.5 * var(--space-8));
    width: var(--line-width);
    background: var(--line);
  }
  .name:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
