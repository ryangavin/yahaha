<!--
  Single Finger's four shapes, drawn on a small keyboard (A below the root to E above),
  with C as the example root: the root alone for major, plus a black key to its left for
  minor, a white key to its left for 7th, and both for m7. Static and decorative: the
  fingering type's tooltip says the same in words.
-->
<script lang="ts">
  // One octave window: A Bb B C C# D Eb E. White keys by index, black keys after a white.
  const WHITES = ['A', 'B', 'C', 'D', 'E']
  const BLACKS: { name: string; after: number }[] = [
    { name: 'Bb', after: 0 },
    { name: 'C#', after: 2 },
    { name: 'Eb', after: 3 },
  ]
  const SHAPES: { chord: string; kind: string; keys: string[] }[] = [
    { chord: 'C', kind: 'major', keys: ['C'] },
    { chord: 'Cm', kind: '+ black key left', keys: ['Bb', 'C'] },
    { chord: 'C7', kind: '+ white key left', keys: ['B', 'C'] },
    { chord: 'Cm7', kind: '+ both', keys: ['Bb', 'B', 'C'] },
  ]
  const WW = 14
  const WH = 40
  const BW = 9
  const BH = 25
</script>

<figure class="chart">
  <figcaption class="engraved">Single Finger shapes</figcaption>
  <div class="grid">
    {#each SHAPES as s (s.chord)}
      <div class="shape">
        <svg viewBox="0 0 {WHITES.length * WW + 2} {WH + 2}" role="img" aria-label="{s.chord}: {s.keys.join(' + ')}">
          {#each WHITES as w, i (w)}
            <rect class="white" class:on={s.keys.includes(w)} class:root={w === 'C'} x={1 + i * WW} y="1" width={WW} height={WH} rx="1.5" />
          {/each}
          {#each BLACKS as b (b.name)}
            <rect class="black" class:on={s.keys.includes(b.name)} x={1 + (b.after + 1) * WW - BW / 2} y="1" width={BW} height={BH} rx="1" />
          {/each}
        </svg>
        <div class="name"><b>{s.chord}</b> <span>{s.kind}</span></div>
      </div>
    {/each}
  </div>
</figure>

<style>
  .chart {
    margin: 0;
    padding: 0.6rem 0.7rem 0.55rem;
    border: 1px solid var(--line);
    border-radius: var(--r-key);
    background: color-mix(in srgb, var(--well) 22%, transparent);
  }
  figcaption {
    margin-bottom: 0.45rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.6rem;
  }
  .shape {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    min-width: 0;
  }
  svg {
    width: 100%;
    max-width: 5.2rem;
    height: auto;
    display: block;
  }
  .white {
    fill: #eef0f2;
    stroke: #6f7780;
    stroke-width: 0.8;
  }
  .black {
    fill: #1b1e22;
    stroke: #0b0c0e;
    stroke-width: 0.6;
  }
  .white.on {
    fill: color-mix(in srgb, var(--accent) 55%, #eef0f2);
  }
  .white.on.root {
    fill: var(--accent);
  }
  .black.on {
    fill: color-mix(in srgb, var(--accent) 80%, #1b1e22);
  }
  .name {
    font-family: var(--font-display);
    font-size: 0.8rem;
    color: var(--muted);
    text-align: center;
    line-height: 1.15;
  }
  .name b {
    color: var(--ink);
    font-size: 0.95rem;
  }
  .name span {
    display: block;
  }
</style>
