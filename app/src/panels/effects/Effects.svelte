<!--
  The Effects screen (docs/design/redesign-b-handoff.md, "Effects"): the shared effect bus
  every part feeds through its sends, one card per block, and the style's insertion effects.

  - A card per block, in the state's order (Reverb, Chorus, Variation, drawn as "Delay"):
    - From style / Mine (#237, `setFollowStyle`): From style takes the style's own type at
      each style change (its XG name shows on the chip); Mine keeps the player's. Choosing a
      type or turning a parameter here (or on a Launchkey effect knob page) makes it Mine.
    - The type chips (#204, `setEffectType`), from the block's own `types` in the state, so
      a type the engine adds shows here with no change. A type change puts the parameters
      back to that type's own values.
    - Knobs: the return level (64 = 0 dB, `setEffectReturn`), then the block's parameters
      (#236, `setEffectParam`) in their own units. The delay shows its Note with tempo sync
      on and its free Time with it off, as the Launchkey's FX page knob 4 does. A 0–1
      parameter (Tempo sync, Ping-pong) is a switch under them.
    - Band (#236, `setBandSend`) and Pads (#267, `setPadSend`): every Style part's, and every
      Multi Pad's, send to this block scaled, in percent.
    - Rack (`setRackSendOverride`, send 1-3 = Reverb, Chorus, Delay): "Rack keeps type" lit,
      the live rack keeps this send's type over the style's; while it does, a "Set by rack"
      badge and a "Use style's" button (override off) show.
  - Added sends 4-6 (`effects.sends[3..]`), a card each: the kind picker (`setSendKind`; a
    kind this build doesn't list still shows by its name), the return (`setSendReturn`), a
    knob per parameter by the state's own name, range and reading (`setSendParam`,
    double-click for its default), and Remove (`removeSend`).
  - Add send, while there are fewer than six: a kind picker (Hall to start) and Add
    (`addSend`).
  - Inserts (#269): the style's insertion effects, each on one Style part, all on or off
    (`setInsertsOn`); each part's on/off (`setPartInsertOn`) and amount
    (`setPartInsertAmount`); the rotary's fast/slow switch (`setRotaryFast`).
  The Mixer keeps each strip's own send knobs and EQ, and opens this screen.
-->
<script lang="ts">
  import type { AppCmd, EffectBlockState, FxBlock, FxParam, FxParamState, InsertEffect, SendKind, SendState } from '../../lib/api/types'
  import type { TipKey } from '../../help/tooltips'
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import HSlider from '../settings/HSlider.svelte'
  import FxKnob from '../mixer/FxKnob.svelte'
  import { MAX_SENDS, returnText, SEND_KINDS, STYLE_SENDS } from './sendKinds'

  const fx = $derived(app.state.effects)

  const TIPS: Record<FxBlock, { type: TipKey; ret: TipKey; band: TipKey; pad: TipKey }> = {
    reverb: { type: 'fx.reverb_type', ret: 'fx.reverb_return', band: 'fx.reverb_band', pad: 'fx.reverb_pad' },
    chorus: { type: 'fx.chorus_type', ret: 'fx.chorus_return', band: 'fx.chorus_band', pad: 'fx.chorus_pad' },
    variation: { type: 'fx.variation_type', ret: 'fx.variation_return', band: 'fx.variation_band', pad: 'fx.variation_pad' },
  }
  const PARAM_TIPS: Record<FxParam, TipKey> = {
    reverbTime: 'fx.param.reverb_time',
    preDelay: 'fx.param.pre_delay',
    reverbTone: 'fx.param.reverb_tone',
    delaySync: 'fx.param.delay_sync',
    delayNote: 'fx.param.delay_note',
    delayTime: 'fx.param.delay_time',
    delayFeedback: 'fx.param.delay_feedback',
    delayTone: 'fx.param.delay_tone',
    pingPong: 'fx.param.ping_pong',
    chorusRate: 'fx.param.chorus_rate',
    chorusDepth: 'fx.param.chorus_depth',
  }
  const INSERT_NAMES: Record<InsertEffect, string> = { distortion: 'Distortion', compressor: 'Compressor', autoWah: 'Auto Wah', tremolo: 'Tremolo', rotary: 'Rotary', phaser: 'Phaser' }

  /** The Variation block is the tempo delay: named for what it plays. */
  const title = (b: EffectBlockState) => (b.block === 'variation' ? 'Delay' : b.name)
  const isSwitch = (p: FxParamState) => p.max - p.min === 1
  /** The delay plays its Note with tempo sync on and its free Time with it off: show that one. */
  function shown(b: EffectBlockState, p: FxParamState): boolean {
    const sync = b.params.find((x) => x.param === 'delaySync')
    if (!sync) return true
    return !((p.param === 'delayTime' && sync.value === 1) || (p.param === 'delayNote' && sync.value === 0))
  }
  const knobs = (b: EffectBlockState) => b.params.filter((p) => !isSwitch(p) && shown(b, p))
  const switches = (b: EffectBlockState) => b.params.filter(isSwitch)

  const percent = (v: number) => `${v}%`

  const send = (cmd: AppCmd) => app.send(cmd)

  /** Send 1-3 is the Reverb, Chorus, Delay block. */
  const BLOCK_SEND: Record<FxBlock, number> = { reverb: 0, chorus: 1, variation: 2 }
  const sends = $derived(fx.sends ?? [])
  const blockSend = (b: EffectBlockState): SendState | undefined => sends.find((s) => s.send === BLOCK_SEND[b.block])
  const added = $derived(sends.filter((s) => s.send >= STYLE_SENDS))
  /** The kinds for an added send's picker, with its own kind first if this build doesn't list it. */
  const kindsFor = (s: SendState) => (SEND_KINDS.some((k) => k.kind === s.kind) ? SEND_KINDS : [{ kind: s.kind, name: s.name }, ...SEND_KINDS])
  let newKind = $state<SendKind>('hall')
</script>

<Overlay id="effects" title="Effects" closeTip="drawer.close" onclose={() => (ui.effects = false)}>
  <div class="screen-body">
    <div class="cards">
      {#each fx.blocks as b (b.block)}
        <section class="card" aria-label={title(b)} data-block={b.block}>
          <header>
            <h3>{title(b)}</h3>
            <div class="source" role="group" aria-label="{title(b)} type from">
              <button
                type="button"
                class="chip mat-raised"
                class:pressed={b.followStyle}
                aria-pressed={b.followStyle}
                use:tip={'fx.follow_style'}
                onclick={() => send({ type: 'setFollowStyle', block: b.block, on: true })}
              >
                <span class="lamp" class:on={b.followStyle} aria-hidden="true"></span>From style
              </button>
              <button
                type="button"
                class="chip mat-raised"
                class:pressed={!b.followStyle}
                aria-pressed={!b.followStyle}
                use:tip={'fx.mine'}
                onclick={() => send({ type: 'setFollowStyle', block: b.block, on: false })}
              >
                <span class="lamp" class:on={!b.followStyle} aria-hidden="true"></span>Mine
              </button>
            </div>
          </header>

          <div class="readout mat-screen" aria-live="polite">
            <span class="name glow-text">{b.effectName}</span>
            <span class="style-name" title={b.styleEffect?.name ?? ''}>
              Style: {b.styleEffect ? b.styleEffect.name + (b.styleEffect.effect ? '' : ' (no match)') : 'sets none'}
            </span>
          </div>

          {#if blockSend(b)}
            {@const st = blockSend(b)!}
            <div class="rack" role="group" aria-label="{title(b)} rack">
              <Toggle on={st.setByRack} tip="fx.send_rack_override" onclick={() => send({ type: 'setRackSendOverride', send: st.send, on: !st.setByRack })}
                >Rack keeps type</Toggle
              >
              {#if st.setByRack}
                <span class="badge">Set by rack: {st.name}</span>
                <button
                  type="button"
                  class="chip mat-raised"
                  use:tip={'fx.send_use_style'}
                  onclick={() => send({ type: 'setRackSendOverride', send: st.send, on: false })}>Use style's</button
                >
              {/if}
            </div>
          {/if}

          <div class="types" role="group" aria-label="{title(b)} type">
            {#each b.types as t (t.effect)}
              <button
                type="button"
                class="chip mat-raised"
                class:pressed={b.effect === t.effect}
                aria-pressed={b.effect === t.effect}
                use:tip={TIPS[b.block].type}
                onclick={() => send({ type: 'setEffectType', block: b.block, effect: t.effect })}>{t.name}</button
              >
            {/each}
          </div>

          <div class="knobs">
            <div class="ret">
              <FxKnob
                value={b.returnLevel}
                tip={TIPS[b.block].ret}
                label="{title(b)} return"
                caption="Return"
                reset={64}
                format={returnText}
                onchange={(v) => send({ type: 'setEffectReturn', block: b.block, level: v })}
              />
            </div>
            {#each knobs(b) as p (p.param)}
              <FxKnob
                value={p.value}
                min={p.min}
                max={p.max}
                tip={PARAM_TIPS[p.param]}
                label="{title(b)} {p.name}"
                caption={p.name}
                reset={p.default}
                format={() => p.display}
                onchange={(v) => send({ type: 'setEffectParam', block: b.block, param: p.param, value: v })}
              />
            {/each}
          </div>

          {#if switches(b).length > 0}
            <div class="switches">
              {#each switches(b) as p (p.param)}
                <Toggle
                  on={p.value === 1}
                  tip={PARAM_TIPS[p.param]}
                  onclick={() => send({ type: 'setEffectParam', block: b.block, param: p.param, value: p.value === 1 ? 0 : 1 })}
                  >{p.name}</Toggle
                >
              {/each}
            </div>
          {/if}

          <div class="sends">
            <span class="engraved">Band</span>
            <HSlider
              value={b.bandSend}
              tip={TIPS[b.block].band}
              label="{title(b)} band send"
              unity={100}
              format={percent}
              onchange={(v) => send({ type: 'setBandSend', block: b.block, level: v })}
            />
            <span class="engraved">Pads</span>
            <HSlider
              value={b.padSend}
              tip={TIPS[b.block].pad}
              label="{title(b)} Multi Pad send"
              unity={100}
              format={percent}
              onchange={(v) => send({ type: 'setPadSend', block: b.block, level: v })}
            />
          </div>
        </section>
      {/each}

      {#each added as s (s.send)}
        {@const label = `Send ${s.send + 1}`}
        <section class="card" aria-label={label} data-send={s.send}>
          <header>
            <h3>{label}</h3>
            <button
              type="button"
              class="chip mat-raised"
              aria-label="Remove {label}"
              use:tip={'fx.send_remove'}
              onclick={() => send({ type: 'removeSend', send: s.send })}>Remove</button
            >
          </header>

          <div class="readout mat-screen" aria-live="polite">
            <span class="name glow-text">{s.name}</span>
            <span class="style-name">Saved with the rack</span>
          </div>

          <select
            class="kind"
            aria-label="{label} type"
            value={s.kind}
            use:tip={'fx.send_kind'}
            onchange={(e) => send({ type: 'setSendKind', send: s.send, kind: e.currentTarget.value })}
          >
            {#each kindsFor(s) as k (k.kind)}
              <option value={k.kind}>{k.name}</option>
            {/each}
          </select>

          <div class="knobs">
            <div class="ret">
              <FxKnob
                value={s.returnLevel}
                tip="fx.send_return"
                label="{label} return"
                caption="Return"
                reset={64}
                format={returnText}
                onchange={(v) => send({ type: 'setSendReturn', send: s.send, level: v })}
              />
            </div>
            {#each s.params as p, i (i)}
              <FxKnob
                value={p.value}
                min={p.min}
                max={p.max}
                tip="fx.send_param"
                label="{label} {p.name}"
                caption={p.name}
                reset={p.default}
                format={() => p.display}
                onchange={(v) => send({ type: 'setSendParam', send: s.send, param: i, value: v })}
              />
            {/each}
          </div>
        </section>
      {/each}

      {#if sends.length > 0 && sends.length < MAX_SENDS}
        <div class="card add-send" role="group" aria-label="Add send">
          <h3>Add send</h3>
          <select class="kind" aria-label="New send type" bind:value={newKind} use:tip={'fx.send_add_kind'}>
            {#each SEND_KINDS as k (k.kind)}
              <option value={k.kind}>{k.name}</option>
            {/each}
          </select>
          <button type="button" class="chip mat-raised add" use:tip={'fx.send_add'} onclick={() => send({ type: 'addSend', kind: newKind })}>Add send</button>
        </div>
      {/if}
    </div>

    <section class="card inserts" aria-label="Style inserts">
      <header>
        <h3>Inserts</h3>
        <div class="source">
          <Toggle on={fx.insertsOn} tip="fx.inserts" onclick={() => send({ type: 'setInsertsOn', on: !fx.insertsOn })}>Inserts on</Toggle>
          <Toggle on={fx.rotaryFast} tip="fx.rotary_fast" onclick={() => send({ type: 'setRotaryFast', on: !fx.rotaryFast })}>Rotary fast</Toggle>
        </div>
      </header>
      {#if fx.inserts.length === 0}
        <p class="none">The style has no insertion effects.</p>
      {:else}
        <div class="insert-list">
          {#each fx.inserts as i (i.part)}
            <div class="insert" class:dry={!i.effect || !i.on || !fx.insertsOn}>
              <Toggle on={i.on} tip="fx.insert_part" onclick={() => send({ type: 'setPartInsertOn', part: i.part, on: !i.on })}>{i.partName}</Toggle>
              <div class="what" title={i.name}>
                <span class="xg">{i.name}</span>
                <span class="plays">→ {i.effect ? INSERT_NAMES[i.effect] : 'dry'}</span>
              </div>
              {#if i.effect}
                <FxKnob
                  value={i.amount}
                  tip="fx.insert_amount"
                  label="{i.partName} insert amount"
                  caption="Amount"
                  reset={64}
                  onchange={(v) => send({ type: 'setPartInsertAmount', part: i.part, amount: v })}
                />
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</Overlay>

<style>
  /* Wider than the other drawers: three effect cards side by side when there's room. */
  :global(.overlay.right[data-overlay='effects']) {
    width: min(62rem, calc(100vw - 32px));
  }
  .screen-body {
    display: grid;
    gap: 0.8rem;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(17rem, 1fr));
    gap: 0.8rem;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    min-width: 0;
    padding: 0.75rem;
    border: 1px solid var(--seam);
    border-radius: var(--r-key);
    background: rgb(0 0 0 / 0.12);
    box-shadow: inset 0 1px 3px rgb(0 0 0 / 0.35);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.4rem 0.8rem;
  }
  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.6rem;
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0.02em;
    color: var(--ink);
  }
  .source,
  .types,
  .switches {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.45em;
    min-height: 2rem;
    padding: 0 0.75em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.92rem;
    letter-spacing: 0.02em;
    color: var(--ink);
    white-space: nowrap;
  }
  .chip.pressed {
    outline: 1px solid var(--accent);
  }
  .chip:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .lamp {
    width: 0.5em;
    height: 0.5em;
    border-radius: 50%;
    background: var(--lamp-off);
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.6);
  }
  .lamp.on {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .readout {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.6rem;
    padding: 0.45rem 0.7rem;
    border-radius: 6px;
    font-family: var(--font-display);
  }
  .readout .name {
    font-size: 1.25rem;
    font-weight: 600;
    white-space: nowrap;
  }
  .style-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8rem;
    opacity: 0.75;
  }
  .rack {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem 0.5rem;
  }
  .badge {
    padding: 0.15em 0.55em;
    border: 1px solid var(--accent);
    border-radius: 999px;
    font-family: var(--font-display);
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--accent);
    white-space: nowrap;
  }
  .kind {
    min-width: 0;
    min-height: 2rem;
    font: inherit;
    font-size: 0.92rem;
  }
  .add-send {
    justify-content: center;
  }
  .add-send h3 {
    font-size: 1.2rem;
  }
  .add {
    justify-content: center;
  }
  .knobs {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-around;
    gap: 0.5rem 0.2rem;
  }
  /* Four knobs to a row in a 17rem card: the reverb's and the delay's all fit. */
  .knobs > :global(*),
  .ret {
    flex: 0 0 3.5rem;
  }
  /* Bigger than a mixer strip's knobs: this screen has the room. */
  .knobs :global(.knob svg),
  .insert :global(.knob svg) {
    width: 2.7rem;
  }
  .knobs :global(.knob .caption),
  .insert :global(.knob .caption) {
    font-size: 0.72rem;
  }
  .knobs :global(.knob .value),
  .insert :global(.knob .value) {
    font-size: 0.85rem;
  }
  /* The return is the block's level into the mix: lit in the accent like a fader cap. */
  .ret :global(.knob .caption) {
    color: var(--accent);
  }
  .sends {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 0.4rem 0.6rem;
    margin-top: auto;
  }
  .inserts .none {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .insert-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
    gap: 0.6rem 1rem;
  }
  .insert {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
  }
  .insert.dry {
    opacity: 0.6;
  }
  .what {
    display: grid;
    flex: 1;
    min-width: 0;
    font-size: 0.85rem;
    line-height: 1.25;
  }
  .xg {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink);
  }
  .plays {
    color: var(--muted);
  }
  .insert > :global(.knob) {
    flex: 0 0 4.2rem;
  }
</style>
