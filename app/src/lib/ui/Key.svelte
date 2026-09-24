<!--
  The basic button. Every Key needs a catalog `tip`; it shows its first keyboard shortcut
  as a small keycap hint unless `hint={false}`.
  `unavailable` is the dark state (e.g. a section this style doesn't have): still
  hoverable for its tooltip, but it looks off and says so to screen readers.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import { TIPS, keyLabel, type TipKey } from '../../help/tooltips'
  import { tip as tipAction } from '../tooltip/tip.svelte'

  let {
    tip,
    onclick,
    children,
    pressed,
    unavailable = false,
    hint = true,
    size = 'm',
    label,
  }: {
    tip: TipKey
    onclick: () => void
    children: Snippet
    pressed?: boolean
    unavailable?: boolean
    hint?: boolean
    size?: 's' | 'm' | 'l'
    /** Accessible name when the visible content isn't enough (icons). */
    label?: string
  } = $props()

  const keyHint = $derived.by(() => {
    const t = TIPS[tip]
    const k = (t.app_keys ?? t.keys)[0]
    return hint && k ? keyLabel(k) : null
  })
</script>

<button
  type="button"
  class="key {size}"
  class:unavailable
  aria-pressed={pressed}
  aria-disabled={unavailable || undefined}
  aria-label={label}
  use:tipAction={tip}
  onclick={() => !unavailable && onclick()}
>
  {@render children()}
  {#if keyHint}<span class="hint" aria-hidden="true">{keyHint}</span>{/if}
</button>

<style>
  .key {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    min-height: 2.5rem;
    min-width: 2.5rem;
    padding: 0.3rem 0.75rem;
    background: var(--raised);
    border: 1px solid var(--line);
    border-bottom-color: var(--line-strong);
    border-radius: var(--r-key);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: var(--fs-label);
    letter-spacing: 0.02em;
    white-space: nowrap;
  }
  .key:hover {
    border-color: var(--line-strong);
  }
  .key:active {
    transform: translateY(1px);
  }
  .key[aria-pressed='true'] {
    background: var(--accent);
    color: var(--accent-ink);
    border-color: var(--accent);
  }
  .s {
    min-height: 2rem;
    min-width: 2rem;
    padding: 0.15rem 0.5rem;
    font-size: var(--fs-body);
  }
  .l {
    min-height: 3.25rem;
    padding: 0.4rem 1.1rem;
    font-size: 1.2rem;
  }
  .unavailable {
    color: var(--muted);
    opacity: 0.55;
    cursor: default;
  }
  .hint {
    font-family: var(--font-body);
    font-weight: 500;
    font-size: 0.7rem;
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 3px;
    padding: 0 0.25em;
    line-height: 1.3;
  }
  .key[aria-pressed='true'] .hint {
    color: inherit;
    border-color: currentColor;
  }
</style>
