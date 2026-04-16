# Checklist

## Pre-Commit Checklist

- [ ] Code compiles cleanly.
- [ ] No new lint errors.
- [ ] Tests pass for affected files.
- [ ] Documentation related to the change is updated.
- [ ] Backup created in `_backups/` before editing.
- [ ] No TODO/FIXME placeholders remain.
- [ ] Changelog entry drafted.

## Pre-PR Checklist

- [ ] PR title follows branch naming and commit conventions.
- [ ] Description includes the problem, fix, and docs updated.
- [ ] Screenshots or logs included if UI/behavior changed.
- [ ] Related docs links are listed.
- [ ] Backup and revert notes are included.

## Pre-Deploy Checklist

- [ ] Build passes in the target environment.
- [ ] Database schema and migrations are verified.
- [ ] Environment variables are documented.
- [ ] Rollback plan is defined.
- [ ] Post-deploy validation steps are documented.

## Accessibility Checklist

- [ ] Keyboard navigation works for all new UI.
- [ ] ARIA roles and labels are present.
- [ ] Contrast ratios meet WCAG AA.
- [ ] Focus styles are visible.
- [ ] Screen reader text is accurate.

## Security Checklist

- [ ] No new broad permissions are added.
- [ ] Data writes are validated before exec.
- [ ] User-selected paths are handled safely.
- [ ] Secrets are not stored in source files.
- [ ] Third-party dependencies are vetted.

## Post-Deploy Validation

- [ ] Verify critical flows manually.
- [ ] Confirm analytics/logging behavior.
- [ ] Check for regressions in the changed area.
- [ ] Confirm that docs and changelog reflect the deployed version.

## Workspace Todo

- **Discovery**: Reproduce issues and collect logs/telemetry. (Status: Completed)
- **Fix build warnings/linker**: Address compiler warnings and linker failures across targets. (Status: In progress)
- **Stabilize dev flow & assets**: Ensure dev build scripts, sample assets, and dev server reliability. (Status: In progress)
- **Enforce U-BFCW UI rules**: Implement and audit U-BFCW UI standards across components.
- **WASM core API stability**: Harden public WASM core APIs and maintain ABI/semantic stability.
- **Frontend↔WASM integration tests**: Add integration tests for data exchange and runtime interactions.
- **Playback/trim correctness**: Verify playback fidelity and trimming algorithms (edge cases).
- **Export reliability**: Harden export pipelines, formats, and error handling.
- **Backend WebSocket tests**: Add tests and mocks for WebSocket comms and reconnection behavior.
- **CI automation**: Add CI jobs for builds, wasm tests, and integration smoke tests.
- **Documentation & handover**: Update docs, runbooks, and handover notes for maintainers.
- **Final verification & release**: Acceptance testing, release checklist, tagging, and publish steps.

## Handover — Dev Runbook (quick)

Follow these steps to run the project in local dev mode and verify the end-to-end trim → export flow used during handover.

- **Prereqs:** Rust toolchain (stable), `wasm32-unknown-unknown` target, `trunk`, `node` + `npm` (for smoke-test), and `cargo`.

- **1) Start backend (optional for local-only UX):**

```powershell
# Run from repo root
cargo run -p flashcut-backend
```

- **2) Start frontend dev server (Trunk):**

```powershell
# from repo root
Push-Location crates/frontend
trunk serve --address 127.0.0.1 --port 8080 --open
Pop-Location
```

- **3) Load a sample video (dev):**

Use the UI `Load Sample Video (dev)` button in the File input drop zone, or set the file input manually for testing:

```powershell
# The smoke-test helper sets the input programmatically; manual alternative:
# Open the app in a browser and choose assets/sample/sample.webm via the file dialog.
```

- **4) Run the headless smoke test (sanity):**

```powershell
Push-Location tools/smoke-test
npm install
npx playwright install chromium
npm run smoke
Pop-Location
```

- **5) Verify export:**

- Watch the in-app progress bar and `Session` panel for errors. In headless runs the smoke test detects encoder console logs named `encoder: assembled blob size` and `encoder: calling final progress 1.0` as success signals.

- **6) Debugging tips:**

- If export stalls, check browser console for `captureStream not supported` or `MediaRecorder not available`.
- If running headless, ensure Chromium supports `MediaRecorder` and `captureStream`. The smoke test runs a local Chromium binary installed by Playwright.
- If `trunk` fails to bind port 8080, find and stop the existing `trunk` process: `netstat -ano | Select-String ":8080"` then `Stop-Process -Id <pid>`.

## Handover next actions (short)

- Finalize `FLASHCUT_HANDOVER.md` with annotated screenshots and a short checklist of U-BFCW audit items. (Owner: maintainer)
- Commit `README.dev.md` with the minimal runbook snippets above and attach smoke-test results (log) to the handover package.
- Add a CI job to run `tools/smoke-test` on a matrix that supports Playwright/Chromium (optional; needed for release gating).

---

Keep this section short and copy it into `FLASHCUT_HANDOVER.md` as a highlighted quick-start block for incoming maintainers.
