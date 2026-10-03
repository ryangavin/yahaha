# ChoiceRow

## Identity (all stations)

- **Kind:** primitive (a labelled row around one ChosenTabs group)
- **Built from:** ChosenTabs, with its `row` and `row-sm` sizes (D2: the sizes and the group kind are an addition to ChosenTabs that must land first)
- **Purpose:** Shows one setting whose values are a few words side by side (Assign: Auto, Multi, R1, R2, R3; Speed: 1/4 … 1/32), with its name at the left, and picks another with one click.
- **Boards:** `Harmony-Dark.dc.html:166-175` (Assign, Auto chosen); light: `Harmony-Light.dc.html:156-165`. Speed (`row-sm`) is specified in Harmony.md (HA-D9) but not drawn on a board.
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what the choices mean: the parent passes `tabs`, `chosen`, `drawn` and acts on `onchoose`. No hairline list around it (the column is the parent's); no unlabelled choice (a bare segmented choice, such as Effects' Source and Type, is ChosenTabs `row` on its own, D1).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The visible name in the label cell ("Assign", "Speed"). Not the group's accessible name (`groupLabel` is). |
| `sublabel` | `string \| undefined` | — | A second label line, 12px `--d` (the Settings rows' optional second line); absent: one line. |
| `groupLabel` | `string` | — | The tab group's `aria-label`, passed to ChosenTabs as `label` ("Assign: which Right parts sound the effect", "Speed: the repeat rate"). |
| `size` | `'row' \| 'row-sm'` | `'row'` | `row`: the row 44 tall with ChosenTabs `row` (32, 24px block). `row-sm`: 36 tall with ChosenTabs `row-sm` (28, 22px block). |
| `tabs` | `TabItem[]` | — | The choices, passed to ChosenTabs unchanged (`TabItem` from `app/src/ui/ChosenTabs/types.ts`, with the `hue`, `weight` and `pad` fields D2 adds). |
| `chosen` | `string \| null` | `null` | The state's value: the tab with this id is inert (a click calls nothing, HA-D27) and wears the face unless `drawn` is given. |
| `drawn` | `string \| undefined` | — | The tab that wears the chosen face when it isn't `chosen` (the arpeggio kind's Auto while `assign` is `multi`, HA-D27). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed through to ChosenTabs, which applies it per tab with each item's `tip` (L3). Stories pass `fn()`. |

Passed to ChosenTabs: `size` (the same name), `label` = `groupLabel`, `tabs`, `chosen`, `drawn`, `tipAction`, `onchoose`.

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchoose` | ChosenTabs' `onchoose`, passed through: a click (or Space / Enter) on a tab other than `chosen` | `(id: string)` the tab's id |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--m`, `--d`, `--line`, `--font-sans`, `--text-12`, `--text-13`, `--weight-regular`, `--space-1`, `--space-12`, `--line-width`, and the new tokens `--label-cell` (64px), `--settings-row` (44px), `--settings-row-sm` (36px), `--lh-14` (14px), `--lh-16` (16px), all defined in BarRow's spec (`app/src/ui/BarRow/SPEC.md` › New tokens) and landing in the tokens contract PR (L1). This component adds no token of its own.
- **The row:** a `div`, its parent's width, `box-sizing: border-box`, height `--settings-row` (`row`) or `--settings-row-sm` (`row-sm`), `border-top: var(--line-width) solid var(--line)`, a flex row, items centred, gap `--space-12`, no wrap, no padding. Carries `data-size`.
- **Label cell** (`data-part="label"`): `width: var(--label-cell)`, `flex: none`, a column with gap `--space-1`, `white-space: nowrap`, `overflow: visible`: the label `--text-13` / 400, line-height `--lh-16`, `--m`; the sublabel (`data-part="sublabel"`, only when given) `--text-12` / 400, line-height `--lh-14`, `--d`. The same cell as BarRow's `setting` variant, so the two line up in one column.
- **Tabs:** ChosenTabs at the row's `size`, `flex: none`, left-aligned after the label cell; its faces, widths, hues and focus ring are ChosenTabs'.
- **States drawn by:** only ChosenTabs' (chosen block, unchosen `--m` or hue, focus ring). The row itself has no hover, no focus and no disabled look.
- **Type:** DM Sans, the label as given.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--m` on `--g` (the label, exists); the tabs' rows are ChosenTabs' (`--g` on `--t`, the hues on `--g`). The sublabel (`--d` on `--g`, 2.48 dark, 1.94 light) is the known O-contrast failure (BarRow D7); no board draws a ChoiceRow sublabel, so no story renders one.
- **Motion:** none.

### Accessibility

- **Role and name:** the row has no role; the one `role="group"` is ChosenTabs', named by `groupLabel`. The label cell is plain text (no `<label>`: there is no form control to point it at), `aria-hidden` is not set (it reads before the group).
- **Keyboard:** ChosenTabs' row keyboard (one tab stop, ← → between tabs, Space / Enter chooses).
- **Tooltip id:** per tab, from each `TabItem.tip`: `harmony.assign` (Assign), `harmony.speed` (Speed); both exist.

## Stories (Story station)

- **Title:** `Primitives/ChoiceRow`.
- **Layout:** `centered`, in a 371px-wide decorator (the Harmony settings column). Meta: `tipAction: fn()`, `onchoose: fn()`.
- Assign's tabs (Harmony.md › Settings column): `[{ id: 'auto', label: 'Auto', tip: 'harmony.assign' }, { id: 'multi', label: 'Multi', tip: 'harmony.assign' }, { id: 'right1', label: 'R1', name: 'Right 1', hue: 'r1', weight: 500, pad: 8, tip: 'harmony.assign' }, { id: 'right2', label: 'R2', name: 'Right 2', hue: 'r2', weight: 500, pad: 8, tip: 'harmony.assign' }, { id: 'right3', label: 'R3', name: 'Right 3', hue: 'r3', weight: 500, pad: 8, tip: 'harmony.assign' }]` (`ASSIGN` below).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ label: 'Assign', groupLabel: 'Assign: which Right parts sound the effect', tabs: ASSIGN, chosen: 'auto' }` | the board's row: hairline, "Assign", Auto in the white block, R1–R3 in their hues | `Board-{dark,light}.png`: Harmony 1021,212 371×44 | the group is named "Assign: which Right parts sound the effect"; Auto has `aria-pressed="true"` and the name "Auto, chosen" |
| `PartChosen` | `Board`'s args with `chosen: 'right2'` | R2 in the white block, no hue (HA-D14); R1, R3 keep theirs | — (not on the board) | R2's `data-face` is "chosen" and it has no `data-hue` |
| `Chooses` | `Board`'s args | — | — | click R1 → `onchoose('right1')`; click Auto (chosen) → `onchoose` not called |
| `DrawnAuto` | `Board`'s args with `tabs` without Multi, `chosen: 'multi'`, `drawn: 'auto'` | Auto drawn chosen while the state is `multi` (the arpeggio kind) | — | Auto has `aria-pressed="true"`; a click on Auto → `onchoose('auto')` |
| `Speed` | `{ label: 'Speed', groupLabel: 'Speed: the repeat rate', size: 'row-sm', tabs: ['1/4','1/6','1/8','1/12','1/16','1/32'].map((s) => ({ id: s, label: s, pad: 8, tip: 'harmony.speed' })), chosen: '1/8' }` | the 36px row with `row-sm` tabs and a 22px block | — (not drawn on a board, HA-D9) | the row's height style is `--settings-row-sm`; "1/8, chosen" |
| `Focused` | `Board`'s args; `parameters: { pseudo: { focusVisible: true } }` | the focus ring on the chosen tab | — | — |

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- ChoiceRow` passes: the cropped story's screenshot is the crop's size and scores at most 0.02, and axe (colour contrast included) finds no violation on any story.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Segment is ChosenTabs `row`.** Effects' Segment (a `role="group"` of `aria-pressed` buttons, 13 / 400, padding 0 10, the chosen face weight 400, the chosen option inert, one tab stop) is the same control as Harmony's ChosenTabs `row`; in Effects' 32px title row (items centred) a 24px Segment and a 32px `row` tab with a centred 24px block draw the same pixels. So there is no Segment component: Effects' Source and Type use ChosenTabs `row` directly, and ChoiceRow is that group with a label cell. Effects' "Segment" references read as ChosenTabs `row`.
- **D2 · What ChosenTabs must gain first.** The `row` and `row-sm` sizes with the group kind, the `drawn` prop and the `hue` / `weight` / `pad` item fields, exactly as listed in the lane report (from Harmony.md › ChosenTabs, size `row`, and Effects.md › Segment's keyboard); this spec doesn't edit ChosenTabs and builds only after that lands.
- **D3 · Keyboard from Effects.** Harmony.md doesn't say how keys move between row tabs; Effects' Segment does (one tab stop, roving `tabindex`, ← → without wrapping, Home / End, keys stopped), so the row sizes follow it and a settings column is one Tab stop per row.
- **D4 · The group's name is the parent's sentence.** The visible label ("Assign") and the group name ("Assign: which Right parts sound the effect") differ on purpose (Harmony.md); `groupLabel` is required rather than derived.
- **D5 · Height follows size.** `row` tabs always sit in a 44px row and `row-sm` in a 36px one (Kit additions › Settings rows), so one `size` prop sets both rather than two props that could disagree.
- **D6 · No crop files yet.** The `crops/` folder doesn't exist; the box above is for the crop station.

Follow-ups: the Settings pages (#528–#533) will use this row; their boards (`SettingsChord`, `SettingsStyle`, …) may add crops and a sublabel story.
