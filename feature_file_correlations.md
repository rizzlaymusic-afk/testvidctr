# Feature File Correlations

This file maps feature areas to source files. Use it to define feature groups, list related files, and provide edit guidance. This base copy is a sanitized template for new projects.

## Template

## Group 1 — App shell and startup

**Files**:
- `src/App.tsx`

**Why correlated**: Coordinates app boot, routing, and global state.

**Edit guidance**: If editing `App.tsx`, verify route initialization and global state consumers.

---

## Group 2 — Playback

**Files**:
- `src/components/Playback/*`

**Why correlated**: Implements playback controls and media handling.

**Edit guidance**: Verify interactions with the session and storage layers when changing playback code.

[Sanitized base-docs snapshot created 2026-04-16.]
