# Testing

This is a clean copy of the project's testing guidance, prepared as a starting doc for a new Rust + WASM UI project. It has been sanitized to remove project-specific editorial notes about unrelated media/player policies.

## Quick verification
- Type checks: `cd frontend && npm run typecheck`
- Build: `cd frontend && npm run verify`
- Rust tests: `cd wasm-core && cargo test`

## Recommended acceptance criteria for demo builds
- UI panels: per-window minimize/restore via title bar, keyboard accessible.
- Theme controls: interactive tiles respond to clicks and keyboard.
- Edit mode: panels draggable and reorderable in the theater area.

## Capture & showcase
- Build and run preview, then capture short GIFs/screenshots to include with PRs or portfolio entries.

[This file is a clean base-docs snapshot created 2026-04-16.]
