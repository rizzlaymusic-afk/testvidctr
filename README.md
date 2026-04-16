# FlashCut — Project Skeleton

This workspace is a scaffold created from the provided handover. It contains initial crates for `shared`, `core-wasm`, `frontend` and `backend`, plus CI and Docker configs.

Next steps: build the WASM core, run tests, and start the dev servers.

# Brothel Coders Agent Workspace

**Ghost System** — a codebase triage, regression monitoring, and security auditing toolkit for any VS Code project.  
No API key required. No paid services. Powered by your active Copilot session.

---

## Workspace Note

This workspace also contains **Ghost Terminal**, a Rust + WebAssembly portfolio application with a configurable React workstation UI.

- Latest app upgrade: deterministic A* pathfinding workload with an animated route theater for recruiter-facing demos
- Latest interaction upgrade: dockable windows with real minimize, expand, and resize controls across the dashboard and theme studio.
- Showcase assets: [frontend/public/media/dashboard-workstation.png](frontend/public/media/dashboard-workstation.png), [frontend/public/media/pathfinding-route-theater.gif](frontend/public/media/pathfinding-route-theater.gif)

- App docs: [frontend/README.md](frontend/README.md)
- Frontend app: [frontend/src/App.tsx](frontend/src/App.tsx)
- Rust core: [wasm-core/src/lib.rs](wasm-core/src/lib.rs)

---

## What It Does

The Ghost System answers questions any agent or developer needs when working on a codebase:

| Question | Tool |
|---|---|
| Where are the Red Flags, Landmarks, Dead Zones, Fault Lines? | Phase 1: Risk Scanner |
| If I touch this file, what will break? | Phase 2: Blast Radius Analyzer |
| Are parts of the codebase disagreeing right now? | Phase 3: Conflict Detector |
| Did I introduce new problems? | Phase 4: Baseline Tracker |
| Are there OWASP security issues? | Phase 7: Security Scanner |
| Are dependencies CVE-affected, unlicensed, or abandoned? | Phase 8: Dependency Auditor |
| What does all of this look like? | Phase 6: Web Dashboard |

All scanning is done locally by Python scripts. All reasoning is done by your active Copilot/agent session. Zero per-use cost. Zero cloud dependency.

## Contributing & Non-code Contributions

I welcome contributions of many kinds — code is great, but non-code work is often the fastest way to demonstrate impact and build a portfolio. Below are easy, high-impact ways you (or a recruiter reviewing your PRs) can show real ownership:

- **Documentation improvements** — fix typos, clarify instructions, add examples and step-by-step walkthroughs. Small docs PRs are very welcome and show attention to detail.
- **Add tests** — write unit or integration tests for fragile logic. A PR that adds tests demonstrates engineering rigor and confidence.
- **Examples & tutorials** — create short, focused guides or runnable examples that demonstrate how features work (e.g., a pathfinding demo recipe).
- **Qualitative bug reports** — a clear bug report with reproduction steps, expected vs actual, and a proposed fix is often more valuable than a half-baked patch.

Before editing, original docs were backed up to `bin/docs_backup_2026-04-16` so you can always review the prior versions.

See `CONTRIBUTING.md` for an expanded guide on how I review contributions and how to present work on your portfolio.

---

## Quick Start

```bash
# 1. Point the scanner at your project's source
python tools/risk_scanner/risk_scanner.py --root ../your-project/src

# 2. Set a baseline
python tools/baseline_tracker/baseline_tracker.py --label my-project-v1

# 3. Open the dashboard
python tools/brothel_ghost.py ui
# → http://localhost:7892

# 4. Headless status check (for agent/CI)
python tools/brothel_ghost.py status --headless
```

**Requirements**: Python 3.8+, stdlib only (zero pip dependencies).

---

## Deploying to a Target Project

To embed the Ghost System into another project:

1. Read **`tools/ghost_prompts/DEPLOY.md`** — step-by-step guide for copying the system into any project
2. Wire `.github/copilot-instructions.md` for Copilot auto-load on every session
3. Run initial scan + baseline to activate regression monitoring

---

## Project Structure

```
.
├── GHOST_SYSTEM.md                   ← Master technical reference
├── README.md                         ← This file
├── CHANGELOG.md
├── feature_file_correlations.md      ← Fill in for your project
├── .gitignore
├── LICENSE
├── bin/                              ← Backup archive (never delete — only move here)
│
├── .github/
│   └── copilot-instructions.md       ← Auto-loaded at every Copilot session start
│
└── tools/
    ├── brothel_ghost.py              ← Unified CLI runner (all phases)
    ├── brothel_ghost_runner.md       ← Full CLI reference
    │
    ├── ghost_prompts/                ← Agent persona instruction files
    │   ├── DEPLOY.md                 ← How to drop Ghost System into any project
    │   ├── ghost_watchdog.instructions.md
    │   ├── ghost_triage.instructions.md
    │   ├── ghost_blast.instructions.md
    │   ├── ghost_conflict.instructions.md
    │   ├── ghost_security.instructions.md
    │   └── ghost_deps.instructions.md
    │
    ├── risk_scanner/                 ← Phase 1: Heuristic risk scan
    ├── blast_radius/                 ← Phase 2: Dependency impact analysis
    ├── conflict_detector/            ← Phase 3: Contract/schema conflict detection
    ├── baseline_tracker/             ← Phase 4: Regression tracking over time
    ├── ui/                           ← Phase 6: 9-page web dashboard
    ├── security_scanner/             ← Phase 7: OWASP-style security scan
    ├── deps_auditor/                 ← Phase 8: CVE + license + abandoned check
    ├── report_templates/             ← Blank report templates (POI, Blast, Conflict)
    │
    └── (optional Phase 0 documentary tools)
        ├── scanner/                  ← TypeScript AST scanner
        ├── exporter/                 ← Documentary → JSON
        ├── importer/                 ← JSON → SQLite (includes schema_findings.sql)
        ├── reporter/                 ← SQLite → coverage report
        └── planner/                  ← Coverage gaps → harvest backlog
```

---

## CLI Reference

```bash
# Run a full pipeline
python tools/brothel_ghost.py all --root ../src --label my-project-v1

# Individual phases
python tools/brothel_ghost.py scan ../src
python tools/brothel_ghost.py blast src/main.ts
python tools/brothel_ghost.py conflicts ..
python tools/brothel_ghost.py baseline my-project-v1
python tools/brothel_ghost.py security ../src
python tools/brothel_ghost.py deps .
```
