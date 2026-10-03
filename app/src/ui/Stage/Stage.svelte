<!--
  Stage: the Stage page at 1440 × 900. A thin layout of components: the app bar, the section row,
  the display, the hardware band (faders; knobs above pads; transport), the status line in the gap
  above the keys, and the keys. Each region takes its data as one object; callbacks pass through.
-->
<script lang="ts">
  import type { Component, ComponentProps } from 'svelte'
  import type { Action } from 'svelte/action'
  import AppBar from '../AppBar/AppBar.svelte'
  import Display from '../Display/Display.svelte'
  import FaderBank from '../FaderBank/FaderBank.svelte'
  import Keys from '../Keys/Keys.svelte'
  import KnobBank from '../KnobBank/KnobBank.svelte'
  import PadBank from '../PadBank/PadBank.svelte'
  import SectionRow from '../SectionRow/SectionRow.svelte'
  import StatusLine from '../StatusLine/StatusLine.svelte'
  import TransportColumn from '../TransportColumn/TransportColumn.svelte'

  type Callbacks = 'tipAction' | `on${string}`
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  type Any = Component<any>
  type Data<C extends Any> = Omit<ComponentProps<C>, Callbacks>
  type On<C extends Any, K extends keyof ComponentProps<C>> = Pick<ComponentProps<C>, K>

  type Props = {
    /** The app bar: pages, Launchkey, audio health. */
    appBar: Data<typeof AppBar>
    /** The section row: Accomp, the count, the helpers. */
    sectionRow: Data<typeof SectionRow>
    /** The display: style line, now playing, sound row. */
    display: Data<typeof Display>
    /** The band's faders: strips, page and layer tabs, lamps. */
    faders: Data<typeof FaderBank>
    /** The band's knobs and their page. */
    knobs: Data<typeof KnobBank>
    /** The band's pads and their bank. */
    pads: Data<typeof PadBank>
    /** The band's transport column. */
    transport: Data<typeof TransportColumn>
    /** The status line above the keys (empty when `text` is null). */
    status: Data<typeof StatusLine>
    /** The keys: range, split, held notes. */
    keys: ComponentProps<typeof Keys>
    /** The app's tooltip action (`use:tip`), passed to every region. */
    tipAction?: Action<HTMLElement, string>
    /** A knob was clicked (KnobBank `onpress`). */
    onknobpress?: ComponentProps<typeof KnobBank>['onpress']
    /** A pad was pressed (PadBank `onpress`). */
    onpadpress?: ComponentProps<typeof PadBank>['onpress']
  } & On<typeof AppBar, 'onchoose' | 'onhealth'> &
    On<typeof SectionRow, 'onaccomp' | 'onmetronome' | 'onmetronomesettings' | 'onunison' | 'onpanic' | 'onhelp'> &
    On<typeof Display, 'onprev' | 'onnext' | 'onbrowse' | 'ononetouch' | 'onsends' | 'onrack' | 'onpart' | 'onsound'> &
    On<
      typeof FaderBank,
      | 'onchoosePage'
      | 'onchooseLayer'
      | 'onlevel'
      | 'onopen'
      | 'onlamp'
      | 'onlamplong'
      | 'onlamprelease'
      | 'onpagebutton'
    > &
    On<typeof KnobBank, 'onpageup' | 'onpagedown' | 'onstep'> &
    On<typeof PadBank, 'onbankup' | 'onbankdown'> &
    On<
      typeof TransportColumn,
      | 'onstartstop'
      | 'onstop'
      | 'onstoplong'
      | 'onreset'
      | 'onfade'
      | 'onfillup'
      | 'onfilldown'
      | 'ontempoup'
      | 'ontempodown'
      | 'onstyletempo'
    > &
    On<typeof StatusLine, 'onclear'>

  let p: Props = $props()
</script>

<div class="screen">
  <AppBar {...p.appBar} tipAction={p.tipAction} onchoose={p.onchoose} onhealth={p.onhealth} />
  <div class="row">
    <SectionRow
      {...p.sectionRow}
      tipAction={p.tipAction}
      onaccomp={p.onaccomp}
      onmetronome={p.onmetronome}
      onmetronomesettings={p.onmetronomesettings}
      onunison={p.onunison}
      onpanic={p.onpanic}
      onhelp={p.onhelp}
    />
  </div>
  <div class="display">
    <Display
      {...p.display}
      tipAction={p.tipAction}
      onprev={p.onprev}
      onnext={p.onnext}
      onbrowse={p.onbrowse}
      ononetouch={p.ononetouch}
      onsends={p.onsends}
      onrack={p.onrack}
      onpart={p.onpart}
      onsound={p.onsound}
    />
  </div>
  <div class="band">
    <FaderBank
      {...p.faders}
      tipAction={p.tipAction}
      onchoosePage={p.onchoosePage}
      onchooseLayer={p.onchooseLayer}
      onlevel={p.onlevel}
      onopen={p.onopen}
      onlamp={p.onlamp}
      onlamplong={p.onlamplong}
      onlamprelease={p.onlamprelease}
      onpagebutton={p.onpagebutton}
    />
    <div class="middle">
      <KnobBank
        {...p.knobs}
        tipAction={p.tipAction}
        onpageup={p.onpageup}
        onpagedown={p.onpagedown}
        onpress={p.onknobpress}
        onstep={p.onstep}
      />
      <div class="pads">
        <PadBank
          {...p.pads}
          tipAction={p.tipAction}
          onbankup={p.onbankup}
          onbankdown={p.onbankdown}
          onpress={p.onpadpress}
        />
      </div>
    </div>
    <TransportColumn
      {...p.transport}
      tipAction={p.tipAction}
      onstartstop={p.onstartstop}
      onstop={p.onstop}
      onstoplong={p.onstoplong}
      onreset={p.onreset}
      onfade={p.onfade}
      onfillup={p.onfillup}
      onfilldown={p.onfilldown}
      ontempoup={p.ontempoup}
      ontempodown={p.ontempodown}
      onstyletempo={p.onstyletempo}
    />
  </div>
  <StatusLine {...p.status} tipAction={p.tipAction} onclear={p.onclear} />
  <Keys {...p.keys} />
</div>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    width: var(--screen-width);
    height: var(--screen-height);
    padding: var(--screen-pad);
    overflow: hidden;
    background: var(--g);
    color: var(--t);
    font-family: var(--font-sans);
  }
  .row,
  .display,
  .band {
    display: flex;
    flex: none;
  }
  .row {
    margin-top: var(--stage-gap-row);
  }
  .display {
    margin-top: var(--stage-gap-display);
  }
  .band {
    gap: var(--band-gap);
    height: var(--band-height);
    margin-top: var(--stage-gap-band);
  }
  .middle {
    display: flex;
    flex: none;
    flex-direction: column;
  }
  .pads {
    display: flex;
    margin-top: var(--band-pads-gap);
  }
</style>
