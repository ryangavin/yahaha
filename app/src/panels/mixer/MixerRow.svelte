<!--
  The mixer row, always visible under the display (docs/design/redesign-b-handoff.md,
  "Mixer row, always visible"): 12 equal strips, Right 1–3 and Left then the 8 Style parts
  (Rhythm 1 … Phrase 2), each with its part colour, and the master column at the end.
  - Details (`ui.mixer`, toggled on the master column): the mixer bar across the top, and
    each strip's details (StripDetail) above its compact strip, so they line up per column.
  - Meters: read every 100 ms while mounted; each strip shows its channel's peak.
  - The Launchkey ghost shows only on the parts the live fader page drives: Panel page,
    faders 1–4 = the keyboard parts; Style page, faders 1–8 = the Style parts.
  - Narrow windows: strips shrink to about 56 px, then the strips scroll sideways inside
    the row (never the page); the master column stays put.
-->
<script lang="ts">
  import type { Meters } from '../../lib/api/types'
  import { app, ui } from '../../lib/store.svelte'
  import { surfaceOf } from '../../lib/surface'
  import Strip from './Strip.svelte'
  import StripDetail from './StripDetail.svelte'
  import MixerBar from './MixerBar.svelte'
  import MasterStrip from './MasterStrip.svelte'
  import { isKeyboard, levelOf, METER_POLL_MS, STRIPS, stylePartOf } from './parts'

  let meters = $state.raw<Meters | null>(null)
  $effect(() => {
    let live = true
    const read = () => app.meters().then((m) => live && (meters = m))
    read()
    const t = setInterval(read, METER_POLL_MS)
    return () => {
      live = false
      clearInterval(t)
    }
  })

  const surface = $derived(surfaceOf(app.state, app.library))
  const page = $derived(app.state.mixer.faderPage)

  /** Strip `i`'s MIDI channel (1-based). */
  const channelOf = (i: number): number | null =>
    isKeyboard(i) ? (app.state.keyboardParts[i]?.channel ?? null) : (app.state.mixer.styleParts[stylePartOf(i)]?.channel ?? null)

  /** Where the Launchkey fader driving strip `i` sits, if the live page drives it. */
  function hwOf(i: number): number | null {
    const f = isKeyboard(i) ? (page === 'panel' ? i : -1) : page === 'style' ? stylePartOf(i) : -1
    return f < 0 ? null : (surface.faders[f]?.position ?? null)
  }
  function levelAt(i: number): number | null {
    const ch = channelOf(i)
    return ch === null ? null : levelOf(meters, ch)
  }
</script>

<section class="mixer-row" aria-label="Mixer">
  {#if ui.mixer}
    <MixerBar {meters} />
  {/if}
  <div class="body">
    <!-- The details bar's Panel/Style tabs control this (the Launchkey fader page). -->
    <div class="strips" id="mixer-strips" role={ui.mixer ? 'tabpanel' : undefined} aria-labelledby={ui.mixer ? `mixer-tab-${page}` : undefined}>
      {#each STRIPS as i (i)}
        <div class="col">
          {#if ui.mixer}
            <StripDetail part={i} {meters} />
          {/if}
          <div class="compact">
            <Strip part={i} level={levelAt(i)} hw={hwOf(i)} />
          </div>
        </div>
      {/each}
    </div>
    <div class="master">
      <MasterStrip />
    </div>
  </div>
</section>

<style>
  .mixer-row {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }
  .body {
    display: flex;
    gap: 0.3rem;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
  }
  .strips {
    display: flex;
    gap: 0.2rem;
    flex: 1 1 auto;
    min-width: 0;
    /* Sideways below 56 px a strip; up and down when the details don't fit the window. */
    overflow: auto;
  }
  /* About 80–90 px at a 1440 px window; never under 56 px (then the strips scroll). */
  .col {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    flex: 1 1 0;
    min-width: 56px;
    max-width: 92px;
    min-height: 0;
  }
  /* Never squashed by the details above it: a whole strip needs about 200 px. */
  .compact {
    flex: 1 1 0;
    min-height: 13.75rem;
  }
  .master {
    display: flex;
    flex: none;
    min-height: 0;
  }
</style>
