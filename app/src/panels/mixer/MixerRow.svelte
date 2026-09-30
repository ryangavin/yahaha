<!--
  The mixer row, always visible under the display (docs/design/redesign-b-handoff.md,
  "Mixer row, always visible"): 12 equal strips, Right 1–3 and Left then the 8 Style parts
  (Rhythm 1 … Phrase 2), each with its part colour, and the master column at the end.
  - The mixer bar across the top, always: its head is one line (the fader layer selector
    and the drawer buttons); with the details (`ui.mixer`, toggled on the master column)
    it shows the rest too, and each strip's details (StripDetail) show above its compact
    strip, so they line up per column.
  - Meters: read every 100 ms while mounted; each strip shows its channel's peak.
  - Fader badges and the Launchkey ghost: a strip whose part a Launchkey fader moves on the
    current page (and layer) shows that fader's badge (F1–F8) and ghost, as the surface
    says (`state.surface.faders`, each fader's `set`): Panel page, faders 1–4 = the keyboard
    parts (unless the live rack routes one to a rack control); Style page, faders 1–8 = the
    Style parts (none in PAN: they have no pan). Shift changes only the buttons under the
    faders, never which part a fader moves, so the badges stay with Shift held.
  - Width: the 12 strips and the master share the row's whole width equally (about 95 px
    each at a 1280 px window); on a narrow window they shrink to 56 px, and only below that
    would the strips scroll sideways inside the row (never the page).
-->
<script module lang="ts">
  import type { SurfaceFader } from '../../lib/api/types'

  /** The strip (0–11) a Launchkey fader moves, or null (a group level, a rack control,
   *  the master, unused). */
  export function stripOfFader(f: SurfaceFader | undefined): number | null {
    const set = f?.set
    if (!set) return null
    switch (set.type) {
      case 'setPartVolume':
      case 'setPartPan':
      case 'setPartSend':
        return set.part
      case 'setStylePartVolume':
      case 'setStylePartSend':
        return 4 + set.part
      default:
        return null
    }
  }
</script>

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

  /** For each strip, the Launchkey fader (0–7) that moves it on the current page, or -1. */
  const faderOf = $derived.by(() => {
    const of = STRIPS.map(() => -1)
    surface.faders.slice(0, 8).forEach((f, i) => {
      const s = stripOfFader(f)
      if (s !== null && of[s] === -1) of[s] = i
    })
    return of
  })
  /** Where the Launchkey fader driving strip `i` sits, if the live page drives it. */
  const hwOf = (i: number): number | null => (faderOf[i] < 0 ? null : (surface.faders[faderOf[i]]?.position ?? null))
  const badgeOf = (i: number): string | null => (faderOf[i] < 0 ? null : `F${faderOf[i] + 1}`)
  function levelAt(i: number): number | null {
    const ch = channelOf(i)
    return ch === null ? null : levelOf(meters, ch)
  }
</script>

<section class="mixer-row" aria-label="Mixer">
  <!-- Always: its head (one line) shows the layer selector; it hides its own details
       while ui.mixer is off. -->
  <MixerBar {meters} />
  <div class="body">
    <!-- The details bar's Panel/Style tabs control this (the Launchkey fader page). -->
    <div class="strips" id="mixer-strips" role={ui.mixer ? 'tabpanel' : undefined} aria-labelledby={ui.mixer ? `mixer-tab-${page}` : undefined}>
      {#each STRIPS as i (i)}
        <div class="col">
          {#if ui.mixer}
            <StripDetail part={i} {meters} />
          {/if}
          <div class="compact">
            <Strip part={i} level={levelAt(i)} hw={hwOf(i)} badge={badgeOf(i)} />
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
  /* Equal shares of the row with the master: about 95 px at a 1280 px window, 140 px at
     1920; never under 56 px (then the strips scroll). */
  .col {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    flex: 1 1 0;
    min-width: 56px;
    min-height: 0;
  }
  /* Never squashed by the details above it or a short row: a whole strip (name, voice,
     insert slot, tags, the fader at its least with its readout, On/Solo) needs 200 px. */
  .compact {
    flex: 1 1 0;
    min-height: 12.5rem;
  }
  /* The master column takes one strip's share (the strips' flex holds 12 of 13). */
  .master {
    display: flex;
    flex: 0 0 calc((100% - 12 * 0.2rem) / 13);
    min-width: 4.5rem;
    min-height: 0;
  }
</style>
