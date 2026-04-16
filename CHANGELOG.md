# Changelog

All changes appended here. Do not rewrite existing entries — add a new section per change.

## [Unreleased]

### Fixed — UI/UX — 2026-04-16
- `frontend/src/components/layout/WindowFrame.tsx`: per-window minimize/restore via full chrome bar click; keyboard toggle (Enter/Space); improved resize-edge behavior so clicks are not intercepted.
- `frontend/src/components/layout/ThemeStudio.tsx`: removed WindowFrame wrapper to avoid pointer interception; theme tiles and swatches are now clickable.
- `frontend/src/pages/Dashboard.tsx`: added Edit Mode drag-to-rearrange for theater panels; converted left/right rails to independent `WindowFrame` panels to avoid global collapse side-effects.
- `frontend/src/store/workspaceContext.tsx`: removed global `leftCollapsed` / `rightCollapsed` flags that caused entire columns to vanish when collapsing a single panel.
- Backups: created safe backups of edited files under `bin/` before large changes.
- Verified: frontend build and `vite preview` run cleanly; no TypeScript build errors reported.

---

## [2.0.0] — 2026-04-16 — Ghost Terminal Dashboard (Full Implementation)

[Truncated for clean base-docs snapshot.]
