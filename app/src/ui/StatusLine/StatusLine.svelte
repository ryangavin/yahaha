<!--
  StatusLine: the one line between the band and the keys that says what the app just did or
  refused (`state.message`). An error leads with the ⚠; a click, Enter or Space asks the parent to
  clear it (`onclear`). Empty, the 20px row stays so the keys never move. The contents are keyed
  on `seq`, so the same text sent twice is announced twice while the button keeps its focus.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'

  type Props = {
    /** `message.text`. `null` or `''`: no message, the line is empty. */
    text?: string | null
    /** `message.error`: the ⚠ shows before the text. Ignored while there is no text. */
    error?: boolean
    /** `message.seq`. A new value re-creates the button's contents, so a repeat is announced again. */
    seq?: number
    /** A fixed width in px. Unset, the line fills its container. */
    width?: number
    /** The tooltip key from `app/src/help/tooltips.ts`, as `data-tip` on the button. `null` turns it off. */
    tip?: string | null
    /** The app's `use:tip` action, passed in by the wiring; applied with `tip` when both are set. */
    tipAction?: Action<HTMLElement, string>
    /** Called on click, Enter or Space on the line's button; the parent sends `clearMessage`. */
    onclear?: () => void
  }

  let { text = null, error = false, seq = 0, width, tip = 'display.status', tipAction, onclear }: Props = $props()

  /** Applies the parent's tooltip action when both it and a key are given. */
  const tipped: Action<HTMLElement, string | null | undefined> = (node, key) => {
    if (!tipAction || key == null) return
    const handle = tipAction(node, key)
    return {
      update: (next) => {
        if (next != null) handle?.update?.(next)
      },
      destroy: () => handle?.destroy?.(),
    }
  }
</script>

<p class="status" role="status" aria-live="polite" style:width={width === undefined ? undefined : `${width}px`}>
  {#if text}
    <button type="button" class="line" data-tip={tip ?? undefined} use:tipped={tip} onclick={() => onclear?.()}>
      {#key seq}
        {#if error}
          <svg aria-hidden="true" data-hue="warn" class="warn" width="12" height="12" viewBox="0 0 12 12">
            <path d="M6 1.5 L11 10.5 H1 Z" /><path d="M6 5 V7.4" /><circle cx="6" cy="8.9" r="0.6" />
          </svg>
        {/if}
        <!-- The space after "Error:" sits outside the hidden span: an accessible-name computation
             trims each element's text, so a space inside it would be lost ("Error:Can't"). At the
             line's start the visible space collapses away. -->
        <span class="text">{#if error}<span class="hidden-word">Error:</span>{/if}{error ? ` ${text}` : text}</span>
      {/key}
    </button>
  {/if}
</p>

<style>
  .status {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    width: 100%;
    min-width: 0;
    height: var(--space-20);
    margin: 0;
    overflow: visible;
    background: transparent;
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    line-height: var(--space-20);
    font-variant-numeric: tabular-nums;
  }
  .line {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    max-width: 100%;
    min-width: 0;
    height: var(--space-20);
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--t);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .line:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
  .warn {
    flex: none;
    fill: none;
    stroke: var(--warn);
    stroke-width: 1;
    stroke-linejoin: round;
  }
  .warn circle {
    fill: var(--warn);
    stroke: none;
  }
  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hidden-word {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
