<!--
  BeatBlocks: the count row's beat blocks, one 24px block per beat of the bar. Past beats are
  `--past`, the current beat is solid in the section's hue with its glow, beats still to come are
  the plain button face, and beat one always carries a white top edge. The parent passes the
  beat; nothing here moves on its own. Decorative: the count row names the beat in words.
-->
<script lang="ts">
  type Props = {
    /** Beats in the bar (the time signature's top number). */
    beats?: number
    /** The current beat, 1-based. 0: stopped, every block still to come. */
    beat?: number
    /** The section hue of the current beat. */
    hue?: 'intro' | 'main' | 'ending' | 'brk' | 'fill'
  }

  let { beats = 4, beat = 0, hue = 'main' }: Props = $props()

  const blocks = $derived(Array.from({ length: Math.max(0, beats) }, (_, i) => i + 1))

  function state(n: number): 'past' | 'current' | 'future' {
    if (n === beat) return 'current'
    return n < beat ? 'past' : 'future'
  }
</script>

<span class="beats" aria-hidden="true">
  {#each blocks as n (n)}
    <span
      class="block {state(n)}"
      class:one={n === 1}
      style:--hue={state(n) === 'current' ? `var(--${hue})` : undefined}
      style:--glow={state(n) === 'current' ? `var(--beat-glow-${hue})` : undefined}
      data-beat={n}
      data-state={state(n)}
    ></span>
  {/each}
</span>

<style>
  .beats {
    display: inline-flex;
    flex: none;
    gap: var(--space-4);
  }
  .block {
    width: var(--beat-size);
    height: var(--beat-size);
    border-radius: var(--beat-radius);
    background: var(--btn);
  }
  .past {
    background: var(--past);
  }
  .current {
    background: var(--hue);
    box-shadow: var(--glow);
  }
  .one {
    box-shadow: inset 0 var(--beat-edge) 0 var(--beat-edge-colour);
  }
  .one.current {
    box-shadow:
      inset 0 var(--beat-edge) 0 var(--beat-edge-colour),
      var(--glow);
  }
</style>
