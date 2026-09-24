<!--
  Sections: Intro I–III, Main A–D (with fills), Break, Ending I–III, and the transport
  (Sync Start, Sync Stop, Auto Fill, Start/Stop). Every lamp is the engine's own page-1
  pad (`state.transport.lamps`), so the screen and the Launchkey always agree, and every
  key sends that pad's `action`.

  REFERENCE PANEL: see app/CONTRIBUTING.md.
-->
<script lang="ts">
  import { tipFor } from '../../help/actions'
  import { BREAK, ENDINGS, FILLS, INTROS, MAINS, type Pad } from '../../lib/api/types'
  import { LAMP, lamp } from '../../lib/leds'
  import { app, clock } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import LampKey from '../../lib/ui/LampKey.svelte'
  import Panel from '../../lib/ui/Panel.svelte'

  const s = $derived(app.state)
  const t = $derived(s.transport)
  const beats = $derived(clock.beats)
  const press = (p: Pad) => () => p.action && app.send(p.action)

  /** The small line under a section key: what's about to happen to it. */
  function sub(name: string): string {
    if (t.queued === name) return 'next'
    const intro = INTROS.indexOf(name)
    if (intro >= 0 && t.pendingIntro === intro) return 'armed'
    return ''
  }
  function mainSub(i: number): string {
    if (t.section === FILLS[i]) return 'fill'
    if (t.queued === FILLS[i]) return 'fill next'
    return sub(MAINS[i])
  }

  const ROMAN = ['I', 'II', 'III']
  const LETTERS = ['A', 'B', 'C', 'D']
</script>

{#snippet key(p: Pad, label: string, subtext: string, size: 'm' | 'l' = 'm')}
  <LampKey look={p} {beats} tip={tipFor(p.action)} sub={subtext} {size} onclick={press(p)}>{label}</LampKey>
{/snippet}

<Panel id="sections" title="Sections">
  {#snippet actions()}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
    <span class="legend" tabindex="0" role="note" use:tip={'section.lamps'}>
      <span class="sw dim"></span>available
      <span class="sw bright"></span>playing
      <span class="sw flash"></span>next
      <span class="sw pulse"></span>armed
    </span>
  {/snippet}

  <div class="grid">
    <div class="row intros" role="group" aria-label="Intros">
      {#each ROMAN as r, i (r)}{@render key(lamp(s, LAMP.intro[i]), `Intro ${r}`, sub(INTROS[i]))}{/each}
    </div>
    <div class="row endings" role="group" aria-label="Endings">
      {#each ROMAN as r, i (r)}{@render key(lamp(s, LAMP.ending[i]), `Ending ${r}`, sub(ENDINGS[i]))}{/each}
    </div>

    <div class="row mains" role="group" aria-label="Main variations">
      {#each LETTERS as l, i (l)}{@render key(lamp(s, LAMP.main[i]), `Main ${l}`, mainSub(i), 'l')}{/each}
    </div>
    <div class="row brk">
      {@render key(lamp(s, LAMP.break), 'Break', sub(BREAK), 'l')}
    </div>

    <div class="row transport" role="group" aria-label="Transport">
      {@render key(lamp(s, LAMP.syncStart), 'Sync Start', t.syncStart ? 'waiting' : '')}
      {@render key(lamp(s, LAMP.syncStop), 'Sync Stop', t.syncStopAvailable ? '' : 'n/a')}
      {@render key(lamp(s, LAMP.autoFill), 'Auto Fill', '')}
    </div>
    <div class="row start">
      {@render key(lamp(s, LAMP.startStop), 'Start / Stop', t.running ? 'playing' : 'stopped', 'l')}
    </div>
  </div>
</Panel>

<style>
  /* Two columns: Intros/Mains/Sync on the left, Endings/Break/Start on the right,
     the way the pads are laid out (Intros left of Endings). */
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 4fr) minmax(0, 3fr);
    gap: 0.6rem 1rem;
  }
  .row {
    display: grid;
    gap: 0.45rem;
  }
  .intros,
  .endings,
  .transport {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .mains {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
  .brk,
  .start {
    grid-template-columns: minmax(0, 1fr);
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: var(--fs-small);
    color: var(--muted);
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
  }
  .sw {
    width: 0.9rem;
    height: 0.35rem;
    border-radius: 2px;
    margin-left: 0.35rem;
    background: rgb(0 255 32);
  }
  .sw.dim {
    opacity: 0.25;
  }
  .sw.flash {
    background: linear-gradient(90deg, rgb(0 255 32) 50%, rgb(0 255 32 / 0.2) 50%);
  }
  .sw.pulse {
    background: linear-gradient(90deg, rgb(0 255 32 / 0.25), rgb(0 255 32), rgb(0 255 32 / 0.25));
  }
  @media (max-width: 640px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
    .mains {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .legend {
      display: none;
    }
  }
</style>
