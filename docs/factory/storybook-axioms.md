# Storybook axioms

Rules every component, story and station follows. Where a rule can be checked by a machine, the plant checks it, so a station can't forget it.

## The owner's axioms

1. **Themes are real themes.** Dark and light (and any later theme) come through Storybook's themes addon (`@storybook/addon-themes`' toolbar, with our decorator putting `data-theme` on a wrapper around each story, never on the page), from day one. Components never branch on the theme; they read tokens.
2. **Design tokens for everything.** Every colour, size, space, radius, font, weight, glow and duration a component uses is a CSS custom property. No literal values in components. A theme is just a set of token values, kept so it's easy to tweak.
3. **Controls for everything the story shows.** Every prop a story sets away from its default is a native Storybook control.
   - **A single component:** every prop is a control.
   - **A complex component:** its own props, plus the props of each child the story changes, grouped by child. Use argTypes `table.category`, named after the child, and map the args onto the child props.
   - **The test:** anything you see in a story, you can tweak from the Controls panel.
4. **Native docs and metadata only.** Descriptions, docs and metadata use Storybook's own mechanisms: autodocs, JSDoc on props, `parameters`, `argTypes`, MDX doc pages. Nothing goes into a story's component tree that isn't meant for production. That means no wrapper components, story-only markup, captions or labels inside the rendered story.

## Additions (orchestrator's call; the owner can veto)

5. **Two layers of tokens.** A palette layer of named raw colours (e.g. `--lime-400`) sits under a semantic layer of roles (`--lamp: var(--lime-400)`). Components use only the semantic layer, and a theme maps roles to palette entries. When the artist's palette arrives, we swap the palette layer and nothing else.
6. **A tokens page.** An MDX "Foundations" page shows every token in the active theme: swatches, the type scale, spacing and radius. Reviewing a palette change means flipping the theme on that page.
7. **Events are actions.** Every callback prop is a Storybook action (`fn()`), so clicks log in the Actions panel and `play` functions assert on them.
8. **Accessibility checks run on every story.** The a11y addon runs on every story, and violations fail the story tests.
9. **Interaction states as stories.** Hover, focus and active come from the pseudo-states addon, not from story-only CSS. Every focusable component has a visible focus style that comes from a token.
10. **Deterministic by props.** Anything that moves over time (meters, flashing queued pads, the beat) takes its moment as a prop, such as a level, a beat or a flash phase. Stories and shots are therefore repeatable, and a control can scrub through the motion. Real timers live in the app, never in the component.
11. **One taxonomy.** Story titles follow the inventory: `Primitives/<Name>`, `Components/<Name>`, `Screens/<Name>`. A primitive imports no other component of the library; a component is an assembly of primitives; a screen is an assembly of components. A screen story renders at the app's 1440×900 with `layout: 'fullscreen'`; component stories use `layout: 'centered'` at real size.
12. **Fixtures, not inline data.** Realistic data (style names, sounds, racks) for complex components and pages lives in shared `*.fixtures.ts` files. That's data, not components, and stories import it.
13. **Machine-checked axioms.** The story test checks rules 3, 7 and 8 for every story:
    - every prop of a primitive has a control;
    - no control is disabled;
    - every callback is an action;
    - a11y passes.

    A lint rule checks rule 2: no colour literals in component styles.
