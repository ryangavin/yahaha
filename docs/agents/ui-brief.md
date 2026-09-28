# UI design decisions

The yahaha desktop app lives in `app/` (Svelte 5, TypeScript, Vite, inside Tauri 2). Its main screen is a skeuomorphic 1:1 mirror of the owner's Novation Launchkey MK4 (49/61 keys), with a sleek look in the spirit of Sylenth1.

- Use no Yamaha or Novation assets.
- Match the existing design system: the tokens and material recipes in `app/src/app.css`, the shared components in `app/src/lib/ui/`, and `app/CONTRIBUTING.md`.
- Hold 60 Hz with no jank. Don't put blur filters on animating elements.
- The UI renders from state and sends actions. Never re-implement engine behaviour in TypeScript.
- Every interactive control gets a tooltip from the single catalog `app/src/help/tooltips.ts`. Each tooltip gives what the control does, the Genos term, the keyboard shortcut, and where it lives on the Launchkey, if anywhere.
- The engine API is documented in `docs/app-api.md` (AppCmd / AppState). If a panel needs data the API doesn't have, don't invent engine behaviour — mock it with the proposed shape in `app/src/lib/api/`.
