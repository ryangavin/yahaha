<!--
  SoundRow: the display's bottom row, Push's track-name row. The rack readout (with its modified
  dot) and the four keyboard parts, each a sound cell with its marks. Every press calls back with
  what was pressed; nothing here opens anything itself.
-->
<script lang="ts">
  import { marksText } from '../PartMarks/marks'
  import PartMarks from '../PartMarks/PartMarks.svelte'
  import RackCell from '../RackCell/RackCell.svelte'
  import SoundCell from '../SoundCell/SoundCell.svelte'
  import StatusDot from '../StatusDot/StatusDot.svelte'
  import type { Action } from 'svelte/action'

  type PartId = 'right1' | 'right2' | 'right3' | 'left'

  type Part = {
    /** Which part: also its tooltip key (`part.<id>.select`). */
    id: PartId
    /** Its short name ("R1"). */
    part: string
    /** Its full name ("Right 1"). */
    partName: string
    /** Its hue. */
    hue: 'r1' | 'r2' | 'r3' | 'l'
    /** The sound's number in its list. */
    number: string
    /** The sound's name. */
    sound: string
    /** The part is off. */
    off?: boolean
    /** The sound was changed since it was loaded. */
    edited?: boolean
    /** The part's plugin isn't installed. */
    missing?: boolean
    /** The part's plugin failed. */
    failed?: boolean
    /** The part plays the Style's bass. */
    bass?: boolean
  }

  type Props = {
    /** The rack's name ("Sunday drive"). */
    rack: string
    /** The Quick Rack slot it's on ("A1"). Empty: none. */
    slot?: string
    /** The rack was changed since it was saved: the 5px dot after its name. */
    modified?: boolean
    /** The four keyboard parts, in order. */
    parts: Part[]
    /** The app's tooltip action (`use:tip`), applied to every control. */
    tipAction?: Action<HTMLElement, string>
    /** Called when the rack is pressed (opens the Rack page). */
    onrack?: () => void
    /** Called with the part's id when its short name is pressed (opens Channel). */
    onpart?: (id: PartId) => void
    /** Called with the part's id when its sound is pressed (opens the quick sound list). */
    onsound?: (id: PartId) => void
  }

  let { rack, slot = '', modified = false, parts, tipAction, onrack, onpart, onsound }: Props = $props()

  const rackName = $derived(
    `Rack: ${rack.trim() || 'No rack'}${modified ? ', modified' : ''}${slot ? `, on Quick Rack ${slot}` : ''}. Opens the Rack page`,
  )
</script>

<div class="row">
  <RackCell {rack} {slot} name={rackName} tip="nav.rack" {tipAction} onpress={onrack}>
    {#snippet mark()}
      {#if modified}<StatusDot hue="t" size="sm" glow={false} />{/if}
    {/snippet}
  </RackCell>
  {#each parts as p (p.id)}
    <SoundCell
      part={p.part}
      partName={p.partName}
      hue={p.hue}
      number={p.number}
      sound={p.sound}
      off={p.off}
      soundName="{p.partName} sound: {p.number} {p.sound}{marksText(p)}. Opens the quick sound list"
      partTip="part.{p.id}.select"
      {tipAction}
      onpart={() => onpart?.(p.id)}
      onsound={() => onsound?.(p.id)}
    >
      {#snippet marks()}
        <PartMarks edited={p.edited} missing={p.missing} failed={p.failed} off={p.off} bass={p.bass} />
      {/snippet}
    </SoundCell>
  {/each}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: var(--rack-cell-width) repeat(4, minmax(0, 1fr));
    gap: var(--space-8);
    width: var(--display-content-width);
    height: var(--sound-row-height);
  }
</style>
