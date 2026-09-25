<!--
  The Registration panel (right-side drawer, open while `ui.regist`), four pages:

  - Bank: the bank file (pick, new, save / save as) and its ten buttons as Regist Bank
    Info shows them (style, tempo, voices): recall, memorize here, rename, clear.
  - Groups: what Memory stores (the Memory window) and what Freeze keeps (Regist Freeze).
  - Sequence: the bank's Registration Sequence (steps, what happens at the end, on/off).
  - Playlist: the set list: files, records (load, button to recall, move, delete),
    sorting, adding the bank or style in use.

  State: registration, playlist. Commands: the Registration and Playlist groups
  (lib/api/registration.ts).
-->
<script lang="ts">
  import { REGIST_GROUPS, SEQUENCE_ENDS, sameFile, type PlaylistRecord, type PlaylistSort, type RegistGroup } from '../../lib/api/registration'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import { REGIST_TIPS, buttonLook, buttonSummary } from './regist'

  const r = $derived(app.state.registration)
  const pl = $derived(app.state.playlist)
  const beats = $derived(clock.beats)

  const TABS = [
    { id: 'bank', name: 'Bank' },
    { id: 'groups', name: 'Memory & Freeze' },
    { id: 'sequence', name: 'Sequence' },
    { id: 'playlist', name: 'Playlist' },
  ] as const
  const SORTS: { id: PlaylistSort; name: string }[] = [
    { id: 'normal', name: 'Normal' },
    { id: 'aToZ', name: 'A → Z' },
    { id: 'zToA', name: 'Z → A' },
  ]

  let bankName = $state('')
  let listName = $state('')

  const has = (list: RegistGroup[], g: RegistGroup) => list.includes(g)
  const sorted = $derived(pl.sort !== 'normal')

  function rename(i: number, e: Event) {
    const name = (e.currentTarget as HTMLInputElement).value
    if (name !== r.buttons[i].name) app.send({ type: 'renameRegist', index: i, name })
  }
  /** The typed name saves to another bank's (playlist's) file, as the backend names files
   * (`fileStem`: "A:B" is "A_B") and the Mac compares them (ignoring case): Save is
   * refused, Overwrite replaces that file. */
  const clash = (name: string, files: { name: string; path: string }[], own: string | null) =>
    !!name.trim() && files.some((f) => sameFile(f.name, name) && f.path !== own)
  const bankClash = $derived(clash(bankName, r.banks, r.bank.path))
  const listClash = $derived(clash(listName, pl.playlists, pl.path))

  function saveBank(overwrite = false) {
    app.send({ type: 'saveRegistBank', name: bankName.trim() || null, ...(overwrite ? { overwrite } : {}) })
    if (overwrite || !bankClash) bankName = ''
  }
  function savePlaylist(overwrite = false) {
    app.send({ type: 'savePlaylist', name: listName.trim() || null, ...(overwrite ? { overwrite } : {}) })
    if (overwrite || !listClash) listName = ''
  }
  function removeStep(k: number) {
    app.send({ type: 'setRegistSequence', steps: r.sequence.steps.filter((_, j) => j !== k), end: r.sequence.end })
  }
  function setRegist(index: number, record: PlaylistRecord, value: string) {
    if (record.kind !== 'bank') return
    app.send({ type: 'setPlaylistRecord', index, record: { ...record, regist: value === '' ? null : Number(value) } })
  }
</script>

<Overlay id="regist" title="Registration" closeTip="drawer.close" onclose={() => (ui.regist = false)}>
  <div class="tabs" role="tablist" aria-label="Registration pages">
    {#each TABS as t (t.id)}
      <button type="button" role="tab" class="tab mat-raised" class:pressed={ui.registTab === t.id} aria-selected={ui.registTab === t.id} use:tip={'regist.tab'} onclick={() => (ui.registTab = t.id)}>{t.name}</button>
    {/each}
  </div>

  {#if ui.registTab === 'bank'}
    <section class="block" aria-label="Bank">
      <div class="row">
        <select class="field" aria-label="Bank" use:tip={'regist.bank'} value={r.bank.path ?? ''} onchange={(e) => e.currentTarget.value && app.send({ type: 'selectRegistBank', path: e.currentTarget.value })}>
          {#if !r.bank.path}<option value="">{r.bank.name} (not saved)</option>{/if}
          {#each r.banks as b (b.path)}<option value={b.path}>{b.name}</option>{/each}
        </select>
        <HwButton tip="regist.new_bank" onclick={() => app.send({ type: 'newRegistBank' })}>New</HwButton>
      </div>
      <div class="row">
        <input class="field" type="text" aria-label="Bank name" placeholder={r.bank.path ? `Save as… (${r.bank.name})` : 'Name this bank'} use:tip={'regist.bank_name'} bind:value={bankName} />
        <HwButton tip="regist.save_bank" onclick={() => saveBank()}>Save{r.bank.dirty ? ' *' : ''}</HwButton>
        {#if bankClash}<HwButton tip="regist.overwrite_bank" onclick={() => saveBank(true)}>Overwrite</HwButton>{/if}
      </div>
      <p class="note">{r.folder ? `Banks live in ${r.folder}.` : 'This session has no Registration folder: banks can be used but not saved.'}</p>
    </section>

    <ol class="buttons">
      {#each r.buttons as b, i (b.index)}
        <li class="button" class:current={r.selected === i}>
          <HwButton tip={REGIST_TIPS[i]} led={buttonLook(r, i)} {beats} shape="square" label="Registration {i + 1}" onclick={() => app.send({ type: 'pressRegist', index: i })}>{i + 1}</HwButton>
          <div class="info">
            {#if b.stored}
              <input class="field name" type="text" aria-label="Name of Registration {i + 1}" value={b.name} use:tip={'regist.rename'} onchange={(e) => rename(i, e)} />
              <button type="button" class="summary" use:tip={'regist.info'} onclick={() => app.send({ type: 'recallRegist', index: i })}>{buttonSummary(r, i)}</button>
            {:else}
              <span class="empty engraved">empty</span>
            {/if}
          </div>
          <div class="actions">
            <HwButton tip="regist.memorize_here" onclick={() => app.send({ type: 'memorizeRegist', index: i })}>Memorize</HwButton>
            {#if b.stored}<HwButton tip="regist.clear" onclick={() => app.send({ type: 'clearRegist', index: i })}>Clear</HwButton>{/if}
          </div>
        </li>
      {/each}
    </ol>
  {:else if ui.registTab === 'groups'}
    <section class="block" aria-label="Memory groups">
      <h3 class="engraved">Memory stores</h3>
      <div class="grid">
        {#each REGIST_GROUPS as g (g.id)}
          <Toggle tip="regist.memorize_group" on={has(r.memorizeGroups, g.id)} onclick={() => app.send({ type: 'setMemorizeGroup', group: g.id, on: !has(r.memorizeGroups, g.id) })}>{g.name}</Toggle>
        {/each}
      </div>
    </section>
    <section class="block" aria-label="Freeze">
      <h3 class="engraved">Freeze keeps</h3>
      <div class="row"><Toggle tip="regist.freeze" on={r.freeze} onclick={() => app.send({ type: 'toggleFreeze' })}>Freeze {r.freeze ? 'on' : 'off'}</Toggle></div>
      <div class="grid">
        {#each REGIST_GROUPS as g (g.id)}
          <Toggle tip="regist.freeze_group" on={has(r.freezeGroups, g.id)} onclick={() => app.send({ type: 'setFreezeGroup', group: g.id, on: !has(r.freezeGroups, g.id) })}>{g.name}</Toggle>
        {/each}
      </div>
      <p class="note">Chord Looper and Live Control are stored once those features are in. Assignable holds the Fade In/Out times.</p>
    </section>
  {:else if ui.registTab === 'sequence'}
    <section class="block" aria-label="Registration Sequence">
      <div class="row">
        <Toggle tip="regist.sequence_on" on={r.sequence.on} onclick={() => app.send({ type: 'toggleRegistSequence' })}>Sequence {r.sequence.on ? 'on' : 'off'}</Toggle>
        <HwButton tip="regist.seq_prev" label="Regist −" onclick={() => app.send({ type: 'stepRegistSequence', delta: -1 })}>Regist −</HwButton>
        <HwButton tip="regist.seq_next" label="Regist +" onclick={() => app.send({ type: 'stepRegistSequence', delta: 1 })}>Regist +</HwButton>
      </div>
      <h3 class="engraved">Order</h3>
      <div class="steps">
        {#each r.sequence.steps as b, k (k)}
          <button type="button" class="step mat-raised" class:at={r.sequence.position === k} use:tip={'regist.sequence_step'} aria-label="Step {k + 1}: Registration {b + 1}" onclick={() => removeStep(k)}>{b + 1}</button>
        {:else}
          <span class="note">No steps yet: add buttons below in the order to play them.</span>
        {/each}
      </div>
      <h3 class="engraved">Add</h3>
      <div class="steps">
        {#each r.buttons as b, i (b.index)}
          <button type="button" class="step mat-raised" class:dark={!b.stored} use:tip={'regist.sequence_steps'} aria-label="Add Registration {i + 1}" onclick={() => app.send({ type: 'setRegistSequence', steps: [...r.sequence.steps, i], end: r.sequence.end })}>{i + 1}</button>
        {/each}
        <HwButton tip="regist.sequence_clear" onclick={() => app.send({ type: 'setRegistSequence', steps: [], end: r.sequence.end })}>Clear</HwButton>
      </div>
      <h3 class="engraved">At the end</h3>
      <div class="row">
        {#each SEQUENCE_ENDS as e (e.id)}
          <Toggle tip="regist.sequence_end" on={r.sequence.end === e.id} onclick={() => app.send({ type: 'setRegistSequence', steps: r.sequence.steps, end: e.id })}>{e.name}</Toggle>
        {/each}
      </div>
    </section>
  {:else}
    <section class="block" aria-label="Playlist file">
      <div class="row">
        <select class="field" aria-label="Playlist" use:tip={'playlist.file'} value={pl.path ?? ''} onchange={(e) => e.currentTarget.value && app.send({ type: 'loadPlaylist', path: e.currentTarget.value })}>
          {#if !pl.path}<option value="">{pl.name} (not saved)</option>{/if}
          {#each pl.playlists as p (p.path)}<option value={p.path}>{p.name}</option>{/each}
        </select>
        <HwButton tip="playlist.new" onclick={() => app.send({ type: 'newPlaylist' })}>New</HwButton>
      </div>
      <div class="row">
        <input class="field" type="text" aria-label="Playlist name" placeholder={pl.path ? `Save as… (${pl.name})` : 'Name this playlist'} use:tip={'playlist.name'} bind:value={listName} />
        <HwButton tip="playlist.save" onclick={() => savePlaylist()}>Save{pl.dirty ? ' *' : ''}</HwButton>
        {#if listClash}<HwButton tip="playlist.overwrite" onclick={() => savePlaylist(true)}>Overwrite</HwButton>{/if}
      </div>
      <div class="row">
        <HwButton tip="playlist.add_bank" onclick={() => app.send({ type: 'addCurrentBank' })}>+ This bank</HwButton>
        <HwButton tip="playlist.add_style" onclick={() => app.send({ type: 'addCurrentStyle' })}>+ This style</HwButton>
        <select class="field small" aria-label="Append a playlist" use:tip={'playlist.append'} value="" onchange={(e) => { if (e.currentTarget.value) app.send({ type: 'appendPlaylist', path: e.currentTarget.value }); e.currentTarget.value = '' }}>
          <option value="">+ Append…</option>
          {#each pl.playlists.filter((p) => p.path !== pl.path) as p (p.path)}<option value={p.path}>{p.name}</option>{/each}
        </select>
      </div>
      <div class="row">
        {#each SORTS as o (o.id)}
          <Toggle tip="playlist.sort" on={pl.sort === o.id} onclick={() => app.send({ type: 'setPlaylistSort', sort: o.id })}>{o.name}</Toggle>
        {/each}
        <HwButton tip="playlist.prev" label="Previous song" onclick={() => app.send({ type: 'stepPlaylist', delta: -1 })}>◀</HwButton>
        <HwButton tip="playlist.next" label="Next song" onclick={() => app.send({ type: 'stepPlaylist', delta: 1 })}>▶</HwButton>
      </div>
    </section>

    <ol class="records">
      {#each pl.records as row, k (row.index)}
        <li class="record" class:current={pl.current === row.index}>
          <button type="button" class="load" class:missing={row.missing} use:tip={'playlist.record'} onclick={() => app.send({ type: 'loadPlaylistRecord', index: row.index })}>
            <span class="n">{k + 1}</span>
            <span class="rname">{row.record.name}</span>
            <span class="kind engraved">{row.record.kind === 'bank' ? 'bank' : 'style'}</span>
          </button>
          {#if row.record.kind === 'bank'}
            <select class="field tiny" aria-label="Button to recall" use:tip={'playlist.edit'} value={row.record.regist == null ? '' : String(row.record.regist)} onchange={(e) => setRegist(row.index, row.record, e.currentTarget.value)}>
              <option value="">—</option>
              {#each r.buttons as b (b.index)}<option value={String(b.index)}>{b.index + 1}</option>{/each}
            </select>
          {/if}
          {#if !sorted}
            <HwButton tip="playlist.up" label="Move up" onclick={() => app.send({ type: 'movePlaylistRecord', index: row.index, delta: -1 })}>↑</HwButton>
            <HwButton tip="playlist.down" label="Move down" onclick={() => app.send({ type: 'movePlaylistRecord', index: row.index, delta: 1 })}>↓</HwButton>
            <HwButton tip="playlist.delete" label="Delete record" onclick={() => app.send({ type: 'deletePlaylistRecord', index: row.index })}>✕</HwButton>
          {/if}
        </li>
      {:else}
        <li class="note">No songs yet: load a bank or a style and add it.</li>
      {/each}
    </ol>
  {/if}
</Overlay>

<style>
  .tabs {
    display: flex;
    gap: 0.35rem;
    margin-bottom: 0.8rem;
  }
  .tab {
    flex: 1;
    min-height: 2.2rem;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
  }
  .tab.pressed {
    color: var(--accent);
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }
  h3 {
    margin: 0.4rem 0 0;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.45rem;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.4rem;
  }
  .field {
    flex: 1;
    min-width: 0;
    min-height: 2.2rem;
    padding: 0 0.5rem;
    border: 1px solid var(--seam);
    border-radius: 4px;
    background: var(--well);
    color: var(--ink);
    font: inherit;
  }
  .field.small {
    flex: 0 1 9rem;
  }
  .field.tiny {
    flex: 0 0 3.6rem;
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .buttons,
  .records {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .button,
  .record {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem;
    border-radius: 5px;
  }
  .button.current,
  .record.current {
    background: rgb(255 255 255 / 0.05);
    outline: 1px solid var(--accent);
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .name {
    min-height: 1.8rem;
  }
  .summary {
    padding: 0;
    border: 0;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.8rem;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .actions {
    display: flex;
    gap: 0.3rem;
  }
  .steps {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    align-items: center;
  }
  .step {
    min-width: 2.2rem;
    min-height: 2.2rem;
    border-radius: 4px;
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
  }
  .step.at {
    outline: 2px solid var(--accent);
  }
  .step.dark {
    color: var(--muted);
  }
  .load {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 2.2rem;
    padding: 0 0.4rem;
    border: 0;
    border-radius: 4px;
    background: var(--well);
    color: var(--ink);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .load.missing .rname {
    text-decoration: line-through;
    color: var(--muted);
  }
  .n {
    width: 1.6rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .rname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
