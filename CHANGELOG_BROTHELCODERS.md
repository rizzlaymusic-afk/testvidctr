# Changelog

All changes appended here. Do not rewrite existing entries — add a new section per change.

## [Unreleased]

---

## [1.0.0] — 2026-04-09

### Initial Release

- Ghost System v1.0 — all 8 phases fully implemented and smoke-tested
- Phase 1: Risk Scanner (`risk_scanner.py` + `patterns.py`) — language-agnostic heuristic scan, TypeScript/JavaScript/Python/Rust support
- Phase 2: Blast Radius Analyzer (`blast_radius.py`) — import graph traversal, feature group impact mapping
- Phase 3: Conflict Detector (`conflict_detector.py`) — schema/contract/config conflict detection
- Phase 4: Baseline Tracker (`baseline_tracker.py`) — SQLite-backed scan comparison, regression detection, health scoring
- Phase 5: Agent Personas (`ghost_prompts/`) — Watchdog, Triage, Blast, Conflict, Security, Dependency analyst personas
- Phase 6: Unified Runner (`brothel_ghost.py`) — single CLI for all phases, `--headless` mode, `all` pipeline
- Phase 6: Dashboard (`serve.py` + `dashboard.html`) — 9-page web UI, CDN-only, hash routing
- Phase 7: Security Scanner (`security_scanner.py` + `security_patterns.py`) — OWASP Top 10 pattern library
- Phase 8: Dependency Auditor (`deps_auditor.py`) — OSV.dev CVE lookup, license audit, abandoned package detection
- `ghost_security.instructions.md` and `ghost_deps.instructions.md` — Phase 7/8 analyst personas
- `DEPLOY.md` — step-by-step guide for dropping Ghost System into any project
- Clean source package: all project-specific references removed, ready for use in any workspace
