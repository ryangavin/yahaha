<!--
  Lock: Genos Menu › Utility › Parameter Lock (RM p.163). A locked group keeps what you set
  from the panel: Registration Memory, One Touch Setting and Playlist recalls leave it alone.
  The lock state is a setup setting (`state.paramLocks`, `setParamLock`), never in a bank.
  Only the Data List lock groups yahaha has are listed; the rest are named in the note.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import type { LockItem } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import Field from './Field.svelte'

  const locks = $derived(app.state.paramLocks)
  const GROUPS: { item: LockItem; name: string; tip: TipKey; note: string }[] = [
    { item: 'splitPoint', name: 'Split Point', tip: 'settings.param_lock_split_point', note: 'The split points stay where you set them.' },
    {
      item: 'fingeringType',
      name: 'Fingering Type',
      tip: 'settings.param_lock_fingering_type',
      note: 'The fingering type and the Chord Detection Area (Upper, Manual Bass) stay as you set them.',
    },
  ]
</script>

<p class="intro">
  A locked group keeps what you set yourself: Registration, One Touch Setting and Playlist recalls leave it alone. You can still change
  it from the panel. Locks are kept with your setup, not in a Registration bank.
</p>

{#each GROUPS as g (g.item)}
  <Field name={g.name} genos="Parameter Lock" inline note={g.note}>
    <Toggle on={locks[g.item]} tip={g.tip} onclick={() => app.send({ type: 'setParamLock', item: g.item, on: !locks[g.item] })}>
      {locks[g.item] ? 'Locked' : 'Off'}
    </Toggle>
  </Field>
{/each}

<p class="rest">
  The Genos also locks Master EQ, Reverb Type, the Reverb, Chorus and Variation return levels and the Vocal Harmony / Mic setting.
  yahaha has none of these to recall, so there is nothing to lock.
</p>

<style>
  .intro,
  .rest {
    margin: 0;
    color: var(--muted);
    font-size: 0.9rem;
    line-height: 1.4;
  }
</style>
