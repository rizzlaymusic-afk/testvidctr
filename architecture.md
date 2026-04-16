# Architecture

This is a clean base-docs snapshot intended as a starting architecture reference for a new Rust + WASM UI project (Tauri optional). Project-specific media/player policy notes have been removed.

## System Overview
- Frontend: React + TypeScript
- Backend: Rust (optional) for heavy compute and WASM bridge
- Persistence: SQLite or other embedded DB

## Component Guidance
- Keep UI decoupled into small components (panels wrapped by a shared WindowFrame)
- Isolate heavy compute into WASM or background workers
- Document feature-file correlations in `feature_file_correlations.md`

## Change Management
- Document architecture changes in `architecture.md` and reference `CHANGELOG.md`.

[Sanitized base-docs snapshot created 2026-04-16.]
