<!--
  AppBar: the 36px bar on top of every page. "yahaha" at the left; the page nav (the display pages,
  a hairline, the full pages) pushed right; then the fixed right area: a hairline, the Launchkey
  status and the audio health at the right edge, so the tabs sit at the same x on every page. A
  white 1px line underneath. Controlled: `chosen` names the page; a click only calls onchoose.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import ChosenTabs from '../ChosenTabs/ChosenTabs.svelte'
  import type { TabItem } from '../ChosenTabs/types'
  import HealthSlot from '../HealthSlot/HealthSlot.svelte'
  import type { HealthTarget } from '../HealthSlot/health'
  import Separator from '../Separator/Separator.svelte'
  import StatusDot from '../StatusDot/StatusDot.svelte'

  type Props = {
    /** The display pages, left of the hairline (Stage … Harm/Arp). */
    displayTabs: TabItem[]
    /** The full pages, right of the hairline (Library, Settings). */
    fullTabs: TabItem[]
    /** The `id` of the page shown; null = none. */
    chosen?: string | null
    /** Called with a page's `id` on a click. The parent moves `chosen`. */
    onchoose?: (id: string) => void
    /** The app's tooltip action (`use:tip`), for the tabs and the health slot. */
    tipAction?: Action<HTMLElement, string>
    /** True while the Launchkey is connected: a green dot; otherwise a hollow grey ring. */
    launchkey?: boolean
    /** HealthSlot: the first keyboard part (0–3: R1 R2 R3 L) whose plugin failed. */
    failedPart?: 0 | 1 | 2 | 3 | null
    /** HealthSlot: false when there is no audio. */
    synthOn?: boolean
    /** HealthSlot: audio dropouts in the last 30 s. */
    dropouts?: number
    /** HealthSlot: the buffer in use, in frames; null when unknown. */
    bufferFrames?: number | null
    /** HealthSlot: the CPU load, 1.0 = the whole buffer; null before the first meters frame. */
    cpu?: number | null
    /** HealthSlot: called with where to fix the trouble. */
    onhealth?: (target: HealthTarget) => void
    /** The bar's width in px. Default: fills its container. */
    width?: number
  }

  let {
    displayTabs,
    fullTabs,
    chosen = null,
    onchoose,
    tipAction,
    launchkey = true,
    failedPart = null,
    synthOn = true,
    dropouts = 0,
    bufferFrames = null,
    cpu = null,
    onhealth,
    width,
  }: Props = $props()
</script>

<header class="bar" style:width={width === undefined ? undefined : `${width}px`}>
  <span class="name">yahaha</span>
  <nav aria-label="Pages">
    <ChosenTabs tabs={displayTabs} {chosen} size="page" {tipAction} {onchoose} />
    <span class="gap"><Separator /></span>
    <ChosenTabs tabs={fullTabs} {chosen} size="page" {tipAction} {onchoose} />
  </nav>
  <div class="right">
    <Separator />
    <span
      class="launchkey"
      role="status"
      aria-label={launchkey ? 'Launchkey connected' : 'Launchkey not connected'}
    >
      <StatusDot hue={launchkey ? 'ok' : 'd'} hollow={!launchkey} />Launchkey
    </span>
    <HealthSlot {failedPart} {synthOn} {dropouts} {bufferFrames} {cpu} {tipAction} onopen={onhealth} />
  </div>
</header>

<style>
  .bar {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-8);
    box-sizing: border-box;
    height: var(--bar-height);
    border-bottom: var(--line-width) solid var(--t);
    background: var(--g);
    color: var(--t);
    font-family: var(--font-sans);
  }
  .name {
    font-size: var(--text-18);
    line-height: var(--app-name-line);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-18);
    white-space: nowrap;
  }
  nav {
    display: flex;
    align-items: stretch;
    height: var(--bar-height);
    margin-left: auto;
  }
  .gap {
    display: flex;
    align-items: center;
    margin: 0 var(--space-8);
  }
  .right {
    display: flex;
    flex: none;
    align-items: center;
    width: var(--app-bar-right-width);
    height: var(--bar-height);
  }
  .launchkey {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    margin-left: var(--space-8);
    color: var(--m);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    white-space: nowrap;
  }
</style>
