# AGENTS.md

## Commands

- `pnpm dev` — frontend only (Vite on port 1420)
- `pnpm tauri dev` — full dev (builds Rust + serves frontend)
- `pnpm build` — frontend production build → `build/`
- `pnpm check` — `svelte-kit sync && svelte-check`
- Rust: run `cargo check` / `cargo build` from `src-tauri/`

Package manager: **pnpm**. Do not use npm.

## Architecture

- **Tauri v2** app with two windows: `main` (the dock) and `settings`.
- Frontend is **Svelte 5** (runes `$state`, no Svelte stores). SvelteKit runs in **SPA mode** (`ssr = false` in `src/routes/+layout.ts`), adapter-static with `index.html` fallback.
- Backend is **Rust** in `src-tauri/src/`. Windows-only features: work-area detection, .lnk parsing, icon extraction (GDI), process tracking. Non-Windows commands return errors.
- Two windows communicate via Tauri events: `config-changed`, `app-launched`, `app-exited`, `fullscreen-detected`, `fullscreen-cleared`.

## Key files

- `src/lib/stores/dockStore.svelte.ts` — frontend state + debounced config saves + event listeners (loaded in app entry)
- `src-tauri/src/lib.rs` — Tauri commands + window setup (tray, fullscreen detector, WS_EX_NOACTIVATE)
- `src-tauri/src/config.rs` — `DockConfig` struct, JSON persistence in app data dir
- `src-tauri/src/launcher.rs` — process tracking via `Child` handles

## Quirks

- Dev server port is **fixed at 1420** (`vite.config.js`, `strictPort: true`).
- Vite ignores `src-tauri/**` for HMR.
- `pnpm-workspace.yaml` only allows `esbuild` builds.
- No test suite exists. No lint/format config found.
- Config defaults are duplicated in Rust (`config.rs`) and TypeScript (`dockStore.svelte.ts`); keep them in sync.
