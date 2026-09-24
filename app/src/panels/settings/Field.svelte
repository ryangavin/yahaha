<!--
  One setting: its name, the Genos term (engraved, when there is one), an optional note
  under the control, and the control itself. `soon` marks a setting a later milestone
  adds; `mock` marks one the engine doesn't have yet (shown only on the real engine).
-->
<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    name,
    genos = null,
    note = null,
    soon = null,
    mock = false,
    inline = false,
    children,
  }: {
    name: string
    genos?: string | null
    note?: string | null
    /** "M5": coming in that milestone; the control is disabled. */
    soon?: string | null
    mock?: boolean
    /** Name and control on one line (toggles). */
    inline?: boolean
    children: Snippet
  } = $props()
</script>

<div class="field" class:inline class:soon>
  <div class="head">
    <span class="name">{name}</span>
    {#if genos && genos.toLowerCase() !== name.toLowerCase()}<span class="genos engraved">{genos}</span>{/if}
    {#if soon}<span class="badge">Coming soon · {soon}</span>{/if}
    {#if mock}<span class="badge mock">Not in the engine yet</span>{/if}
  </div>
  <div class="control">{@render children()}</div>
  {#if note}<p class="note">{note}</p>{/if}
</div>

<style>
  .field {
    display: grid;
    gap: 0.4rem;
  }
  .inline {
    grid-template-columns: 1fr auto;
    align-items: center;
    column-gap: 0.75rem;
  }
  .inline .note {
    grid-column: 1 / -1;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.25rem 0.55rem;
    min-width: 0;
  }
  .name {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.02rem;
    letter-spacing: 0.02em;
  }
  .genos {
    text-transform: uppercase;
  }
  .soon .name {
    color: var(--muted);
  }
  .badge {
    padding: 0 0.4rem;
    border: 1px dashed var(--line-strong);
    border-radius: 3px;
    font-family: var(--font-display);
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .badge.mock {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    color: var(--accent);
  }
  .control {
    min-width: 0;
  }
  .note {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
  }
</style>
