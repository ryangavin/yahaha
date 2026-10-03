<!--
  GroupHeader: the hairline row that names a group of controls ("Faders", "Knobs", "Pads",
  "Transport", "Tempo"). The title is a real heading; what follows it (tabs, the knob page block,
  the pad page name and legend) is the parent's `children` snippet, rendered straight into the row;
  the optional page counter sits at the right end. Not a control: no click, no focus of its own.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'

  type Props = {
    /** The group's name, the heading's text ("Faders"). */
    title: string
    /** A word that qualifies the title, drawn after it as " · {detail}" ("Faders · Reverb"). */
    detail?: string
    /** The page counter at the right end: label in grey, value in white ("Page 1/6"). */
    count?: { label: string; value: string }
    /** The heading level of the title. */
    level?: 2 | 3 | 4
    /** The id put on the heading, for the parent's `aria-labelledby`. */
    id?: string
    /** A fixed width in px (the band's 654, 626, 88). Default: fills its container. */
    width?: number
    /** What follows the title; each top-level element is a flex item 12px after the one before. */
    children?: Snippet
  }

  let { title, detail, count, level = 2, id, width, children }: Props = $props()
</script>

<div class="row" style:width={width === undefined ? '100%' : `${width}px`}>
  <svelte:element this={`h${level}`} class="title" {id}
    >{title}{#if detail} · <span class="detail">{detail}</span>{/if}</svelte:element
  >
  {@render children?.()}
  {#if count}
    <span class="count">{count.label} <span class="value">{count.value}</span></span>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    box-sizing: border-box;
    height: var(--group-header-height);
    padding: 0;
    border-bottom: var(--line-width) solid var(--line);
    white-space: nowrap;
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
  }
  .title {
    flex: none;
    margin: 0;
    color: var(--m);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    line-height: normal;
  }
  .detail {
    color: var(--t);
  }
  .count {
    flex: none;
    margin-left: auto;
    color: var(--m);
    font-size: var(--text-13);
    font-weight: var(--weight-regular);
    line-height: normal;
  }
  .value {
    color: var(--t);
  }
</style>
