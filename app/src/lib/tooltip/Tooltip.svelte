<script lang="ts">
  import TipCard from './TipCard.svelte'
  import { tips, TOOLTIP_ID } from './tip.svelte'

  let el: HTMLDivElement | undefined = $state()
  let pos = $state({ x: 0, y: 0, above: false })

  // Place below the control, or above it when there's no room; keep it on screen.
  $effect(() => {
    const r = tips.rect
    if (!r || !el) return
    const w = el.offsetWidth
    const h = el.offsetHeight
    const gap = 10
    const above = r.bottom + gap + h > window.innerHeight && r.top - gap - h > 0
    const x = Math.max(8, Math.min(window.innerWidth - w - 8, r.left + r.width / 2 - w / 2))
    const y = above ? r.top - gap - h : Math.min(window.innerHeight - h - 8, r.bottom + gap)
    pos = { x, y, above }
  })
</script>

<div
  bind:this={el}
  id={TOOLTIP_ID}
  role="tooltip"
  class="tooltip"
  class:shown={tips.key !== null}
  style:transform={`translate(${pos.x}px, ${pos.y}px)`}
>
  {#if tips.key}<TipCard key={tips.key} />{/if}
</div>

<style>
  .tooltip {
    position: fixed;
    left: 0;
    top: 0;
    z-index: 100;
    pointer-events: none;
    padding: 0.7rem 0.85rem 0.75rem;
    background: var(--tip-bg);
    color: var(--ink);
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    box-shadow: 0 12px 32px -8px rgb(0 0 0 / 0.55);
    opacity: 0;
    visibility: hidden;
    transition: opacity 90ms ease-out;
  }
  .shown {
    opacity: 1;
    visibility: visible;
  }
  @media (prefers-reduced-motion: reduce) {
    .tooltip {
      transition: none;
    }
  }
</style>
