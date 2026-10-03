<!--
  StyleLine: the display's top line. The style is "the device": ◀ its name in the accent block ▶
  (the name opens the Browser), its category and time signature, a style waiting for the bar
  line, then One Touch 1-4 (the applied one the white block) and the band's sends.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import AccentBlock from '../AccentBlock/AccentBlock.svelte'
  import Button from '../Button/Button.svelte'
  import SendReadout from '../SendReadout/SendReadout.svelte'
  import WaitingChip from '../WaitingChip/WaitingChip.svelte'

  type Props = {
    /** The style's name ("Sunday Drive Pop"). */
    styleName: string
    /** The style's category ("Pop"). */
    category?: string
    /** The style's time signature ("4/4"). */
    timeSignature?: string
    /** A style waiting for the bar line, outlined in the accent. Empty: none. */
    queued?: string
    /** The applied One Touch, 1-4; 0 when none is. */
    oneTouch?: number
    /** The band's reverb send, 0-127. */
    reverb?: number
    /** The band's chorus send, 0-127. */
    chorus?: number
    /** The band's delay send, 0-127. */
    delay?: number
    /** The app's tooltip action (`use:tip`), applied to every control. */
    tipAction?: Action<HTMLElement, string>
    /** Called when ◀ is pressed (the previous style). */
    onprev?: () => void
    /** Called when ▶ is pressed (the next style). */
    onnext?: () => void
    /** Called when the style's name is pressed (opens the Browser). */
    onbrowse?: () => void
    /** Called with 1-4 when a One Touch is pressed (applies it at once). */
    ononetouch?: (n: number) => void
    /** Called when the sends are pressed (opens Effects). */
    onsends?: () => void
  }

  let {
    styleName,
    category = '',
    timeSignature = '',
    queued = '',
    oneTouch = 0,
    reverb = 0,
    chorus = 0,
    delay = 0,
    tipAction,
    onprev,
    onnext,
    onbrowse,
    ononetouch,
    onsends,
  }: Props = $props()

  const PADS = [1, 2, 3, 4] as const
  const meta = $derived([category, timeSignature].filter(Boolean).join(' · '))
  const groupName = $derived(
    `One Touch Setting (OTS): ${oneTouch ? `${oneTouch} applied` : 'none applied'}. ` +
      'Click to apply; on the Launchkey, Shift + pads 9 to 12',
  )
</script>

<div class="line">
  <Button symbol="prev" size="icon" name="Previous style (Track left)" tip="style.prev" {tipAction} onpress={onprev} />
  <span class="name">
    <AccentBlock
      label={styleName}
      as="button"
      empty="No style"
      name="{styleName.trim() || 'No style'}: open the Browser"
      tip="browser.open"
      {tipAction}
      onpress={onbrowse}
    />
  </span>
  <Button symbol="next" size="icon" name="Next style (Track right)" tip="style.next" {tipAction} onpress={onnext} />
  {#if meta}<span class="meta">{meta}</span>{/if}
  <WaitingChip label={queued} hue="a" size="line" />
  <div class="ots" role="group" aria-label={groupName}>
    <span class="ots-label" aria-hidden="true">One Touch</span>
    {#each PADS as n (n)}
      <Button
        label={String(n)}
        size="icon"
        chosen={n === oneTouch}
        pressed={n === oneTouch}
        name={n === oneTouch
          ? `One Touch ${n}, applied (Shift + pad ${n + 8})`
          : `Apply One Touch ${n} (Shift + pad ${n + 8})`}
        tip="ots.{n}"
        {tipAction}
        onpress={() => ononetouch?.(n)}
      />
    {/each}
  </div>
  <span class="sends"><SendReadout {reverb} {chorus} {delay} {tipAction} onpress={onsends} /></span>
</div>

<style>
  .line {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    width: var(--display-content-width);
    height: var(--control-height);
    white-space: nowrap;
  }
  /* The style's name keeps its width, as on the board (whose line runs 11px past 814); past
     --style-name-max it ends in the block's ellipsis. */
  .name {
    display: flex;
    flex: none;
    max-width: var(--style-name-max);
  }
  .meta {
    margin-left: var(--space-4);
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .ots {
    margin-left: auto;
    height: var(--control-height);
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
  .ots-label {
    margin-right: var(--space-4);
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .sends {
    display: flex;
    margin-left: var(--space-16);
  }
</style>
