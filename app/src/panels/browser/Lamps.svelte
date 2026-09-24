<!--
  A style's sections as small lamps, grouped like the Launchkey's section pads:
  Intro I–III · Main A–D · Break · Ending I–III. Lit = the style has it. Colours are the
  section pads' own (from the engine's lamps), so they match the hardware.
-->
<script lang="ts">
  import { sectionLamps } from './model'

  let { summary, colours }: { summary: string; colours: { intro: string; main: string; brk: string; ending: string } } = $props()

  const l = $derived(sectionLamps(summary))
</script>

<span class="lamps" role="img" aria-label={summary || 'no sections'}>
  <span class="grp">{#each l.intro as on, i (i)}<i class:on style:--c={colours.intro}></i>{/each}</span>
  <span class="grp">{#each l.main as on, i (i)}<i class:on style:--c={colours.main}></i>{/each}</span>
  <span class="grp"><i class:on={l.brk} style:--c={colours.brk}></i></span>
  <span class="grp">{#each l.ending as on, i (i)}<i class:on style:--c={colours.ending}></i>{/each}</span>
</span>

<style>
  .lamps {
    display: inline-flex;
    gap: 5px;
    align-items: center;
  }
  .grp {
    display: inline-flex;
    gap: 2px;
  }
  i {
    width: 7px;
    height: 7px;
    border-radius: 2px;
    background: rgb(255 255 255 / 0.07);
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.6);
  }
  i.on {
    background: var(--c);
    box-shadow: 0 0 4px var(--c);
  }
</style>
