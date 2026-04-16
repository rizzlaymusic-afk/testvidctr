FlashCut — Developer Quick Start

This file contains minimal steps to run the project locally for development and verification (trim → export).

Prerequisites
- Rust (stable) with `wasm32-unknown-unknown` target
- trunk (install via `cargo install trunk`)
- Node.js + npm (for smoke-test Playwright)

Quick Run

Start backend (optional):

```powershell
# from repo root
cargo run -p flashcut-backend
```

Start frontend dev server (Trunk):

```powershell
# from repo root
Push-Location crates/frontend
trunk serve --address 127.0.0.1 --port 8080 --open
Pop-Location
```

Load sample video
- Use the `Load Sample Video (dev)` button in the File input drop zone.
- Or run the smoke test which sets the file input programmatically.

Run headless smoke test

```powershell
Push-Location tools/smoke-test
npm install
npx playwright install chromium
npm run smoke
Pop-Location
```

Troubleshooting
- If export stalls, open DevTools Console; look for `captureStream not supported` or `MediaRecorder not available`.
- If `trunk` fails on port 8080, stop existing process: `netstat -ano | Select-String ":8080"` then `Stop-Process -Id <pid>`.

Commit & push
- After edits, commit and push to your remote branch.

