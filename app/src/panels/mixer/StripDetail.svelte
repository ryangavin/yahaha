<!--
  A mixer strip's details, above its compact strip in the row (shown with `ui.mixer`):
  what the old drawer's strip had that the compact one doesn't.

  - The MIDI channel at yahaha's output.
  - The Chorus send (CC 93). A keyboard part's is 0 until set (double-click: 0); a Style
    part's shows the style's own until turned, then the part's own (marked •, #268), and a
    double-click hands the part's sends back to the style.
  - A keyboard part's channel-strip EQ (#247, `setPartEq`): Low and High shelf gain
    (−12..+12 dB) and frequency, in the XG EQ frequency steps; then its insert slot
    (effect, on, amount).
  - The badge: Plays bass (Manual Bass), the plugin's state, or a Style Bass muted by
    Manual Bass.
  - The track's CPU (#340): its share of the buffer over the last second, and the worst
    buffer (pk).
  Every strip's detail column has the same rows, reserved when empty, so the compact
  strips below stay aligned.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { FLAT_EQ, INSERT_EFFECTS, OFF_INSERT, type InsertEffect, type Meters, type PartEq } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import FxKnob from './FxKnob.svelte'
  import { cpuOf, cpuWarn, pct } from './cpu'
  import { dbText, GAIN_KNOB_MAX, gainKnob, HIGH_STEPS, hzText, knobGain, LOW_STEPS, stepOf, withEq } from './eq'
  import { pluginBadge, pluginTip } from './voice'

  let { part, meters }: { part: number; meters: Meters | null } = $props()

  const kbd = $derived(part < 4 ? app.state.keyboardParts[part] : null)
  const sty = $derived(part >= 4 ? app.state.mixer.styleParts[part - 4] : null)
  const name = $derived(kbd?.name ?? sty?.name ?? '')
  const channel = $derived(kbd?.channel ?? sty?.channel ?? null)
  const cpu = $derived(channel === null ? null : cpuOf(meters, [channel]))

  const badge = $derived.by((): { text: string; tip: TipKey } | null => {
    if (kbd) {
      if (kbd.playsBass) return { text: 'Plays bass', tip: 'detection.manual_bass' }
      if (kbd.plugin) return { text: pluginBadge(kbd.plugin), tip: pluginTip(kbd.plugin) }
      return null
    }
    return sty?.mutedByManualBass ? { text: 'Manual Bass', tip: 'detection.manual_bass' } : null
  })

  const eq = $derived(kbd?.eq ?? FLAT_EQ)
  const ins = $derived(kbd?.insert ?? OFF_INSERT)
  const setEq = (change: Partial<PartEq>) => app.send({ type: 'setPartEq', part, eq: withEq(eq, change) })
  const setChorus = (v: number) =>
    kbd ? app.send({ type: 'setPartSend', part, send: 'chorus', value: v }) : app.send({ type: 'setStylePartSend', part: part - 4, send: 'chorus', value: v })
</script>

<div class="detail" class:style={sty !== null} role="group" aria-label="{name} details">
  <div class="ch" use:tip={'mixer.channel'}>
    {#if channel !== null}<span class="engraved">Ch</span> <b>{channel}</b>{:else}&nbsp;{/if}
  </div>

  <!-- The Chorus send, and a keyboard part's insert amount beside it (one knob row). -->
  <div class="knobs">
    {#if kbd}
      <FxKnob value={kbd.chorus} tip="mixer.part.chorus" label="{name} chorus" caption="Cho" reset={0} onchange={setChorus} />
    {:else if sty}
      <FxKnob
        value={sty.chorus}
        tip="mixer.style.chorus"
        label="{name} chorus"
        caption="Cho"
        reset={10}
        onchange={setChorus}
        onreset={() => app.send({ type: 'resetStylePartSends', part: part - 4 })}
        own={sty.sendsSet.includes('chorus')}
      />
    {/if}
    {#if kbd}
      <span class="amt" class:off={!ins.on}>
        <FxKnob value={ins.amount} tip="mixer.part.insert_amount" label="{name} insert amount" caption="Amt" reset={64} onchange={(v) => app.send({ type: 'setKeyboardInsertAmount', part, amount: v })} />
      </span>
    {/if}
  </div>

  <div class="eq" class:kbd={kbd !== null} role={kbd ? 'group' : undefined} aria-label={kbd ? `${name} EQ` : undefined}>
    {#if kbd}
      <FxKnob value={gainKnob(eq.lowGain)} max={GAIN_KNOB_MAX} centre tip="mixer.part.eq_low_gain" label="{name} EQ low" caption="Low" reset={gainKnob(0)} format={(v) => dbText(knobGain(v))} onchange={(v) => setEq({ lowGain: knobGain(v) })} />
      <FxKnob value={stepOf(LOW_STEPS, eq.lowFreq)} max={LOW_STEPS.length - 1} tip="mixer.part.eq_low_freq" label="{name} EQ low frequency" caption="L Hz" reset={stepOf(LOW_STEPS, FLAT_EQ.lowFreq)} format={(v) => hzText(LOW_STEPS[v])} onchange={(v) => setEq({ lowFreq: LOW_STEPS[v] })} />
      <FxKnob value={gainKnob(eq.highGain)} max={GAIN_KNOB_MAX} centre tip="mixer.part.eq_high_gain" label="{name} EQ high" caption="High" reset={gainKnob(0)} format={(v) => dbText(knobGain(v))} onchange={(v) => setEq({ highGain: knobGain(v) })} />
      <FxKnob value={stepOf(HIGH_STEPS, eq.highFreq)} max={HIGH_STEPS.length - 1} tip="mixer.part.eq_high_freq" label="{name} EQ high frequency" caption="H Hz" reset={stepOf(HIGH_STEPS, FLAT_EQ.highFreq)} format={(v) => hzText(HIGH_STEPS[v])} onchange={(v) => setEq({ highFreq: HIGH_STEPS[v] })} />
    {/if}
  </div>

  <div class="ins" class:kbd={kbd !== null} class:off={!ins.on} role={kbd ? 'group' : undefined} aria-label={kbd ? `${name} insert` : undefined}>
    {#if kbd}
      <button
        type="button"
        class="ins-on mat-raised"
        class:on={ins.on}
        aria-pressed={ins.on}
        aria-label="{name} insert on"
        use:tip={'mixer.part.insert_on'}
        onclick={() => app.send({ type: 'setKeyboardInsertOn', part, on: !ins.on })}>Ins</button
      >
      <select
        class="ins-kind"
        aria-label="{name} insert effect"
        value={ins.effect}
        use:tip={'mixer.part.insert_effect'}
        onchange={(e) => app.send({ type: 'setKeyboardInsertEffect', part, effect: e.currentTarget.value as InsertEffect })}
      >
        {#each INSERT_EFFECTS as o (o.effect)}<option value={o.effect}>{o.name}</option>{/each}
      </select>
    {/if}
  </div>

  <div class="badge">
    {#if badge}<span class="tag" use:tip={badge.tip}>{badge.text}</span>{/if}
  </div>

  <div class="cpu" class:warn={cpuWarn(cpu)} data-testid="cpu">
    {#if cpu !== null}
      <span class="cpu-text" use:tip={'mixer.cpu'} aria-label="{name} CPU {pct(cpu.avg)}, peak {pct(cpu.peak)}">{pct(cpu.avg)} <span class="pk">pk {pct(cpu.peak)}</span></span>
      <span class="bar" aria-hidden="true"><span class="fill" style:width="{Math.min(1, cpu.avg) * 100}%"></span><span class="mark" style:left="{Math.min(1, cpu.peak) * 100}%"></span></span>
    {/if}
  </div>
</div>

<style>
  /* Fixed row heights, the same on every strip (a Style part's EQ and insert rows stay
     empty), so the compact strips below line up. */
  .detail {
    display: grid;
    /* Channel · Chorus (+ insert amount) · EQ · insert · badge · CPU: short enough that the
       details and a whole compact strip fit a 900 px tall window. */
    grid-template-rows: 1.1rem 2.7rem 5.4rem 1.5rem 1.1rem 1.2rem;
    justify-items: center;
    align-items: start;
    gap: 0.1rem;
    min-width: 0;
    width: 100%;
    padding: 0.1rem 0.15rem;
    font-size: 0.72rem;
    color: var(--ink);
  }
  .ch {
    font-family: var(--font-display);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .ch b {
    font-weight: 700;
  }
  .knobs {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 2.6rem);
    justify-content: center;
    gap: 0.1rem;
    width: 100%;
  }
  .amt.off :global(.knob) {
    opacity: 0.6;
  }
  /* Four knobs, two by two, to fit a narrow strip. */
  .eq {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.1rem;
    width: 100%;
  }
  .eq.kbd,
  .ins.kbd {
    padding-top: 0.15rem;
    border-top: 1px solid var(--seam);
  }
  /* The insert: on, then its effect, on one line. */
  .ins {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 0.15rem;
    width: 100%;
  }
  .ins.off .ins-kind {
    opacity: 0.6;
  }
  .ins-kind {
    min-width: 0;
    width: 100%;
    min-height: 1.4rem;
    font: inherit;
    font-size: 0.68rem;
    color: var(--ink);
    background: var(--raised);
    border: 1px solid var(--line);
    border-radius: 4px;
  }
  .ins-on {
    height: 1.4rem;
    padding: 0 0.3em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.72rem;
    color: var(--muted);
  }
  .ins-on.on {
    color: var(--accent-ink);
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }
  .badge {
    max-width: 100%;
    overflow: hidden;
  }
  .tag {
    display: inline-block;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-display);
    font-size: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    padding: 0 0.3em;
    border-radius: 3px;
    color: var(--accent-ink);
    background: var(--accent);
  }
  /* #340: the track's CPU, a line and a thin bar (the tick is the worst buffer). */
  .cpu {
    display: grid;
    gap: 0.15rem;
    width: 100%;
    justify-items: center;
    font-family: var(--font-display);
    font-size: 0.66rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .cpu-text {
    white-space: nowrap;
  }
  .pk {
    opacity: 0.8;
  }
  .cpu.warn .pk {
    color: var(--danger);
    opacity: 1;
  }
  .bar {
    position: relative;
    width: 80%;
    height: 3px;
    border-radius: 2px;
    background: var(--lamp-off);
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 2px;
    background: var(--accent);
  }
  .cpu.warn .fill {
    background: var(--danger);
  }
  .mark {
    position: absolute;
    top: -1px;
    width: 1px;
    height: 5px;
    background: var(--ink);
    opacity: 0.6;
  }
</style>
