<!--
  Library › Instruments (docs/racks.md, "Plugins coming and going"; the wireframe's
  `browserInst`): a tile per plugin and SoundFont, with its counts.

  ┌ AU ───────────────┐ ┌ AU ─────────────────┐ ┌ ⚠ ──────────────────────┐ ┌ SF2 ─────────┐
  │ Tiny Synth  NEW   │ │ Sampler Deluxe      │ │ String Deluxe ⚠ MISSING │ │ GeneralUser  │
  │ Example · 5 pre…  │ │ Fake · 12 presets   │ │ used in 2 racks         │ │ 260 presets  │
  │ [Browse][+ New]   │ │ [Browse][+ New] […] │ │ [Show racks]            │ │ [Browse]     │
  └───────────────────┘ └─────────────────────┘ └─────────────────────────┘ └──────────────┘

  Browse shows that instrument's sounds in Sounds (every preset of it). + New sound loads a
  blank instance on the target part and opens its window (as the Sound Browser's "New
  sound from…" did). Opening a New plugin either way marks it seen (markPluginSeen).
  Missing tiles say how many racks and sounds use the plugin; Show racks opens Racks with
  "Needs attention" on. More… holds the housekeeping the Sound Browser's Instruments tab
  had: category, in process, Edit….
-->
<script lang="ts">
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import { CATEGORY_LABELS } from '../../lib/api/sound-library'
  import type { PatchCategory, PluginEntry } from '../../lib/api/types'
  import { fontLine, instruments, pluginLoads, scanLine } from '../sounds/instruments'
  import { libraryNav } from './nav.svelte'

  const part = $derived(ui.libraryPart)
  const catalog = $derived(app.sounds)
  const plugins = $derived(app.state.plugins)
  const list = $derived(instruments(catalog, plugins.list))
  const fonts = $derived(list.filter((i) => i.kind === 'font'))
  // New plugins first, then the catalog's order (by maker, then name).
  const pluginTiles = $derived(list.flatMap((i) => (i.kind === 'plugin' ? [i] : [])).sort((a, b) => Number(b.plugin.new) - Number(a.plugin.new)))
  const listing = $derived(new Set(app.state.sounds?.listingPresets ?? []))
  const kp = $derived(app.state.keyboardParts[part])
  let more = $state<string | null>(null)

  const plural = (n: number, one: string) => `${n} ${one}${n === 1 ? '' : 's'}`
  function uses(racks: number, sounds: number): string {
    const out = [racks ? `used in ${plural(racks, 'rack')}` : '', sounds ? `${plural(sounds, 'sound')} of yours` : ''].filter(Boolean)
    return out.join(' · ')
  }

  function seen(p: PluginEntry) {
    if (p.new) app.send({ type: 'markPluginSeen', id: p.id })
  }

  /** Sounds, showing only this instrument (`sf:<file>` or `au:<id>`), every other filter off. */
  function browse(id: string) {
    libraryNav.instrument = id
    libraryNav.source = 'all'
    libraryNav.favourites = false
    libraryNav.category = null
    libraryNav.query = ''
    ui.libraryTab = 'sounds'
  }

  // + New sound: the part loads the plugin's default state, and its editor opens once it
  // plays. Save as… (the footer) keeps it.
  let pendingEditor = $state<string | null>(null)
  function newSound(p: PluginEntry) {
    seen(p)
    app.send({ type: 'setPartPlugin', part, id: p.id, state: null })
    pendingEditor = p.id
  }
  $effect(() => {
    const pl = kp?.plugin
    if (!pendingEditor || !pl || pl.id !== pendingEditor) return
    if (pl.status === 'playing') {
      if (pl.editor) app.pluginEditor(part, true)
      pendingEditor = null
    } else if (pl.status !== 'loading') pendingEditor = null
  })

  function showRacks() {
    libraryNav.attention = true
    ui.libraryTab = 'racks'
  }
</script>

<div class="inst">
  <div class="bar">
    <span class="engraved">{plural(pluginTiles.length, 'plugin')} · {plural(fonts.length, 'SoundFont')}{plugins.missing.length ? ` · ${plugins.missing.length} missing` : ''}</span>
    {#if plugins.available}
      <HwButton tip="part.plugin_rescan" onclick={() => !plugins.scanning && app.send({ type: 'rescanPlugins' })}>{plugins.scanning ? 'Scanning…' : 'Rescan'}</HwButton>
    {/if}
  </div>

  <div class="tiles">
    {#each pluginTiles as inst (inst.id)}
      {@const p = inst.plugin}
      {@const here = pluginLoads(app.state.keyboardParts, p.id).find((l) => l.part === part)}
      <section class="tile" class:fresh={p.new} aria-label={p.name}>
        <div class="ph au"><span>AU</span><span>{p.format}</span></div>
        <div class="tb">
          <b class="tname">{p.name}{#if p.new}<span class="badge new">New</span>{/if}</b>
          <span class="meta">{p.manufacturer} · <span class:warn={!!p.lastError}>{scanLine(p, inst.entry, listing.has(inst.id))}</span></span>
          {#if uses(p.racks, p.sounds)}<span class="meta">{uses(p.racks, p.sounds)}</span>{/if}
          <div class="acts">
            <button type="button" class="act" use:tip={'library.inst_browse'} onclick={() => (seen(p), browse(inst.id))}>Browse</button>
            <button type="button" class="act primary" use:tip={'library.inst_new'} aria-label="New sound from {p.name} on {kp?.name}" onclick={() => newSound(p)}>+ New sound</button>
            <button type="button" class="act" aria-pressed={more === inst.id} use:tip={'library.inst_more'} onclick={() => (more = more === inst.id ? null : inst.id)}>More…</button>
          </div>
          {#if more === inst.id}
            <div class="house">
              <label class="field">
                <span class="k">Category</span>
                <select aria-label="Category of {p.name}" value={inst.entry?.category ?? 'synthLead'} use:tip={'sounds.set_category'} onchange={(ev) => app.send({ type: 'setSoundCategory', id: inst.id, category: ev.currentTarget.value as PatchCategory })}>
                  {#each Object.entries(CATEGORY_LABELS) as [id, label] (id)}<option value={id}>{label}</option>{/each}
                </select>
              </label>
              {#if p.canRunInProcess}
                <label class="field">
                  <input type="checkbox" checked={p.inProcess} use:tip={'part.plugin_in_process'} onchange={() => app.send({ type: 'setPluginInProcess', id: p.id, inProcess: !p.inProcess })} />
                  <span>In process</span>
                </label>
              {/if}
              {#if here?.editor && here.status === 'playing'}<button type="button" class="act" use:tip={'part.plugin_edit'} onclick={() => app.pluginEditor(part, true)}>Edit…</button>{/if}
            </div>
          {/if}
        </div>
      </section>
    {/each}

    {#each plugins.missing as m (m.id)}
      <section class="tile missing" aria-label="{m.name || m.id}, missing">
        <div class="ph"><span>not installed</span></div>
        <div class="tb">
          <b class="tname">{m.name || m.id}<span class="badge missing">⚠ Missing</span></b>
          <span class="meta">{m.manufacturer ? `${m.manufacturer} · ` : ''}not found by the last scan</span>
          <span class="meta">{uses(m.racks, m.sounds) || 'no racks or sounds use it'}</span>
          <div class="acts">
            <button type="button" class="act primary" use:tip={'library.inst_show_racks'} onclick={showRacks}>Show racks</button>
          </div>
        </div>
      </section>
    {/each}

    {#each fonts as inst (inst.id)}
      {#if inst.kind === 'font'}
        <section class="tile" aria-label={inst.name}>
          <div class="ph sf"><span>SF2</span><span>SoundFont</span></div>
          <div class="tb">
            <b class="tname">{inst.name}</b>
            <span class="meta">{fontLine(inst.font)}</span>
            <div class="acts">
              <button type="button" class="act" use:tip={'library.inst_browse'} onclick={() => browse(`sf:${inst.font.file}`)}>Browse</button>
            </div>
          </div>
        </section>
      {/if}
    {/each}
  </div>
  {#if !plugins.available}<p class="note">Plugins need the built-in synth.</p>{/if}
  <p class="note">+ New sound loads a blank instance on {kp?.name} and opens its window. New plugins show up after the startup scan or Rescan.</p>
</div>

<style>
  .inst {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    height: 100%;
    min-height: 0;
    overflow: auto;
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
    gap: 0.8rem;
    align-content: start;
  }
  .tile {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: rgb(0 0 0 / 0.12);
    overflow: hidden;
  }
  .tile.fresh {
    border-color: color-mix(in srgb, var(--accent) 70%, transparent);
  }
  .tile.missing {
    border-style: dashed;
    border-color: color-mix(in srgb, var(--danger) 60%, transparent);
  }
  .ph {
    display: flex;
    justify-content: space-between;
    padding: 0.35rem 0.6rem;
    background: var(--screen-bg);
    color: var(--screen-dim);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .ph.au {
    color: var(--accent);
  }
  .tb {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.55rem 0.6rem 0.65rem;
    min-width: 0;
  }
  .tname {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-family: var(--font-display);
    font-size: 1.05rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta,
  .note,
  .k {
    color: var(--muted);
    font-size: var(--fs-small);
  }
  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note {
    margin: 0;
  }
  .warn {
    color: var(--danger);
  }
  .badge {
    padding: 0 4px;
    border: 2px solid var(--accent);
    border-radius: 3px;
    color: var(--accent);
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .badge.missing {
    border-color: var(--danger);
    color: var(--danger);
  }
  .acts,
  .house {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.3rem;
  }
  button.act {
    min-height: 2rem;
    border: 1px solid var(--line-strong);
    border-radius: 4px;
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    padding: 0 0.65rem;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
  }
  button.act.primary {
    border-color: color-mix(in srgb, var(--accent) 70%, transparent);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  button.act[aria-pressed='true'] {
    border-color: var(--accent);
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: var(--fs-small);
  }
  .field select {
    min-height: 2rem;
    padding: 0 0.4rem;
    border: 1px solid var(--well-edge);
    border-radius: 4px;
    background: var(--screen-bg);
    color: var(--screen-ink);
  }
</style>
