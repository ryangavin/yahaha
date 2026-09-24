<!--
  The opt-in pop-up tip (the help footer's "Pop-up" switch): the same entry as the footer,
  next to the control. Off by default, since it covers the instrument while you play. The
  footer carries the accessible description, so this copy is hidden from screen readers.
-->
<script lang="ts">
  import TipCard from './TipCard.svelte'
  import { tips } from './tip.svelte'

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
  aria-hidden="true"
  class="tooltip"
  class:shown={tips.key !== null && tips.rect !== null}
  style:transform={`translate(${pos.x}px, ${pos.y}px)`}
>
  {#if tips.key && tips.rect}<TipCard key={tips.key} />{/if}
</div>

<style>
  .tooltip {
    position: fixed;
    left: 0;
    top: 0;
    z-index: 100;
    pointer-events: none;
    padding: 0.7rem 0.85rem 0.75rem;
    /* Same brushed-metal material as the instrument, a touch lighter, so it reads as a
       plate lifted off the panel. */
    background: linear-gradient(180deg, color-mix(in srgb, var(--chassis-hi) 85%, white 6%), var(--chassis-lo));
    color: var(--ink);
    border: 1px solid var(--seam);
    border-radius: 8px;
    box-shadow:
      inset 0 1px 0 var(--chassis-edge),
      0 14px 34px -10px rgb(0 0 0 / 0.65);
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
