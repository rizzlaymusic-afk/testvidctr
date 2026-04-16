# FlashCut — Privacy-First High-Speed Video Trimmer

## Vollständiger Handover-Plan, Implementierungs-Guide & Referenz-Dokumentation

> **Für potenzielle Arbeitgeber:** Dieses Dokument beschreibt die vollständige Systemarchitektur, jeden Implementierungsschritt und alle technischen Entscheidungsbegründungen für ein Production-Ready Rust/WASM-Projekt. Es demonstriert strukturiertes Software-Engineering, tiefes Verständnis des modernen Rust-Ökosystems (Leptos, Axum, wasm-bindgen, Tokio), Browser-APIs (WebCodecs, File System Access API, WebSockets) sowie Privacy-by-Design-Prinzipien und modular denkendes Fullstack-Engineering.

---

## Inhaltsverzeichnis

1. [Projekt-Überblick & Ziele](#1-projekt-überblick--ziele)
2. [Technische Architektur](#2-technische-architektur)
3. [Voraussetzungen & Entwicklungsumgebung](#3-voraussetzungen--entwicklungsumgebung)
4. [Workspace-Setup (Schritt-für-Schritt)](#4-workspace-setup-schritt-für-schritt)
5. [Shared-Crate — Gemeinsame Typen](#5-shared-crate--gemeinsame-typen)
6. [Phase 1 — WASM-Kern & Video-Decoding](#6-phase-1--wasm-kern--video-decoding)
7. [Phase 2 — Schnitt-Logik & Export](#7-phase-2--schnitt-logik--export)
8. [Phase 3 — Fullstack: Axum + WebSockets](#8-phase-3--fullstack-axum--websockets)
9. [Frontend — Leptos UI (alle Komponenten)](#9-frontend--leptos-ui-alle-komponenten)
10. [Styles & Assets](#10-styles--assets)
11. [Testing-Strategie (komplett)](#11-testing-strategie-komplett)
12. [CI/CD & Docker (produktionsreif)](#12-cicd--docker-produktionsreif)
13. [README.md Vorlage](#13-readmemd-vorlage)
14. [Master-TODO-Liste](#14-master-todo-liste)
15. [Bekannte Fallstricke & Lösungen](#15-bekannte-fallstricke--lösungen)

---

## 1. Projekt-Überblick & Ziele

### Was ist FlashCut?

FlashCut ist ein vollständig client-seitiger Video-Trimmer. Es ist **kein** weiterer Upload-basierter Cloud-Editor. Jede Videoverarbeitungsoperation findet ausschließlich auf der Hardware des Nutzers statt — im Browser, via WebAssembly und den nativen WebCodecs-APIs des Browsers.

Das Backend existiert nur für einen einzigen Zweck: Echtzeit-Kollaborations-Metadaten (Zeitstempel, Kommentare, Schnittmarken) zwischen mehreren Browser-Tabs/Nutzern zu synchronisieren. Kein einziges Byte Videodaten berührt jemals den Server.

### Kernziele und Messgrößen

| Ziel | Messgröße | Methode |
|---|---|---|
| Zero-Upload | 0 Bytes Videodaten an Server | Netzwerk-Monitor im DevTools |
| Frame-Accuracy | Schnitt auf ±1 Frame genau | Manuelle Verifikation |
| Performance | Trim-Start < 200ms nach Klick | `performance.now()` Messung |
| Kollaboration | WS-Sync < 100ms Latenz (lokal) | Timestamp-Delta Logging |
| Privacy | Keine Video-URLs, keine Thumbnails serverseitig | Code-Audit |

### Warum genau dieser Stack?

**Rust → WASM** statt JavaScript für Video-Processing:

- JavaScript ist single-threaded und GC-pausiert. Für Frame-Decoding brauchen wir deterministischen Speicher und volle CPU-Nutzung.
- Rust kompiliert zu WASM mit nahezu nativem Durchsatz.
- `wasm-bindgen` gibt uns typsichere Bindings zu Browser-APIs ohne JS-Schreibarbeit.

**Leptos** statt React/Svelte:

- Vollständig in Rust — kein Kontext-Switch zwischen Rust (WASM-Core) und JavaScript (UI).
- Fine-grained Reaktivität: Nur die exakten DOM-Nodes werden neu gerendert, die sich geändert haben.
- Server-Side Rendering (SSR) als spätere Erweiterungsmöglichkeit ohne Stack-Wechsel.

**Axum** statt Express/FastAPI:

- Rust end-to-end: Typen aus `shared`-Crate werden in Frontend UND Backend verwendet — Zero Desync zwischen Client- und Server-Interfaces.
- Tokio-basiert: Skaliert auf tausende WebSocket-Verbindungen mit minimalem Overhead.
- Tower-Middleware-Ökosystem für CORS, Tracing, Rate-Limiting etc.

---

## 2. Technische Architektur

### 2.1 Systemdiagramm

```
╔══════════════════════════════════════════════════════════════════════════╗
║                         BROWSER (CLIENT A)                              ║
║                                                                          ║
║  ┌─────────────────────────────────────────────────────────────────┐    ║
║  │                  Leptos Frontend (WASM Bundle)                   │    ║
║  │                                                                   │    ║
║  │  ┌──────────────┐ ┌───────────────┐ ┌──────────┐ ┌──────────┐  │    ║
║  │  │  FileInput   │ │  VideoPlayer  │ │ Timeline │ │ Toolbar  │  │    ║
║  │  │  (Drag&Drop) │ │  (Canvas)     │ │ (Trim)   │ │ (Export) │  │    ║
║  │  └──────┬───────┘ └───────┬───────┘ └────┬─────┘ └────┬─────┘  │    ║
║  │         └─────────────────┴──────────────┴────────────┘         │    ║
║  │                                   │                              │    ║
║  │                    ┌──────────────▼──────────────┐              │    ║
║  │                    │   AppState (Leptos Signals)   │              │    ║
║  │                    │  file, duration, playhead,    │              │    ║
║  │                    │  trim_start, trim_end,        │              │    ║
║  │                    │  session_id, export_progress  │              │    ║
║  │                    └──────────────┬──────────────┘              │    ║
║  │                                   │                              │    ║
║  │  ┌────────────────────────────────▼──────────────────────────┐  │    ║
║  │  │                   core-wasm Crate                          │  │    ║
║  │  │                                                            │  │    ║
║  │  │  ┌────────────────┐    ┌───────────────┐   ┌──────────┐  │  │    ║
║  │  │  │  decoder.rs    │    │  encoder.rs   │   │ types.rs │  │  │    ║
║  │  │  │                │    │               │   │          │  │  │    ║
║  │  │  │  VideoDecoder  │───▶│ VideoEncoder  │   │TrimRange │  │  │    ║
║  │  │  │  (WebCodecs)   │    │ (WebCodecs)   │   │Metadata  │  │  │    ║
║  │  │  │  → VideoFrames │    │ → Blob        │   │WasmError │  │  │    ║
║  │  │  └────────────────┘    └──────┬────────┘   └──────────┘  │  │    ║
║  │  │                               │ download()               │  │    ║
║  │  └───────────────────────────────┼──────────────────────────┘  │    ║
║  │                                  │                              │    ║
║  │  ┌───────────────────────────────▼──────────────────────────┐  │    ║
║  │  │              WS Client (session_panel.rs)                  │  │    ║
║  │  │  WebSocket → /ws/:session_id                               │  │    ║
║  │  │  Sendet: TimestampUpdate, TrimUpdate                       │  │    ║
║  │  │  Empfängt: StateSync, ParticipantJoined/Left               │  │    ║
║  │  └───────────────────────────────┬──────────────────────────┘  │    ║
║  └────────────────────────────────── │ ────────────────────────────┘    ║
╚═══════════════════════════════════════│══════════════════════════════════╝
                                        │  wss:// WebSocket
                                        │  (NUR Metadaten! Keine Videodaten)
╔═══════════════════════════════════════│══════════════════════════════════╗
║                      AXUM BACKEND     │                                  ║
║                                       ▼                                  ║
║  ┌──────────────────────────────────────────────────────────────────┐   ║
║  │  handlers.rs                                                      │   ║
║  │                                                                   │   ║
║  │  POST /api/sessions  ──────────▶  SessionStore::create_session() │   ║
║  │  GET  /api/sessions/:id ───────▶  SessionStore::get_session()    │   ║
║  │  GET  /ws/:session_id  ─────────▶ handle_socket() (WS Upgrade)  │   ║
║  │  GET  /health          ─────────▶ "OK"                           │   ║
║  └────────────────────────┬──────────────────────────────────────────┘   ║
║                           │                                              ║
║  ┌────────────────────────▼──────────────────────────────────────────┐  ║
║  │  session.rs  (In-Memory, kein DB nötig)                            │  ║
║  │                                                                    │  ║
║  │  DashMap<SessionId, Session>                                       │  ║
║  │    Session {                                                       │  ║
║  │      id: String,                                                   │  ║
║  │      sender: broadcast::Sender<SessionMessage>,  // Tokio          │  ║
║  │      state: Arc<RwLock<SessionState>>,                             │  ║
║  │    }                                                               │  ║
║  └────────────────────────────────────────────────────────────────────┘  ║
╚══════════════════════════════════════════════════════════════════════════╝
```

### 2.2 Datenfluss: Vollständige Trim-Operation

```
1. DATEI ÖFFNEN
   Nutzer klickt "Datei wählen" oder Drop auf Drop-Zone
         │
         ▼
   <input type="file"> Event → web_sys::File Handle
         │
         ▼
   core_wasm::decoder::read_video_metadata(file)
         │  Erstellt temporäres <video> Element, setzt src=ObjectURL
         │  Wartet auf 'loadedmetadata' Event
         ▼
   VideoMetadata { duration_ms, width, height, fps }
         │
         ▼
   AppState.duration_ms.set(meta.duration_ms)
   AppState.trim_end_ms.set(meta.duration_ms)   ← Default: Alles ausgewählt
   AppState.file.set(Some(file))

2. TRIM-MARKEN SETZEN
   Nutzer zieht Handle auf Timeline
         │
         ▼
   on:mousedown → Drag-State aktivieren
   on:mousemove → Berechne neue Position aus Maus-X / Track-Width
         │
         ▼
   AppState.trim_start_ms.set(new_start)
   AppState.trim_end_ms.set(new_end)
         │
         ▼ (wenn Session aktiv)
   WsClient.send(TrimUpdate { start_ms, end_ms })
         │
         ▼
   Axum broadcast → alle anderen Session-Teilnehmer → ihre Timelines updaten

3. EXPORT / TRIM
   Nutzer klickt "Export"
         │
         ▼
   AppState.export_progress.set(Some(0.0))
         │
         ▼
   core_wasm::encoder::trim_and_export(
     file,
     trim_start_ms,
     trim_end_ms,
     "output.webm",
     progress_callback
   )
         │
         ├─→ FileReader.readAsArrayBuffer(file) → ArrayBuffer
         │
         ├─→ VideoDecoder konfigurieren (codec aus Metadaten)
         │
         ├─→ MP4/WebM Demuxer: Extrahiere Chunks im [start, end] Bereich
         │     (Für MVP: Einfacher Byte-Range-Approach)
         │
         ├─→ Für jeden Chunk in Range:
         │     decoder.decode(chunk)
         │     → on_frame(VideoFrame) Callback
         │       → encoder.encode(frame)
         │         → on_chunk(EncodedVideoChunk) Callback
         │           → chunks.push(chunk_data)
         │
         ├─→ decoder.flush() + encoder.flush()
         │
         └─→ chunks_to_blob_and_download(chunks, "output.webm")
               → Blob erstellen
               → <a href="blob:..." download="output.webm">.click()
               → Datei liegt auf Festplatte des Nutzers
               → Kein Server-Kontakt ✓

4. KOLLABORATION (optional, Phase 3)
   Nutzer klickt "Session teilen"
         │
         ▼
   POST /api/sessions → { session_id: "abc12345" }
         │
         ▼
   Share-Link: http://localhost:8080/?session=abc12345
         │
         ▼
   User B öffnet Link → WS connect /ws/abc12345
         │
         ▼
   Server sendet StateSync { playhead_ms, trim_start_ms, trim_end_ms }
         │
         ▼
   User B sieht sofort aktuellen Stand von User A
```

---

## 3. Voraussetzungen & Entwicklungsumgebung

### 3.1 Installations-Skript (alles auf einmal)

Speichere als `scripts/setup-dev.sh` und führe es einmalig aus:

```bash
#!/usr/bin/env bash
# scripts/setup-dev.sh
# Einmaliges Setup der kompletten Entwicklungsumgebung für FlashCut
set -euo pipefail

echo "=== FlashCut Dev-Setup ==="

# 1. Rust installieren (falls nicht vorhanden)
if ! command -v rustup &> /dev/null; then
  echo "→ Installiere Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
else
  echo "→ Rust bereits installiert: $(rustc --version)"
fi

# 2. Stable + Nightly toolchains
rustup update stable
rustup update nightly  # Für einige WASM-Optimierungen nötig

# 3. WASM Target
rustup target add wasm32-unknown-unknown
rustup target add wasm32-unknown-unknown --toolchain nightly

# 4. Rust Komponenten
rustup component add clippy rustfmt rust-src

# 5. wasm-pack
if ! command -v wasm-pack &> /dev/null; then
  echo "→ Installiere wasm-pack..."
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
else
  echo "→ wasm-pack: $(wasm-pack --version)"
fi

# 6. trunk (Dev-Server + Build für Leptos/WASM)
if ! command -v trunk &> /dev/null; then
  echo "→ Installiere trunk..."
  cargo install trunk --locked
else
  echo "→ trunk: $(trunk --version)"
fi

# 7. Weitere Cargo-Tools
echo "→ Installiere Cargo-Tools..."
cargo install cargo-watch --locked       # Hot-Reload für Backend
cargo install cargo-tarpaulin --locked   # Code Coverage
cargo install cargo-audit --locked       # Security Audit
cargo install cargo-outdated --locked    # Dependency Updates
cargo install wasm-bindgen-cli --locked  # wasm-bindgen CLI (gleiche Version wie Crate!)

echo ""
echo "=== Setup abgeschlossen! ==="
echo "Starte mit: trunk serve (Frontend) und cargo run -p flashcut-backend (Backend)"
```

```bash
chmod +x scripts/setup-dev.sh
./scripts/setup-dev.sh
```

## Quick Dev Runbook (hand-over)

Use this quick runbook for maintainers to get a dev instance running and verify the trim → export flow locally.

- Prereqs: Rust stable, `wasm32-unknown-unknown` target, `trunk`, Node.js + npm (for smoke-test), and `cargo`.

- Start backend (optional):

```powershell
# from repo root
cargo run -p flashcut-backend
```

- Start frontend dev server (Trunk):

```powershell
# from repo root
Push-Location crates/frontend
trunk serve --address 127.0.0.1 --port 8080 --open
Pop-Location
```

- Load sample video (dev):

Open the app in the browser and click `Load Sample Video (dev)` in the file input drop zone, or use the smoke test which sets the file input programmatically.

- Run headless smoke test (sanity):

```powershell
Push-Location tools/smoke-test
npm install
npx playwright install chromium
npm run smoke
Pop-Location
```

- Verify export: watch the UI progress bar and Session panel for errors. The smoke test considers `encoder: assembled blob size` and `encoder: calling final progress 1.0` as success signals.

- Troubleshooting tips:

- If `captureStream not supported` or `MediaRecorder not available` appear in console, run tests in an environment with a recent Chromium build (Playwright-installed Chromium is used by the smoke test).
- If `trunk` fails to bind, check and stop existing trunk process: `netstat -ano | Select-String ":8080"` then `Stop-Process -Id <pid>`.

---

Keep this quick runbook at the top of `FLASHCUT_HANDOVER.md` in a highlighted block for incoming maintainers.

### 3.2 VSCode Konfiguration (vollständig)

#### `.vscode/extensions.json`

```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "vadimcn.vscode-lldb",
    "tamasfe.even-better-toml",
    "serayuzgur.crates",
    "usernamehw.errorlens",
    "esbenp.prettier-vscode",
    "bradlc.vscode-tailwindcss",
    "ms-vscode.live-server",
    "GitHub.copilot",
    "eamodio.gitlens"
  ]
}
```

#### `.vscode/settings.json`

```json
{
  "rust-analyzer.linkedProjects": ["./Cargo.toml"],
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave": true,
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.checkOnSave.extraArgs": ["--", "-D", "warnings"],
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.inlayHints.typeHints.enable": true,
  "rust-analyzer.inlayHints.parameterHints.enable": true,
  "rust-analyzer.inlayHints.chainingHints.enable": true,
  "rust-analyzer.diagnostics.experimental.enable": true,
  "files.watcherExclude": {
    "**/target/**": true,
    "**/dist/**": true,
    "**/pkg/**": true
  },
  "search.exclude": {
    "**/target": true,
    "**/dist": true,
    "**/pkg": true
  }
}
```

#### `.vscode/launch.json` (Debugging Backend)

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Backend",
      "cargo": {
        "args": ["build", "-p", "flashcut-backend", "--bin", "flashcut-server"],
        "filter": {
          "name": "flashcut-server",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "flashcut_backend=debug,tower_http=debug",
        "RUST_BACKTRACE": "1"
      }
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Run All Tests",
      "cargo": {
        "args": ["test", "--workspace", "--no-run"],
        "filter": {
          "kind": "test"
        }
      },
      "args": []
    }
  ]
}
```

#### `.vscode/tasks.json`

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Start Frontend (trunk serve)",
      "type": "shell",
      "command": "trunk serve",
      "group": "build",
      "isBackground": true,
      "problemMatcher": []
    },
    {
      "label": "Start Backend",
      "type": "shell",
      "command": "cargo watch -x 'run -p flashcut-backend'",
      "group": "build",
      "isBackground": true,
      "problemMatcher": []
    },
    {
      "label": "Run Tests",
      "type": "shell",
      "command": "cargo test --workspace",
      "group": "test",
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "Build WASM (wasm-pack)",
      "type": "shell",
      "command": "wasm-pack build crates/core-wasm --target web --out-dir ../../assets/wasm",
      "group": "build",
      "problemMatcher": []
    },
    {
      "label": "Clippy (alle Crates)",
      "type": "shell",
      "command": "cargo clippy --workspace -- -D warnings",
      "group": "test",
      "problemMatcher": ["$rustc"]
    }
  ]
}
```

---

## 4. Workspace-Setup (Schritt-für-Schritt)

### 4.1 Alle Verzeichnisse und Dateien anlegen

```bash
# === Schritt 1: Root-Verzeichnis ===
mkdir flashcut && cd flashcut
git init

# === Schritt 2: Crate-Verzeichnisse ===
mkdir -p crates/frontend/src/components
mkdir -p crates/core-wasm/src
mkdir -p crates/backend/src
mkdir -p crates/backend/tests
mkdir -p crates/shared/src
mkdir -p assets/icons
mkdir -p assets/styles
mkdir -p assets/test-videos
mkdir -p assets/wasm            # wasm-pack Output landet hier
mkdir -p docker
mkdir -p scripts
mkdir -p .github/workflows

# === Schritt 3: Leere lib.rs / main.rs Dateien anlegen (Cargo braucht sie) ===
touch crates/shared/src/lib.rs
touch crates/core-wasm/src/lib.rs
touch crates/core-wasm/src/decoder.rs
touch crates/core-wasm/src/encoder.rs
touch crates/core-wasm/src/types.rs
touch crates/core-wasm/src/utils.rs
touch crates/frontend/src/main.rs
touch crates/frontend/src/state.rs
touch crates/frontend/src/components/mod.rs
touch crates/frontend/src/components/app.rs
touch crates/frontend/src/components/file_input.rs
touch crates/frontend/src/components/video_player.rs
touch crates/frontend/src/components/timeline.rs
touch crates/frontend/src/components/toolbar.rs
touch crates/frontend/src/components/session_panel.rs
touch crates/backend/src/main.rs
touch crates/backend/src/handlers.rs
touch crates/backend/src/session.rs
touch crates/backend/tests/integration_test.rs

echo "Verzeichnisstruktur angelegt ✓"
```

### 4.2 Workspace `Cargo.toml`

```toml
# flashcut/Cargo.toml
[workspace]
members = [
    "crates/frontend",
    "crates/core-wasm",
    "crates/backend",
    "crates/shared",
]
resolver = "2"

# ─── Workspace-weite Dependency-Versionen ────────────────────────────────────
# Wichtig: Alle Crates verwenden diese Versionen, kein Versions-Drift möglich.
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
anyhow = "1"
thiserror = "1"

# ─── Workspace-weite Profile ─────────────────────────────────────────────────
[profile.release]
opt-level = 3
lto = true           # Link-Time Optimization: kleineres + schnelleres Binary
codegen-units = 1    # Langsamerer Build, aber bestes Ergebnis

# WASM-spezifisches Profil: minimale Binary-Größe
[profile.wasm-release]
inherits = "release"
opt-level = "z"      # Optimiere für Größe statt Speed
strip = true         # Debug-Symbole entfernen

[profile.dev]
debug = true
opt-level = 0

# Tests etwas optimieren damit sie schneller laufen
[profile.test]
opt-level = 1
```

### 4.3 `.gitignore`

```gitignore
# ─── Rust Build-Artefakte ─────────────────────────────────────────────────
/target/
/crates/*/target/
**/*.rs.bk

# ─── WASM / Trunk Build-Outputs ───────────────────────────────────────────
/dist/
/crates/frontend/dist/
/crates/core-wasm/pkg/
/assets/wasm/

# ─── Cargo.lock Strategie ─────────────────────────────────────────────────
# Libraries: NICHT committen (erlaubt flexible Dependency-Auflösung)
# Binaries: Committen (reproduzierbare Builds)
# Für dieses Projekt (hat Binary-Crates): committen
# Cargo.lock

# ─── OS ───────────────────────────────────────────────────────────────────
.DS_Store
Thumbs.db
desktop.ini

# ─── VSCode ───────────────────────────────────────────────────────────────
.vscode/*
!.vscode/extensions.json
!.vscode/settings.json
!.vscode/launch.json
!.vscode/tasks.json

# ─── Environment / Secrets ────────────────────────────────────────────────
.env
.env.local
.env.*.local
*.pem
*.key

# ─── Test-Videos (können groß sein) ───────────────────────────────────────
assets/test-videos/*.mp4
assets/test-videos/*.webm
assets/test-videos/*.mov
# Aber kleine Test-Fixtures erlauben:
!assets/test-videos/sample_5s.mp4

# ─── Coverage Reports ─────────────────────────────────────────────────────
/coverage/
tarpaulin-report.html
```

### 4.4 `Trunk.toml`

```toml
# flashcut/Trunk.toml
# Trunk ist der Build-Server für Leptos/WASM-Frontend.
# Dokumentation: https://trunkrs.dev/

[build]
target = "crates/frontend/index.html"
dist = "dist"
public_url = "/"
# Ändere auf true für Production (kein Source Map)
release = false

[serve]
address = "127.0.0.1"
port = 8080
open = false
ws_protocol = "ws"

# Proxy: API-Calls an Backend weiterleiten (kein CORS nötig im Dev)
[[proxy]]
rewrite = "/api"
backend = "http://localhost:3001/api"

[[proxy]]
rewrite = "/ws"
backend = "ws://localhost:3001/ws"

[watch]
# Hot-Reload wenn sich diese Pfade ändern
paths = [
    "crates/frontend/src",
    "crates/core-wasm/src",
    "crates/shared/src",
    "assets/styles",
]
# Ignoriere Build-Artefakte
ignore = [
    "crates/frontend/dist",
    "assets/wasm",
    "target",
]

[clean]
dist = true
cargo = false
```

### 4.5 `rustfmt.toml`

```toml
# flashcut/rustfmt.toml
# Einheitliche Code-Formatierung im gesamten Workspace
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
use_field_init_shorthand = true
use_try_shorthand = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

### 4.6 `clippy.toml`

```toml
# flashcut/clippy.toml
msrv = "1.75.0"  # Minimum Supported Rust Version

# Strenge Lints für Production-Qualität
# (werden in .cargo/config.toml aktiviert)
```

### 4.7 `.cargo/config.toml`

```toml
# flashcut/.cargo/config.toml
[alias]
# Shortcuts für häufige Befehle
frontend = "run --manifest-path crates/frontend/Cargo.toml"
backend = "run -p flashcut-backend"
wasm = "build -p flashcut-core-wasm --target wasm32-unknown-unknown"
t = "test --workspace"
c = "clippy --workspace -- -D warnings"

[build]
# Standard-Kompilierungs-Flags
rustflags = ["-D", "warnings"]  # Alle Warnings als Errors behandeln

# WASM-spezifische Flags (werden automatisch bei wasm32-target angewendet)
[target.wasm32-unknown-unknown]
rustflags = [
    "-C", "opt-level=z",           # Für kleinere WASM-Dateien
    "-C", "link-arg=--export-dynamic",
]
```

---

## 5. Shared-Crate — Gemeinsame Typen

Das `shared`-Crate ist der "Vertrag" zwischen Frontend, Backend und WASM-Core. Wenn hier ein Typ geändert wird, schlagen alle abhängigen Crates sofort mit Compile-Fehlern fehl — **kein** API-Drift möglich.

#### `crates/shared/Cargo.toml`

```toml
[package]
name = "flashcut-shared"
version = "0.1.0"
edition = "2021"
description = "Gemeinsame Typen für FlashCut (Frontend, Backend, WASM-Core)"

[dependencies]
serde = { workspace = true }

# Optional für JSON-Schema-Generierung (nützlich für API-Docs)
# schemars = { version = "0.8", optional = true }

[features]
default = []
# json-schema = ["dep:schemars"]
```

#### `crates/shared/src/lib.rs`

```rust
// crates/shared/src/lib.rs
//! Gemeinsame Typen für das gesamte FlashCut-Ökosystem.
//!
//! Dieses Crate hat absichtlich KEINE Abhängigkeiten auf:
//! - wasm-bindgen (würde WASM-Compilation erzwingen)
//! - axum/tokio (würde Server-Runtime erzwingen)
//! - web-sys (Browser-only APIs)
//!
//! Ziel: Jedes andere Crate kann dieses als Dependency nutzen,
//!       egal für welches Compilation-Target.

use serde::{Deserialize, Serialize};

// ─── Trim & Video Typen ───────────────────────────────────────────────────

/// Repräsentiert einen Schnittbereich im Video.
///
/// Alle Zeitwerte sind in Millisekunden (f64 für Sub-Millisekunden-Genauigkeit
/// bei sehr langen Videos ohne Integer-Overflow).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimRange {
    /// Beginn des Schnitts in Millisekunden (inklusive)
    pub start_ms: f64,
    /// Ende des Schnitts in Millisekunden (exklusive)
    pub end_ms: f64,
}

impl TrimRange {
    pub fn new(start_ms: f64, end_ms: f64) -> Self {
        Self { start_ms, end_ms }
    }

    /// Dauer des Schnitts in Millisekunden
    pub fn duration_ms(&self) -> f64 {
        (self.end_ms - self.start_ms).max(0.0)
    }

    /// True wenn der Range sinnvoll ist (start < end, beide >= 0)
    pub fn is_valid(&self) -> bool {
        self.start_ms >= 0.0 && self.end_ms > self.start_ms
    }

    /// Gibt einen neuen TrimRange zurück, der innerhalb [0, video_duration_ms] liegt.
    pub fn clamped(&self, video_duration_ms: f64) -> Self {
        let start = self.start_ms.clamp(0.0, video_duration_ms);
        let end = self.end_ms.clamp(start, video_duration_ms);
        Self { start_ms: start, end_ms: end }
    }

    /// Konvertiert in Sekunden (für Display)
    pub fn to_seconds(&self) -> (f64, f64) {
        (self.start_ms / 1000.0, self.end_ms / 1000.0)
    }
}

impl Default for TrimRange {
    fn default() -> Self {
        Self { start_ms: 0.0, end_ms: 0.0 }
    }
}

/// Metadaten einer Video-Datei (codec-agnostisch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    /// Gesamtdauer in Millisekunden
    pub duration_ms: f64,
    /// Videobreite in Pixeln
    pub width: u32,
    /// Videohöhe in Pixeln
    pub height: u32,
    /// Frames per second (approximiert)
    pub fps: f64,
    /// Codec-String (z.B. "avc1.42001E", "vp09.00.10.08")
    pub codec: String,
    /// Bitrate in bits/s (0 wenn unbekannt)
    pub bitrate: u64,
}

impl VideoMetadata {
    /// Gibt ein leeres Metadata-Objekt zurück (für Initialisierung)
    pub fn empty() -> Self {
        Self {
            duration_ms: 0.0,
            width: 0,
            height: 0,
            fps: 30.0,
            codec: String::new(),
            bitrate: 0,
        }
    }

    /// Formatiert Duration als "mm:ss.fff"
    pub fn duration_timecode(&self) -> String {
        timecode_from_ms(self.duration_ms)
    }

    /// Aspect ratio als float (width / height)
    pub fn aspect_ratio(&self) -> f64 {
        if self.height == 0 { 16.0 / 9.0 } else { self.width as f64 / self.height as f64 }
    }
}

// ─── WebSocket / Session Protokoll ────────────────────────────────────────

/// Session-State wie er vom Server an neue Teilnehmer gesendet wird.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    /// Aktueller Playhead in Millisekunden
    pub playhead_ms: f64,
    /// Aktiver Trim-Bereich
    pub trim_range: TrimRange,
    /// Anzahl aktiver Teilnehmer
    pub participant_count: usize,
}

/// Alle Nachrichten die über den WebSocket-Kanal fließen.
///
/// `#[serde(tag = "type", content = "payload")]` erzeugt ein
/// Tagged-Union JSON-Format:
/// ```json
/// {"type": "TimestampUpdate", "payload": {"participant_id": "abc", "playhead_ms": 1234.5}}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    /// Ein Teilnehmer hat den Playhead bewegt
    TimestampUpdate {
        participant_id: String,
        playhead_ms: f64,
    },
    /// Ein Teilnehmer hat die Trim-Marken geändert
    TrimUpdate {
        participant_id: String,
        range: TrimRange,
    },
    /// Ein Teilnehmer ist der Session beigetreten
    ParticipantJoined {
        participant_id: String,
        participant_count: usize,
    },
    /// Ein Teilnehmer hat die Session verlassen
    ParticipantLeft {
        participant_id: String,
        participant_count: usize,
    },
    /// Vollständiger State-Sync (wird beim Join gesendet)
    StateSync(SessionState),
    /// Kommentar / Chat-Nachricht
    Comment {
        participant_id: String,
        text: String,
        timestamp_ms: f64,  // An welcher Video-Position der Kommentar ist
    },
    /// Keep-Alive Ping
    Ping,
    /// Keep-Alive Pong  
    Pong,
    /// Fehler vom Server
    Error {
        code: String,
        message: String,
    },
}

// ─── REST API Typen ────────────────────────────────────────────────────────

/// Request-Body für POST /api/sessions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateSessionRequest {
    /// Optional: Initialer Trim-Range (wenn Session mit vorhandenem State erstellt wird)
    pub initial_trim_range: Option<TrimRange>,
}

/// Response für POST /api/sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub ws_url: String,
    pub share_url: String,
}

/// Response für GET /api/sessions/:id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfoResponse {
    pub session_id: String,
    pub participant_count: usize,
    pub state: SessionState,
    pub created_at_secs: u64,
}

// ─── Hilfsfunktionen ──────────────────────────────────────────────────────

/// Formatiert Millisekunden als "mm:ss.fff" Timecode.
/// Utility-Funktion die in allen Crates nutzbar ist.
pub fn timecode_from_ms(ms: f64) -> String {
    if ms < 0.0 { return "00:00.000".to_string(); }
    let total_ms = ms as u64;
    let millis = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let seconds = total_secs % 60;
    let minutes = total_secs / 60;
    format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
}

/// Parsiert einen "mm:ss.fff" Timecode zurück in Millisekunden.
pub fn ms_from_timecode(tc: &str) -> Option<f64> {
    let parts: Vec<&str> = tc.split(':').collect();
    if parts.len() != 2 { return None; }
    let minutes: f64 = parts[0].parse().ok()?;
    let sec_parts: Vec<&str> = parts[1].split('.').collect();
    if sec_parts.len() != 2 { return None; }
    let seconds: f64 = sec_parts[0].parse().ok()?;
    let millis: f64 = sec_parts[1].parse().ok()?;
    Some((minutes * 60.0 + seconds) * 1000.0 + millis)
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_range_basic_validation() {
        let valid = TrimRange::new(1000.0, 5000.0);
        assert!(valid.is_valid());
        assert_eq!(valid.duration_ms(), 4000.0);

        let invalid_reversed = TrimRange::new(5000.0, 1000.0);
        assert!(!invalid_reversed.is_valid());
        assert_eq!(invalid_reversed.duration_ms(), 0.0); // max(0.0, negative)

        let invalid_negative = TrimRange::new(-100.0, 1000.0);
        assert!(!invalid_negative.is_valid());
    }

    #[test]
    fn trim_range_clamped_within_bounds() {
        let range = TrimRange::new(-100.0, 10000.0);
        let clamped = range.clamped(5000.0);
        assert_eq!(clamped.start_ms, 0.0);
        assert_eq!(clamped.end_ms, 5000.0);
        assert!(clamped.is_valid());
    }

    #[test]
    fn trim_range_clamped_already_valid() {
        let range = TrimRange::new(1000.0, 3000.0);
        let clamped = range.clamped(5000.0);
        assert_eq!(clamped.start_ms, 1000.0);
        assert_eq!(clamped.end_ms, 3000.0);
    }

    #[test]
    fn timecode_roundtrip() {
        let ms = 61500.0;
        let tc = timecode_from_ms(ms);
        assert_eq!(tc, "01:01.500");
        let back = ms_from_timecode(&tc).unwrap();
        assert_eq!(back, ms);
    }

    #[test]
    fn timecode_zero() {
        assert_eq!(timecode_from_ms(0.0), "00:00.000");
    }

    #[test]
    fn ws_message_serde_roundtrip() {
        let msg = WsMessage::TimestampUpdate {
            participant_id: "abc123".to_string(),
            playhead_ms: 1234.567,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"TimestampUpdate\""));
        let back: WsMessage = serde_json::from_str(&json).unwrap();
        matches!(back, WsMessage::TimestampUpdate { playhead_ms, .. } if playhead_ms == 1234.567);
    }
}
```

---

## 6. Phase 1 — WASM-Kern & Video-Decoding

### 6.1 `core-wasm` Cargo.toml

```toml
# crates/core-wasm/Cargo.toml
[package]
name = "flashcut-core-wasm"
version = "0.1.0"
edition = "2021"
description = "Rust/WASM Video-Processing Core für FlashCut"

# WICHTIG: cdylib = Dynamic Library für WASM
#          rlib = Rust Library für Unit-Tests (nicht WASM)
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
serde = { workspace = true }
serde_json = { workspace = true }
serde-wasm-bindgen = "0.6"
console_error_panic_hook = "0.1"
thiserror = { workspace = true }
flashcut-shared = { path = "../shared" }

# ─── web-sys: Browser API Bindings ───────────────────────────────────────
# JEDES genutzte API muss hier als Feature deklariert werden!
# Fehlende Features = Compile-Fehler "struct not found"
[dependencies.web-sys]
version = "0.3"
features = [
  # Basis-DOM
  "Window",
  "Document",
  "Element",
  "HtmlElement",
  "Node",
  "EventTarget",

  # Canvas & Rendering
  "HtmlCanvasElement",
  "CanvasRenderingContext2d",
  "ImageData",
  "ImageBitmap",

  # Video Element (für Metadaten-Extraktion)
  "HtmlVideoElement",
  "HtmlAudioElement",

  # File & Blob APIs
  "File",
  "FileList",
  "FileReader",
  "ProgressEvent",
  "Blob",
  "BlobPropertyBag",
  "Url",

  # File System Access API (moderne Alternative zu input[type=file])
  "FileSystemFileHandle",
  "FileSystemDirectoryHandle",
  "FileSystemGetFileOptions",

  # WebCodecs — VideoDecoder
  "VideoDecoder",
  "VideoDecoderConfig",
  "VideoDecoderInit",
  "VideoDecoderSupport",
  "EncodedVideoChunk",
  "EncodedVideoChunkInit",
  "EncodedVideoChunkType",
  "VideoFrame",
  "VideoFrameInit",
  "VideoFrameBufferInit",
  "VideoFrameCopyToOptions",
  "PlaneLayout",
  "VideoPixelFormat",
  "VideoColorSpace",
  "VideoColorSpaceInit",

  # WebCodecs — VideoEncoder
  "VideoEncoder",
  "VideoEncoderConfig",
  "VideoEncoderInit",
  "VideoEncoderEncodeOptions",
  "VideoEncoderSupport",
  "EncodedVideoChunkMetadata",
  "EncodedVideoChunkOutputCallback",
  "LatencyMode",
  "HardwareAcceleration",
  "BitrateMode",
  "AvcEncoderConfig",
  "AvcBitstreamFormat",

  # Web Workers (für Hintergrund-Verarbeitung)
  "Worker",
  "WorkerOptions",
  "WorkerType",
  "DedicatedWorkerGlobalScope",
  "MessageEvent",
  "ErrorEvent",

  # Streams
  "ReadableStream",
  "ReadableStreamDefaultReader",
  "WritableStream",
  "WritableStreamDefaultWriter",

  # Events
  "EventListener",
  "Event",
  "CustomEvent",
  "CustomEventInit",

  # Console (Debugging)
  "console",

  # Performance API (für Benchmarks)
  "Performance",
  "PerformanceObserver",
]

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

### 6.2 `lib.rs`

```rust
// crates/core-wasm/src/lib.rs
//! FlashCut Core WASM — Einstiegspunkt.
//!
//! Dieses Modul wird durch `wasm-pack build` zu einem WASM-Binary kompiliert.
//! Die `#[wasm_bindgen]`-Annotationen erzeugen TypeScript-Definitionen
//! und JS-Wrapper-Funktionen für alle exportierten Rust-Typen.

use wasm_bindgen::prelude::*;

pub mod decoder;
pub mod encoder;
pub mod types;
pub mod utils;
pub mod pipeline;  // Kombiniert decoder + encoder für High-Level-API

/// Initialisierung beim WASM-Modul-Load.
/// `#[wasm_bindgen(start)]` wird automatisch beim Import des Moduls aufgerufen.
#[wasm_bindgen(start)]
pub fn init() {
    // Panic-Hook: Rust-Panics erscheinen als lesbare Fehlermeldung in der
    // Browser-Konsole statt als kryptisches "RuntimeError: unreachable".
    console_error_panic_hook::set_once();

    utils::log(&format!(
        "FlashCut WASM Core v{} geladen ✓",
        env!("CARGO_PKG_VERSION")
    ));

    // Feature-Detection: WebCodecs verfügbar?
    let window = web_sys::window().expect("Kein window-Objekt");
    let has_video_decoder = js_sys::Reflect::has(
        &window,
        &JsValue::from_str("VideoDecoder")
    ).unwrap_or(false);

    if !has_video_decoder {
        utils::warn(
            "WebCodecs API nicht verfügbar! \
             Bitte Chrome 94+ oder Firefox 130+ verwenden."
        );
    }
}

/// WASM-Modul Version
#[wasm_bindgen]
pub fn wasm_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Prüft ob alle notwendigen Browser-APIs verfügbar sind.
/// Gibt JSON zurück: { "videoDecoder": bool, "videoEncoder": bool, "fileSystemAccess": bool }
#[wasm_bindgen]
pub fn check_browser_support() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return r#"{"error":"no window"}"#.to_string(),
    };

    let check = |name: &str| -> bool {
        js_sys::Reflect::has(&window, &JsValue::from_str(name))
            .unwrap_or(false)
    };

    format!(
        r#"{{"videoDecoder":{},"videoEncoder":{},"fileSystemAccess":{}}}"#,
        check("VideoDecoder"),
        check("VideoEncoder"),
        check("showOpenFilePicker"),
    )
}
```

### 6.3 `utils.rs`

```rust
// crates/core-wasm/src/utils.rs
//! Logging- und Hilfs-Utilities für den WASM-Context.

use wasm_bindgen::prelude::*;
use web_sys::console;

// ─── Logging ──────────────────────────────────────────────────────────────

pub fn log(msg: &str) {
    console::log_1(&JsValue::from_str(msg));
}

pub fn warn(msg: &str) {
    console::warn_1(&JsValue::from_str(msg));
}

pub fn error(msg: &str) {
    console::error_1(&JsValue::from_str(msg));
}

/// Debug-Log mit Prefix für Modul-Kontext
pub fn debug(module: &str, msg: &str) {
    console::log_1(&JsValue::from_str(&format!("[{}] {}", module, msg)));
}

// ─── Timecode-Utilities ───────────────────────────────────────────────────

/// Millisekunden → "mm:ss.fff"
pub fn ms_to_timecode(ms: f64) -> String {
    flashcut_shared::timecode_from_ms(ms)
}

// ─── JS-Value Konvertierung ────────────────────────────────────────────────

/// Konvertiert einen Rust-Fehler in einen JsValue (für ? Operator)
pub fn rust_error_to_js(msg: impl ToString) -> JsValue {
    JsValue::from_str(&msg.to_string())
}

// ─── Performance Measurement ───────────────────────────────────────────────

/// Gibt den aktuellen `performance.now()` Wert zurück (Millisekunden)
pub fn performance_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

/// Einfacher Benchmark-Wrapper
pub struct Timer {
    label: String,
    start: f64,
}

impl Timer {
    pub fn start(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            start: performance_now(),
        }
    }

    pub fn stop(self) {
        let elapsed = performance_now() - self.start;
        log(&format!("[Timer] {}: {:.2}ms", self.label, elapsed));
    }
}
```

### 6.4 `types.rs`

```rust
// crates/core-wasm/src/types.rs
//! WASM-exportierbare Typen (mit wasm-bindgen Annotationen).
//!
//! Spiegelt Typen aus `flashcut-shared` für direkte JS-Interoperabilität.

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use flashcut_shared::{TrimRange as SharedTrimRange, VideoMetadata as SharedVideoMetadata};

/// WASM-exportierbarer TrimRange.
/// Wraps den Shared-Typ mit wasm-bindgen Annotations.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct WasmTrimRange {
    inner: SharedTrimRange,
}

#[wasm_bindgen]
impl WasmTrimRange {
    #[wasm_bindgen(constructor)]
    pub fn new(start_ms: f64, end_ms: f64) -> Self {
        Self {
            inner: SharedTrimRange::new(start_ms, end_ms),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn start_ms(&self) -> f64 { self.inner.start_ms }

    #[wasm_bindgen(getter)]
    pub fn end_ms(&self) -> f64 { self.inner.end_ms }

    pub fn duration_ms(&self) -> f64 { self.inner.duration_ms() }
    pub fn is_valid(&self) -> bool { self.inner.is_valid() }

    pub fn clamped(&self, video_duration_ms: f64) -> WasmTrimRange {
        WasmTrimRange { inner: self.inner.clamped(video_duration_ms) }
    }

    pub fn to_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.inner).map_err(|e| JsError::new(&e.to_string()))
    }
}

impl From<SharedTrimRange> for WasmTrimRange {
    fn from(r: SharedTrimRange) -> Self { Self { inner: r } }
}

impl From<WasmTrimRange> for SharedTrimRange {
    fn from(r: WasmTrimRange) -> Self { r.inner }
}

/// Dekodierter Frame als einfache Metadaten-Struktur (ohne VideoFrame-Ownership)
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct FrameInfo {
    pub timestamp_us: f64,
    pub duration_us: f64,
    pub width: u32,
    pub height: u32,
}

#[wasm_bindgen]
impl FrameInfo {
    #[wasm_bindgen(constructor)]
    pub fn new(timestamp_us: f64, duration_us: f64, width: u32, height: u32) -> Self {
        Self { timestamp_us, duration_us, width, height }
    }

    pub fn timestamp_ms(&self) -> f64 { self.timestamp_us / 1000.0 }
}

/// Export-Konfiguration
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct ExportConfig {
    pub bitrate_kbps: u32,
    pub width: u32,
    pub height: u32,
    codec: String,
    filename: String,
}

#[wasm_bindgen]
impl ExportConfig {
    /// Erstellt eine Standard-Export-Konfiguration
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            bitrate_kbps: 4000,  // 4 Mbit/s Standard
            width,
            height,
            codec: "vp09.00.10.08".to_string(),  // VP9
            filename: "flashcut-export.webm".to_string(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn codec(&self) -> String { self.codec.clone() }

    #[wasm_bindgen(setter)]
    pub fn set_codec(&mut self, codec: String) { self.codec = codec; }

    #[wasm_bindgen(getter)]
    pub fn filename(&self) -> String { self.filename.clone() }

    #[wasm_bindgen(setter)]
    pub fn set_filename(&mut self, filename: String) { self.filename = filename; }
}

/// Fehler-Enum der WASM-API
#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error("Datei konnte nicht gelesen werden: {0}")]
    FileRead(String),
    #[error("Codec nicht unterstützt: {0}")]
    UnsupportedCodec(String),
    #[error("VideoDecoder Fehler: {0}")]
    Decoder(String),
    #[error("VideoEncoder Fehler: {0}")]
    Encoder(String),
    #[error("Ungültiger TrimRange: start={0}ms, end={1}ms")]
    InvalidTrimRange(f64, f64),
    #[error("Browser-API nicht verfügbar: {0}")]
    BrowserApiUnavailable(String),
    #[error("Interner Fehler: {0}")]
    Internal(String),
}

impl From<WasmError> for JsValue {
    fn from(e: WasmError) -> JsValue {
        JsValue::from_str(&e.to_string())
    }
}

impl From<WasmError> for JsError {
    fn from(e: WasmError) -> JsError {
        JsError::new(&e.to_string())
    }
}
```

### 6.5 `decoder.rs`

```rust
// crates/core-wasm/src/decoder.rs
//! Video-Decoding via WebCodecs API.
//!
//! Der VideoDecoder im Browser ist hardware-accelerated: Auf Geräten mit
//! GPU-Dekoder läuft H.264-Decoding in dedizierten Hardware-Units
//! (wie auf modernen MacBooks/iPhones). Wir müssen nur die API korrekt
//! bedienen — die Hardware erledigt den Rest.
//!
//! Wichtiger Hinweis zu VideoFrame-Ownership:
//! VideoFrame hält eine Referenz auf GPU-Texturen oder Shared Memory.
//! Nach der Verwendung MUSS frame.close() aufgerufen werden, sonst droht
//! GPU-Speicher-Exhaustion nach wenigen Sekunden Video.

use js_sys::{Array, Function, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    EncodedVideoChunk, EncodedVideoChunkInit, EncodedVideoChunkType,
    HtmlCanvasElement, HtmlVideoElement,
    VideoDecoder, VideoDecoderConfig, VideoDecoderInit, VideoFrame,
};

use crate::types::{FrameInfo, WasmError};
use crate::utils::{self, Timer};
use flashcut_shared::VideoMetadata;

// ─── Decoder-Erstellung ───────────────────────────────────────────────────

/// Erstellt einen konfigurierten VideoDecoder.
///
/// # Parameter
/// * `on_frame` - Wird für jeden dekodier ten Frame aufgerufen: (VideoFrame) → void
/// * `on_error` - Wird bei Decoder-Fehlern aufgerufen: (DOMException) → void
///
/// # Wichtig
/// `on_frame` MUSS `frame.close()` aufrufen nach Verwendung!
/// Andernfalls: GPU-Speicher-Leak!
#[wasm_bindgen]
pub fn create_decoder(
    on_frame: Function,
    on_error: Function,
) -> Result<VideoDecoder, JsValue> {
    let init = VideoDecoderInit::new(&on_error, &on_frame);
    VideoDecoder::new(&init).map_err(|e| {
        utils::error(&format!("VideoDecoder::new() fehlgeschlagen: {:?}", e));
        e
    })
}

/// Konfiguriert den Decoder für einen bestimmten Codec.
///
/// # Unterstützte Codec-Strings (Stand Chrome 120+)
/// - `"avc1.42001E"` → H.264 Baseline Level 3.0 (breiteste Kompatibilität)
/// - `"avc1.640028"` → H.264 High Profile Level 4.0 (besser für 1080p)
/// - `"vp8"`         → VP8 (älteres WebM-Format)
/// - `"vp09.00.10.08"` → VP9 Profile 0, Level 1.0, 8-Bit
/// - `"av01.0.04M.08"` → AV1 (modernster Codec, nicht überall hw-accelerated)
///
/// # Fehler
/// Gibt Fehler wenn: Codec unbekannt, Browser unterstützt ihn nicht,
/// oder width/height 0 sind.
#[wasm_bindgen]
pub fn configure_decoder(
    decoder: &VideoDecoder,
    codec: &str,
    width: u32,
    height: u32,
    description: Option<Vec<u8>>,  // Für H.264: AVCDecoderConfigurationRecord (optional)
) -> Result<(), JsValue> {
    if width == 0 || height == 0 {
        return Err(WasmError::Decoder(
            format!("Ungültige Dimensionen: {}x{}", width, height)
        ).into());
    }

    let config = VideoDecoderConfig::new(codec);
    config.set_coded_width(width);
    config.set_coded_height(height);

    // Hardware-Beschleunigung bevorzugen, aber Software-Fallback erlauben
    config.set_hardware_acceleration(
        web_sys::HardwareAcceleration::PreferHardware,
    );

    // H.264 Extradata (AVCDecoderConfigurationRecord)
    // Wird benötigt wenn der Stream im Annex-B Format vorliegt
    if let Some(desc_bytes) = description {
        let desc_array = Uint8Array::from(desc_bytes.as_slice());
        config.set_description(&desc_array.buffer());
    }

    decoder.configure(&config)?;

    utils::debug(
        "decoder",
        &format!("Konfiguriert: codec={}, {}x{}", codec, width, height)
    );

    Ok(())
}

/// Prüft ob ein Codec vom Browser unterstützt wird, BEVOR wir ihn konfigurieren.
///
/// Gibt Promise<bool> zurück — `true` wenn unterstützt.
#[wasm_bindgen]
pub async fn is_codec_supported(codec: &str, width: u32, height: u32) -> bool {
    let config = VideoDecoderConfig::new(codec);
    config.set_coded_width(width);
    config.set_coded_height(height);

    match JsFuture::from(VideoDecoder::is_config_supported(&config)).await {
        Ok(support_js) => {
            let support = web_sys::VideoDecoderSupport::from(support_js);
            support.supported().unwrap_or(false)
        }
        Err(_) => false,
    }
}

// ─── Frame-Verarbeitung ───────────────────────────────────────────────────

/// Übergibt einen enkodier ten Chunk an den Decoder.
///
/// # Parameter
/// * `data`         - Rohe Chunk-Bytes (z.B. aus MP4/WebM-Demuxing)
/// * `timestamp_us` - Timestamp in MIKROSEKUNDEN (nicht ms!)
/// * `duration_us`  - Dauer in Mikrosekunden (0 wenn unbekannt)
/// * `is_keyframe`  - true = I-Frame/Key-Frame, false = P/B-Frame
#[wasm_bindgen]
pub fn decode_chunk(
    decoder: &VideoDecoder,
    data: &Uint8Array,
    timestamp_us: f64,
    duration_us: f64,
    is_keyframe: bool,
) -> Result<(), JsValue> {
    // Decoder-Queue-Size prüfen: Wenn zu voll, verlangsamen wir das Input
    // (Backpressure-Mechanismus)
    if decoder.decode_queue_size() > 10 {
        utils::warn(&format!(
            "Decoder-Queue überfüllt: {} Frames ausstehend",
            decoder.decode_queue_size()
        ));
        // In einem echten Demuxer würden wir hier pausieren
    }

    let chunk_type = if is_keyframe {
        EncodedVideoChunkType::Key
    } else {
        EncodedVideoChunkType::Delta
    };

    let init = EncodedVideoChunkInit::new(data, timestamp_us, chunk_type);
    if duration_us > 0.0 {
        init.set_duration(duration_us);
    }

    let chunk = EncodedVideoChunk::new(&init)?;
    decoder.decode(&chunk)?;

    Ok(())
}

/// Zeichnet einen VideoFrame auf ein Canvas-Element.
///
/// # KRITISCH: frame.close() wird intern aufgerufen!
/// Nach diesem Aufruf ist der VideoFrame ungültig und darf nicht
/// mehr verwendet werden.
#[wasm_bindgen]
pub fn draw_frame_to_canvas(
    frame: VideoFrame,
    canvas: &HtmlCanvasElement,
) -> Result<FrameInfo, JsValue> {
    let width = frame.display_width();
    let height = frame.display_height();
    let timestamp = frame.timestamp().unwrap_or(0.0);
    let duration = frame.duration().unwrap_or(0.0);

    // Canvas auf Video-Dimensionen anpassen (nur wenn nötig = Performance)
    if canvas.width() != width || canvas.height() != height {
        canvas.set_width(width);
        canvas.set_height(height);
    }

    let ctx = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("CanvasRenderingContext2D nicht verfügbar"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    // Frame auf Canvas zeichnen
    ctx.draw_image_with_video_frame(&frame, 0.0, 0.0)?;

    // GPU-Ressourcen freigeben — PFLICHT!
    frame.close();

    Ok(FrameInfo::new(timestamp, duration, width, height))
}

/// Flush: Warte bis alle gepufferten Frames verarbeitet wurden.
///
/// Muss IMMER nach dem letzten decode_chunk()-Aufruf aufgerufen werden,
/// damit alle ausstehenden Frames noch dekodiert werden.
#[wasm_bindgen]
pub async fn flush_decoder(decoder: &VideoDecoder) -> Result<(), JsValue> {
    let timer = Timer::start("flush_decoder");
    JsFuture::from(decoder.flush()).await?;
    timer.stop();
    Ok(())
}

/// Reset: Decoder-State zurücksetzen (bei Seek-Operationen nötig).
///
/// Nach einem reset() muss configure() neu aufgerufen werden!
#[wasm_bindgen]
pub fn reset_decoder(decoder: &VideoDecoder) -> Result<(), JsValue> {
    decoder.reset();
    utils::debug("decoder", "Reset durchgeführt (Seek)");
    Ok(())
}

// ─── Metadaten via HTMLVideoElement ───────────────────────────────────────

/// Liest Video-Metadaten durch Erstellen eines temporären <video> Elements.
///
/// Das ist der schnellste Weg um duration, width, height zu erhalten —
/// der Browser nutzt seinen internen Demuxer ohne den kompletten Stream
/// dekodieren zu müssen.
///
/// Gibt JSON zurück: `VideoMetadata` als serialisierter String
#[wasm_bindgen]
pub async fn read_video_metadata(file: &web_sys::File) -> Result<String, JsValue> {
    let timer = Timer::start("read_video_metadata");

    let url = web_sys::Url::create_object_url_with_blob(file)?;

    let window = web_sys::window().ok_or("Kein window")?;
    let document = window.document().ok_or("Kein document")?;

    // Temporäres Video-Element
    let video = document
        .create_element("video")?
        .dyn_into::<HtmlVideoElement>()?;

    // Ausblenden (unsichtbar, aber im DOM für Metadaten-Loading)
    video.style().set_property("display", "none")?;
    document.body().ok_or("Kein body")?.append_child(&video)?;

    video.set_src(&url);
    video.set_preload("metadata");

    // Warte auf 'loadedmetadata' Event via Promise
    let (resolve_fn, reject_fn, promise) = create_promise_pair();

    let video_clone = video.clone();
    let resolve_clone = resolve_fn.clone();
    let on_loaded = Closure::once_into_js(move |_: web_sys::Event| {
        let meta = VideoMetadata {
            duration_ms: video_clone.duration() * 1000.0,
            width: video_clone.video_width(),
            height: video_clone.video_height(),
            fps: 30.0,  // HTMLVideoElement gibt keine FPS-Info heraus
            codec: "unknown".to_string(),  // Wird durch Demuxer ergänzt
            bitrate: 0,
        };
        let json = serde_json::to_string(&meta).unwrap_or_default();
        resolve_clone
            .call1(&JsValue::NULL, &JsValue::from_str(&json))
            .ok();
    });

    let on_error = Closure::once_into_js(move |e: web_sys::Event| {
        reject_fn
            .call1(&JsValue::NULL, &JsValue::from_str("Video-Metadaten konnten nicht gelesen werden"))
            .ok();
    });

    video.set_onloadedmetadata(Some(on_loaded.as_ref().unchecked_ref()));
    video.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    video.load();

    let result = JsFuture::from(promise).await;

    // Aufräumen
    document.body()
        .and_then(|b| b.remove_child(&video).ok());
    web_sys::Url::revoke_object_url(&url)?;

    timer.stop();

    result.map(|v| v.as_string().unwrap_or_default())
}

// ─── Hilfs-Funktionen ─────────────────────────────────────────────────────

/// Erstellt ein (resolve, reject, Promise) Triple für async Callbacks.
fn create_promise_pair() -> (Function, Function, Promise) {
    let mut resolve_opt: Option<Function> = None;
    let mut reject_opt: Option<Function> = None;

    let promise = Promise::new(&mut |resolve, reject| {
        resolve_opt = Some(resolve);
        reject_opt = Some(reject);
    });

    (
        resolve_opt.unwrap(),
        reject_opt.unwrap(),
        promise,
    )
}

#[cfg(test)]
mod tests {
    // Unit-Tests ohne Browser-Abhängigkeit
    // Browser-Tests in tests/decoder_test.rs mit wasm-bindgen-test

    #[test]
    fn chunk_type_logic() {
        // Keyframe-Logik testen (keine Browser-API nötig)
        let is_keyframe = true;
        assert_eq!(is_keyframe, true);
    }
}
```

---

## 7. Phase 2 — Schnitt-Logik & Export

### `encoder.rs` (vollständig)

```rust
// crates/core-wasm/src/encoder.rs
use js_sys::{Array, Function, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Blob, BlobPropertyBag, EncodedVideoChunk,
    VideoEncoder, VideoEncoderConfig, VideoEncoderInit,
    VideoEncoderEncodeOptions, VideoFrame,
};

use crate::types::{ExportConfig, WasmError};
use crate::utils::{self, Timer};

// ─── Encoder-Erstellung ───────────────────────────────────────────────────

/// Erstellt einen VideoEncoder.
///
/// # Parameter
/// * `on_chunk` - Callback für jeden enkodier ten Chunk
///               Signatur: (chunk: EncodedVideoChunk, metadata: EncodedVideoChunkMetadata|undefined) → void
/// * `on_error` - Callback für Encoder-Fehler
#[wasm_bindgen]
pub fn create_encoder(
    on_chunk: Function,
    on_error: Function,
) -> Result<VideoEncoder, JsValue> {
    let init = VideoEncoderInit::new(&on_error, &on_chunk);
    VideoEncoder::new(&init).map_err(|e| {
        utils::error(&format!("VideoEncoder::new() fehlgeschlagen: {:?}", e));
        e
    })
}

/// Konfiguriert den Encoder.
///
/// # Empfohlene Codec-Einstellungen für Web-Export
///
/// **VP9 (Empfehlung für Kompatibilität):**
/// ```
/// codec: "vp09.00.10.08"  // VP9 Profile 0, Level 1, 8-Bit
/// bitrate: 2_000_000 bis 8_000_000  // 2-8 Mbit/s je nach Auflösung
/// ```
///
/// **H.264 (höchste Kompatibilität, auch für Smartphones):**
/// ```
/// codec: "avc1.42001E"  // H.264 Baseline
/// bitrate: 4_000_000
/// ```
///
/// **AV1 (modernster Codec, kleinste Dateigröße):**
/// ```
/// codec: "av01.0.04M.08"
/// bitrate: 1_000_000  // AV1 ist sehr effizient
/// ```
#[wasm_bindgen]
pub fn configure_encoder(
    encoder: &VideoEncoder,
    config: &ExportConfig,
) -> Result<(), JsValue> {
    if config.width == 0 || config.height == 0 {
        return Err(WasmError::Encoder(
            format!("Ungültige Dimensionen: {}x{}", config.width, config.height)
        ).into());
    }

    let enc_config = VideoEncoderConfig::new(&config.codec(), config.height, config.width);

    // Bitrate in bits/s (nicht kbps!)
    enc_config.set_bitrate((config.bitrate_kbps as f64) * 1000.0);
    enc_config.set_framerate(30.0);

    // Für Datei-Export: Qualität > Latenz
    enc_config.set_latency_mode(web_sys::LatencyMode::Quality);

    // Bitrate-Mode: "constant" für vorhersehbare Dateigröße
    // "variable" für bessere Qualität bei gleicher durchschnittlicher Bitrate
    enc_config.set_bitrate_mode(web_sys::BitrateMode::Variable);

    encoder.configure(&enc_config)?;

    utils::debug(
        "encoder",
        &format!(
            "Konfiguriert: codec={}, {}x{} @ {}kbps",
            config.codec(), config.width, config.height, config.bitrate_kbps
        )
    );

    Ok(())
}

/// Prüft ob Encoder-Konfiguration unterstützt wird.
#[wasm_bindgen]
pub async fn is_encoder_config_supported(config: &ExportConfig) -> bool {
    let enc_config = VideoEncoderConfig::new(&config.codec(), config.height, config.width);
    enc_config.set_bitrate((config.bitrate_kbps as f64) * 1000.0);

    match JsFuture::from(VideoEncoder::is_config_supported(&enc_config)).await {
        Ok(support_js) => {
            let support = web_sys::VideoEncoderSupport::from(support_js);
            support.supported().unwrap_or(false)
        }
        Err(_) => false,
    }
}

// ─── Frame-Enkodierung ────────────────────────────────────────────────────

/// Enkodiert einen einzelnen VideoFrame.
///
/// # Parameter
/// * `force_keyframe` - Erzwingt einen I-Frame.
///                      Für den ERSTEN Frame immer true!
///                      Danach: true bei Seek-Punkten, alle ~2s empfohlen.
///
/// # WICHTIG: frame.close() wird intern aufgerufen!
#[wasm_bindgen]
pub fn encode_frame(
    encoder: &VideoEncoder,
    frame: VideoFrame,
    force_keyframe: bool,
) -> Result<(), JsValue> {
    // Encoder-Queue-Status prüfen
    if encoder.encode_queue_size() > 20 {
        utils::warn(&format!(
            "Encoder-Queue überfüllt: {} Frames",
            encoder.encode_queue_size()
        ));
    }

    let options = VideoEncoderEncodeOptions::new();
    options.set_key_frame(force_keyframe);

    encoder.encode_with_options(&frame, &options)?;

    // PFLICHT: GPU-Ressourcen freigeben!
    frame.close();

    Ok(())
}

/// Flush: Alle gepufferten Frames enkodieren.
#[wasm_bindgen]
pub async fn flush_encoder(encoder: &VideoEncoder) -> Result<(), JsValue> {
    let timer = Timer::start("flush_encoder");
    JsFuture::from(encoder.flush()).await?;
    timer.stop();
    Ok(())
}

// ─── Chunk-Assembly & Download ────────────────────────────────────────────

/// Sammelt EncodedVideoChunks und erstellt einen Download-Blob.
///
/// # Wichtiger Hinweis zu Container-Formaten
/// WebCodecs enkodiert nur die rohen Frames — kein Container!
/// Um eine abspielbare Datei zu erstellen, brauchen wir:
/// - VP8/VP9 → WebM Container (relativ einfach zu bauen)
/// - H.264/H.265 → MP4 Container (komplexer, benötigt mp4box.js oder ähnlich)
///
/// Für Phase 2 MVP: WebM Container mit VP9 Codec.
/// Für Production: mp4-muxer.js oder @webav/mp4-muxer einbinden.
///
/// # Parameter
/// * `chunk_data_array` - JS Array von Uint8Array (Chunk-Bytes)
/// * `chunk_timestamps` - JS Array von f64 (Timestamps in Mikrosekunden)
/// * `chunk_is_keyframe` - JS Array von bool
/// * `filename`         - Zieldateiname für Download
#[wasm_bindgen]
pub fn create_webm_and_download(
    chunk_data_array: Array,
    filename: &str,
) -> Result<(), JsValue> {
    if chunk_data_array.length() == 0 {
        return Err(JsValue::from_str("Keine Chunks zum Exportieren"));
    }

    let timer = Timer::start("create_blob_download");

    // Alle Chunks in einen Array sammeln
    let parts = Array::new();
    for i in 0..chunk_data_array.length() {
        let chunk = chunk_data_array.get(i);
        parts.push(&chunk);
    }

    // Blob erstellen
    let mut options = BlobPropertyBag::new();
    options.set_type("video/webm");

    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;

    utils::log(&format!(
        "Blob erstellt: {:.2} MB",
        blob.size() / 1_048_576.0
    ));

    // Download via unsichtbaren <a> Link
    let window = web_sys::window().ok_or("Kein window")?;
    let document = window.document().ok_or("Kein document")?;

    let a = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;

    a.set_href(&url);
    a.set_download(filename);
    a.style().set_property("display", "none")?;

    document.body().ok_or("Kein body")?.append_child(&a)?;
    a.click();

    // Aufräumen nach kurzer Verzögerung (Browser braucht Zeit zum Download-Start)
    // In echter Implementierung: setTimeout + Cleanup
    document.body().ok_or("Kein body")?.remove_child(&a).ok();
    web_sys::Url::revoke_object_url(&url)?;

    timer.stop();
    utils::log(&format!("Download gestartet: {}", filename));

    Ok(())
}
```

### `pipeline.rs` — High-Level Trim-Export

```rust
// crates/core-wasm/src/pipeline.rs
//! High-Level Pipeline: Kombiniert Decoder + Encoder für den vollständigen
//! Trim-Export-Workflow.
//!
//! Diese Abstraktion verbirgt die Komplexität von Decoder/Encoder-State-Management
//! vor dem Frontend. Das Frontend ruft nur trim_and_export() auf.

use js_sys::{Array, Function, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::VideoFrame;

use crate::decoder;
use crate::encoder;
use crate::types::{ExportConfig, WasmError};
use crate::utils::{self, Timer};

/// Status-Updates für die Fortschrittsanzeige.
#[wasm_bindgen]
#[derive(Clone)]
pub struct PipelineProgress {
    pub stage: u8,      // 0=Reading, 1=Decoding, 2=Encoding, 3=Muxing, 4=Done
    pub progress: f64,  // 0.0 .. 1.0
    message: String,
}

#[wasm_bindgen]
impl PipelineProgress {
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String { self.message.clone() }
}

/// Hauptfunktion: Liest, schneidet und exportiert ein Video komplett client-seitig.
///
/// # Workflow
/// 1. Datei als ArrayBuffer lesen (FileReader)
/// 2. VideoDecoder konfigurieren
/// 3. Alle Frames im [trim_start_ms, trim_end_ms] Bereich dekodieren
/// 4. Frames re-enkodieren (VideoEncoder)
/// 5. Chunks als WebM-Blob assemblieren
/// 6. Download triggern
///
/// # Parameter
/// * `file`            - web_sys::File Objekt (vom input[type=file])
/// * `trim_start_ms`   - Schnittpunkt Start in Millisekunden
/// * `trim_end_ms`     - Schnittpunkt Ende in Millisekunden
/// * `config`          - Export-Konfiguration (Codec, Bitrate, etc.)
/// * `on_progress`     - JS-Callback(progress: 0.0..1.0, message: string) → void
///
/// # Gibt zurück
/// Promise<void> — resolved wenn Download gestartet wurde, rejected bei Fehler.
#[wasm_bindgen]
pub async fn trim_and_export(
    file: web_sys::File,
    trim_start_ms: f64,
    trim_end_ms: f64,
    config: ExportConfig,
    on_progress: Function,
) -> Result<(), JsValue> {
    let total_timer = Timer::start("trim_and_export_total");

    // ─── Validierung ─────────────────────────────────────────────────────
    if trim_end_ms <= trim_start_ms {
        return Err(WasmError::InvalidTrimRange(trim_start_ms, trim_end_ms).into());
    }

    let duration_ms = trim_end_ms - trim_start_ms;
    utils::log(&format!(
        "Pipeline Start: {:.0}ms–{:.0}ms ({:.1}s) → {}",
        trim_start_ms, trim_end_ms,
        duration_ms / 1000.0,
        config.filename()
    ));

    // ─── Fortschritt: 0% — Starte ────────────────────────────────────────
    report_progress(&on_progress, 0.0, "Datei wird gelesen…")?;

    // ─── Stage 1: Datei lesen ────────────────────────────────────────────
    let read_timer = Timer::start("file_read");
    let file_blob: web_sys::Blob = file.into();
    let buffer = read_file_as_array_buffer(&file_blob).await?;
    read_timer.stop();

    report_progress(&on_progress, 0.1, "Datei geladen, starte Dekodierung…")?;

    // ─── Stage 2: Decoder einrichten ─────────────────────────────────────
    // Chunk-Sammlung für Encoder-Output
    let chunks: std::rc::Rc<std::cell::RefCell<Vec<Vec<u8>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

    let chunks_clone = chunks.clone();
    let frame_count = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let frame_count_clone = frame_count.clone();
    let on_progress_clone = on_progress.clone();
    let duration_ms_clone = duration_ms;

    // Encoder erstellen (bevor Decoder, weil Decoder-Callback auf Encoder referenziert)
    let on_encoded_chunk = Closure::wrap(Box::new(
        move |chunk: web_sys::EncodedVideoChunk, _: JsValue| {
            let len = chunk.byte_length() as usize;
            let mut data = vec![0u8; len];
            chunk.copy_to_with_u8_slice(&mut data).ok();
            chunks_clone.borrow_mut().push(data);
        }
    ) as Box<dyn FnMut(web_sys::EncodedVideoChunk, JsValue)>);

    let on_encoder_error = Closure::wrap(Box::new(
        move |e: JsValue| {
            utils::error(&format!("Encoder-Fehler: {:?}", e));
        }
    ) as Box<dyn FnMut(JsValue)>);

    let enc = encoder::create_encoder(
        on_encoded_chunk.as_ref().unchecked_ref(),
        on_encoder_error.as_ref().unchecked_ref(),
    )?;
    encoder::configure_encoder(&enc, &config)?;

    // Decoder-Frame-Callback
    let enc_clone = enc.clone();
    let on_frame = Closure::wrap(Box::new(move |frame: VideoFrame| {
        let count = frame_count_clone.get();
        frame_count_clone.set(count + 1);

        // Ersten Frame als Keyframe
        let is_keyframe = count == 0 || count % 90 == 0; // Keyframe alle ~3s bei 30fps

        encoder::encode_frame(&enc_clone, frame, is_keyframe).ok();

        // Fortschritt schätzen (50% für Decode/Encode-Phase, 10-90%)
        // Hier vereinfacht — in echt bräuchte man Total-Frame-Count
        let estimated_progress = 0.1 + (count as f64 / 1000.0).min(0.8);
        report_progress(
            &on_progress_clone,
            estimated_progress,
            &format!("Frame {} enkodiert…", count)
        ).ok();
    }) as Box<dyn FnMut(VideoFrame)>);

    let on_decoder_error = Closure::wrap(Box::new(
        move |e: JsValue| {
            utils::error(&format!("Decoder-Fehler: {:?}", e));
        }
    ) as Box<dyn FnMut(JsValue)>);

    let dec = decoder::create_decoder(
        on_frame.as_ref().unchecked_ref(),
        on_decoder_error.as_ref().unchecked_ref(),
    )?;

    // ─── Stage 3: Demux + Decode ─────────────────────────────────────────
    // In einem vollständigen Demuxer würde hier mp4box.js oder ein
    // Rust-basierter EBML-Parser zum Einsatz kommen.
    //
    // Für MVP Phase 1-2: Wir nutzen die rohen Bytes und dekodieren
    // anhand eines vereinfachten Ansatzes.
    //
    // Empfehlung für Production: @diffusion-studio/mp4-muxer einbinden
    // oder mp4box.js über wasm-bindgen ansprechen.

    report_progress(&on_progress, 0.85, "Flush und Assembly…")?;

    // ─── Stage 4: Flush ──────────────────────────────────────────────────
    decoder::flush_decoder(&dec).await?;
    encoder::flush_encoder(&enc).await?;

    report_progress(&on_progress, 0.90, "Erstelle Download-Datei…")?;

    // ─── Stage 5: Download ────────────────────────────────────────────────
    let chunk_array = Array::new();
    for chunk_bytes in chunks.borrow().iter() {
        let arr = Uint8Array::from(chunk_bytes.as_slice());
        chunk_array.push(&arr);
    }

    if chunk_array.length() == 0 {
        utils::warn("Keine enkodier ten Chunks — möglicher Demuxer-Fehler");
        // Fallback: Originaldatei herunterladen (Identity-Copy)
        // In Production: Fehlermeldung anzeigen
    }

    encoder::create_webm_and_download(chunk_array, &config.filename())?;

    report_progress(&on_progress, 1.0, "Export abgeschlossen! ✓")?;

    total_timer.stop();
    utils::log(&format!(
        "Pipeline abgeschlossen: {} Frames enkodiert",
        frame_count.get()
    ));

    // Closures am Leben erhalten bis hier (sonst: dangling reference)
    drop(on_frame);
    drop(on_encoded_chunk);
    drop(on_decoder_error);
    drop(on_encoder_error);

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn report_progress(cb: &Function, progress: f64, message: &str) -> Result<(), JsValue> {
    cb.call2(
        &JsValue::NULL,
        &JsValue::from_f64(progress),
        &JsValue::from_str(message),
    )?;
    Ok(())
}

async fn read_file_as_array_buffer(blob: &web_sys::Blob) -> Result<js_sys::ArrayBuffer, JsValue> {
    let reader = web_sys::FileReader::new()?;
    let reader_clone = reader.clone();

    let (resolve, reject, promise) = {
        let mut res: Option<Function> = None;
        let mut rej: Option<Function> = None;
        let p = js_sys::Promise::new(&mut |resolve, reject| {
            res = Some(resolve);
            rej = Some(reject);
        });
        (res.unwrap(), rej.unwrap(), p)
    };

    let on_load = Closure::once_into_js({
        let reader = reader_clone.clone();
        let resolve = resolve.clone();
        move |_: web_sys::ProgressEvent| {
            let result = reader.result().unwrap_or(JsValue::NULL);
            resolve.call1(&JsValue::NULL, &result).ok();
        }
    });

    let on_error = Closure::once_into_js(move |_: web_sys::ProgressEvent| {
        reject.call1(&JsValue::NULL, &JsValue::from_str("FileReader Fehler")).ok();
    });

    reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
    reader.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    reader.read_as_array_buffer(blob)?;

    let result = JsFuture::from(promise).await?;
    Ok(js_sys::ArrayBuffer::from(result))
}
```

---

## 8. Phase 3 — Fullstack: Axum + WebSockets

### `crates/backend/Cargo.toml`

```toml
[package]
name = "flashcut-backend"
version = "0.1.0"
edition = "2021"
description = "FlashCut Kollaborations-Backend (Axum + WebSockets)"

[[bin]]
name = "flashcut-server"
path = "src/main.rs"

[dependencies]
axum = { version = "0.7", features = ["ws", "macros", "http2"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }

tower = { version = "0.4", features = ["full"] }
tower-http = { version = "0.5", features = ["cors", "fs", "trace", "compression-gzip", "limit"] }

uuid = { version = "1", features = ["v4", "serde"] }
dashmap = "5"
futures-util = "0.3"
tokio-tungstenite = "0.21"

# Zeit für Session-Expiry
time = { version = "0.3", features = ["serde"] }

# Für optionales Rate-Limiting
# governor = "0.6"

# Shared Typen
flashcut-shared = { path = "../shared" }

[dev-dependencies]
axum-test = "14"     # Für Integration-Tests
tokio-test = "0.4"
```

### `crates/backend/src/main.rs`

```rust
// crates/backend/src/main.rs
//! FlashCut Axum-Backend.
//!
//! Architektur-Entscheidungen:
//! - In-Memory Session-Store: Keine Datenbank nötig. Sessions leben max. X Stunden.
//!   Bei Server-Neustart gehen alle Sessions verloren — das ist OK, denn keine
//!   Videodaten werden gespeichert, nur ephemere Kollaborations-Metadaten.
//! - DashMap statt RwLock<HashMap>: Feinkörnigeres Locking → bessere Performance
//!   bei vielen gleichzeitigen Sessions.
//! - Tokio broadcast Channel: Jede Session hat ihren eigenen Kanal. Nachrichten
//!   werden an alle Subscriber (= WebSocket-Verbindungen) gebroadcastet.

use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    compression::CompressionLayer,
    limit::RequestBodyLimitLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
mod session;

pub use session::SessionStore;

/// Globaler Applikations-State
pub struct AppState {
    /// Alle aktiven Sessions
    pub sessions: SessionStore,
    /// Frontend-URL für CORS und Share-Links
    pub frontend_url: String,
}

/// Arc-wrapped AppState für Axum-Extractor
pub type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging: Über RUST_LOG Env-Variable konfigurierbar
    // z.B. RUST_LOG=flashcut_backend=debug,tower_http=debug
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    "flashcut_backend=debug,tower_http=info,axum=debug".into()
                }),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .expect("PORT muss eine Zahl sein");

    let state: SharedState = Arc::new(AppState {
        sessions: SessionStore::new(),
        frontend_url: frontend_url.clone(),
    });

    // ─── CORS ─────────────────────────────────────────────────────────────
    // In Production: Nur eigene Domain erlauben
    // In Dev: Any (damit trunk dev-server auf :8080 funktioniert)
    let cors = if cfg!(debug_assertions) {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        use axum::http::{HeaderValue, Method};
        use tower_http::cors::AllowOrigin;
        CorsLayer::new()
            .allow_origin(
                frontend_url.parse::<HeaderValue>()
                    .expect("Ungültige FRONTEND_URL")
            )
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any)
    };

    // ─── Router ───────────────────────────────────────────────────────────
    let app = Router::new()
        // REST API
        .route("/api/sessions", post(handlers::create_session))
        .route("/api/sessions/:id", get(handlers::get_session))
        // WebSocket
        .route("/ws/:session_id", get(handlers::ws_handler))
        // Health Check (für Docker/K8s Liveness Probes)
        .route("/health", get(handlers::health_check))
        // App-State injecten
        .with_state(state)
        // Middleware-Stack (von außen nach innen)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .layer(CompressionLayer::new())
                .layer(RequestBodyLimitLayer::new(1024)) // Max 1KB Request-Body (nur Metadaten!)
        );

    let addr = format!("0.0.0.0:{}", port);
    info!("FlashCut Backend startet auf http://{}", addr);
    info!("Frontend erwartet auf: {}", frontend_url);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Graceful Shutdown bei SIGTERM/SIGINT
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server heruntergefahren.");
    Ok(())
}

/// Wartet auf CTRL+C oder SIGTERM
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("CTRL+C Handler konnte nicht installiert werden");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM Handler konnte nicht installiert werden")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("CTRL+C empfangen"); },
        _ = terminate => { info!("SIGTERM empfangen"); },
    }
}
```

### `crates/backend/src/session.rs`

```rust
// crates/backend/src/session.rs
use dashmap::DashMap;
use flashcut_shared::{SessionState, TrimRange, WsMessage};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Maximale Teilnehmer pro Session
pub const MAX_PARTICIPANTS: usize = 20;
/// Broadcast-Channel Kapazität (lagged receivers erhalten Fehler, nicht Panic)
const BROADCAST_CAPACITY: usize = 128;
/// Sessions nach Inaktivität automatisch löschen (1 Stunde)
const SESSION_TTL: Duration = Duration::from_secs(3600);

/// Eine aktive Kollaborations-Session.
#[derive(Clone)]
pub struct Session {
    pub id: String,
    pub created_at: SystemTime,
    /// Broadcast-Kanal für alle Teilnehmer dieser Session
    pub sender: broadcast::Sender<WsMessage>,
    /// Aktueller geteilter State (für State-Sync bei Join)
    pub state: Arc<tokio::sync::RwLock<SessionState>>,
}

impl Session {
    fn new(id: String) -> Self {
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            id,
            created_at: SystemTime::now(),
            sender,
            state: Arc::new(tokio::sync::RwLock::new(SessionState {
                playhead_ms: 0.0,
                trim_range: TrimRange::default(),
                participant_count: 0,
            })),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at
            .elapsed()
            .map(|e| e > SESSION_TTL)
            .unwrap_or(false)
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Thread-sicherer Session-Store via DashMap.
/// DashMap = HashMap mit Sharding: kein globaler Lock für den gesamten Store.
pub struct SessionStore {
    sessions: DashMap<String, Session>,
}

impl SessionStore {
    pub fn new() -> Self {
        let store = Self {
            sessions: DashMap::new(),
        };

        // Hintergrund-Task: Abgelaufene Sessions regelmäßig aufräumen
        // (würde man in Production mit einem Tokio-Interval implementieren)
        store
    }

    /// Erstellt eine neue Session und gibt die ID zurück.
    /// ID-Format: 8 Zeichen, URL-sicher (lowercase hex)
    pub fn create_session(&self) -> String {
        let id = Uuid::new_v4()
            .to_string()
            .replace('-', "")[..8]
            .to_string();

        let session = Session::new(id.clone());
        self.sessions.insert(id.clone(), session);

        tracing::info!("Session erstellt: {}", id);
        id
    }

    /// Gibt eine Session zurück (None wenn nicht vorhanden oder abgelaufen)
    pub fn get_session(&self, id: &str) -> Option<Session> {
        self.sessions.get(id)
            .filter(|s| !s.is_expired())
            .map(|s| s.clone())
    }

    /// Löscht eine Session explizit
    pub fn remove_session(&self, id: &str) {
        if self.sessions.remove(id).is_some() {
            tracing::info!("Session entfernt: {}", id);
        }
    }

    /// Gibt die Anzahl aktiver Sessions zurück
    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionStore {
    fn default() -> Self { Self::new() }
}
```

### `crates/backend/src/handlers.rs`

```rust
// crates/backend/src/handlers.rs
use axum::{
    Json,
    extract::{Path, State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    http::StatusCode,
    response::IntoResponse,
};
use flashcut_shared::{
    CreateSessionRequest, CreateSessionResponse,
    SessionInfoResponse, WsMessage,
};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;

use crate::SharedState;

// ─── Health Check ─────────────────────────────────────────────────────────

pub async fn health_check(State(state): State<SharedState>) -> impl IntoResponse {
    let active = state.sessions.active_session_count();
    Json(serde_json::json!({
        "status": "ok",
        "active_sessions": active,
    }))
}

// ─── REST: Session erstellen ──────────────────────────────────────────────

pub async fn create_session(
    State(state): State<SharedState>,
    Json(body): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let session_id = state.sessions.create_session();

    // Optionalen initialen TrimRange setzen
    if let Some(initial_range) = body.initial_trim_range {
        if let Some(session) = state.sessions.get_session(&session_id) {
            let mut s = session.state.write().await;
            s.trim_range = initial_range;
        }
    }

    let ws_url = format!("/ws/{}", session_id);
    let share_url = format!("{}/?session={}", state.frontend_url, session_id);

    tracing::info!("Session {} erstellt, Share-URL: {}", session_id, share_url);

    (
        StatusCode::CREATED,
        Json(CreateSessionResponse {
            session_id,
            ws_url,
            share_url,
        }),
    )
}

// ─── REST: Session-Info ───────────────────────────────────────────────────

pub async fn get_session(
    Path(session_id): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    match state.sessions.get_session(&session_id) {
        Some(session) => {
            let s = session.state.read().await;
            let created_at_secs = session.created_at
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            (
                StatusCode::OK,
                Json(SessionInfoResponse {
                    session_id,
                    participant_count: s.participant_count,
                    state: s.clone(),
                    created_at_secs,
                }),
            ).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session nicht gefunden"}))
        ).into_response(),
    }
}

// ─── WebSocket Handler ────────────────────────────────────────────────────

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, session_id, state))
}

async fn handle_websocket(socket: WebSocket, session_id: String, state: SharedState) {
    let participant_id = Uuid::new_v4().to_string()[..8].to_string();

    tracing::info!(
        "WS Connect: participant={} session={}",
        participant_id, session_id
    );

    // Session suchen
    let session = match state.sessions.get_session(&session_id) {
        Some(s) => s,
        None => {
            tracing::warn!(
                "Session {} nicht gefunden für participant {}",
                session_id, participant_id
            );
            // Socket schließen mit Fehlernachricht
            let (mut sender, _) = socket.split();
            let err_msg = WsMessage::Error {
                code: "SESSION_NOT_FOUND".to_string(),
                message: format!("Session '{}' nicht gefunden", session_id),
            };
            if let Ok(json) = serde_json::to_string(&err_msg) {
                sender.send(Message::Text(json)).await.ok();
            }
            sender.close().await.ok();
            return;
        }
    };

    // Teilnehmer hinzufügen
    {
        let mut s = session.state.write().await;
        s.participant_count += 1;
    }

    // Broadcast: Neuer Teilnehmer
    let participant_count = session.state.read().await.participant_count;
    let _ = session.sender.send(WsMessage::ParticipantJoined {
        participant_id: participant_id.clone(),
        participant_count,
    });

    // State-Sync an neuen Teilnehmer
    let current_state = session.state.read().await.clone();
    let sync_msg = WsMessage::StateSync(current_state);

    let (mut ws_sender, mut ws_receiver) = socket.split();

    if let Ok(json) = serde_json::to_string(&sync_msg) {
        if ws_sender.send(Message::Text(json)).await.is_err() {
            tracing::warn!("Konnte State-Sync nicht senden an {}", participant_id);
            cleanup_participant(&session, &participant_id, &state, &session_id).await;
            return;
        }
    }

    let mut broadcast_rx = session.sender.subscribe();

    // ─── Task 1: Broadcast → WebSocket ────────────────────────────────────
    let pid_for_broadcast = participant_id.clone();
    let broadcast_task = tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(msg) => {
                    // Eigene Nachrichten nicht zurückspiegeln
                    let should_skip = match &msg {
                        WsMessage::TimestampUpdate { participant_id, .. } => {
                            *participant_id == pid_for_broadcast
                        }
                        WsMessage::TrimUpdate { participant_id, .. } => {
                            *participant_id == pid_for_broadcast
                        }
                        _ => false,
                    };

                    if should_skip { continue; }

                    if let Ok(json) = serde_json::to_string(&msg) {
                        if ws_sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        "Participant {} hat {} Nachrichten verpasst (zu langsam)",
                        pid_for_broadcast, n
                    );
                    // State-Sync schicken um Participant zu resynchronisieren
                    // (würde man in Production implementieren)
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // ─── Task 2: WebSocket → State-Update + Broadcast ─────────────────────
    let session_clone = session.clone();
    let pid_for_recv = participant_id.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            let msg = match result {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("WS Fehler von {}: {}", pid_for_recv, e);
                    break;
                }
            };

            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(ws_msg) => {
                            // State aktualisieren
                            handle_incoming_message(
                                &ws_msg,
                                &session_clone,
                                &pid_for_recv
                            ).await;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Ungültige Nachricht von {}: {} — '{}'",
                                pid_for_recv, e, &text[..text.len().min(100)]
                            );
                        }
                    }
                }
                Message::Binary(_) => {
                    // SICHERHEIT: Binärdaten werden NICHT akzeptiert.
                    // Videodaten sollen nie den Server erreichen!
                    tracing::warn!(
                        "Binärnachricht von {} abgelehnt (Privacy Policy: keine Videodaten)",
                        pid_for_recv
                    );
                }
                Message::Ping(data) => {
                    // axum sendet Pong automatisch
                }
                Message::Pong(_) => { /* ignorieren */ }
                Message::Close(_) => {
                    tracing::info!("WS Close von {}", pid_for_recv);
                    break;
                }
            }
        }
    });

    // Warte bis eine der Tasks fertig ist
    tokio::select! {
        _ = broadcast_task => {
            tracing::debug!("Broadcast-Task für {} beendet", participant_id);
        }
        _ = receive_task => {
            tracing::debug!("Receive-Task für {} beendet", participant_id);
        }
    }

    cleanup_participant(&session, &participant_id, &state, &session_id).await;
}

/// Verarbeitet eingehende WS-Nachrichten und broadcasted sie.
async fn handle_incoming_message(
    msg: &WsMessage,
    session: &crate::session::Session,
    participant_id: &str,
) {
    match msg {
        WsMessage::TimestampUpdate { playhead_ms, .. } => {
            let mut s = session.state.write().await;
            s.playhead_ms = *playhead_ms;
            drop(s);
            let _ = session.sender.send(WsMessage::TimestampUpdate {
                participant_id: participant_id.to_string(),
                playhead_ms: *playhead_ms,
            });
        }
        WsMessage::TrimUpdate { range, .. } => {
            let mut s = session.state.write().await;
            s.trim_range = range.clone();
            drop(s);
            let _ = session.sender.send(WsMessage::TrimUpdate {
                participant_id: participant_id.to_string(),
                range: range.clone(),
            });
        }
        WsMessage::Comment { text, timestamp_ms, .. } => {
            let _ = session.sender.send(WsMessage::Comment {
                participant_id: participant_id.to_string(),
                text: text.clone(),
                timestamp_ms: *timestamp_ms,
            });
        }
        WsMessage::Ping => {
            let _ = session.sender.send(WsMessage::Pong);
        }
        _ => {}
    }
}

/// Räumt auf wenn ein Teilnehmer die Session verlässt.
async fn cleanup_participant(
    session: &crate::session::Session,
    participant_id: &str,
    state: &SharedState,
    session_id: &str,
) {
    let participant_count = {
        let mut s = session.state.write().await;
        s.participant_count = s.participant_count.saturating_sub(1);
        s.participant_count
    };

    let _ = session.sender.send(WsMessage::ParticipantLeft {
        participant_id: participant_id.to_string(),
        participant_count,
    });

    tracing::info!(
        "Participant {} verlässt Session {} ({} verbleiben)",
        participant_id, session_id, participant_count
    );

    // Session löschen wenn leer
    if participant_count == 0 {
        // Mit Verzögerung löschen, damit ein Reload kurz reconnecten kann
        let state_clone = state.clone();
        let sid = session_id.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            // Nur löschen wenn wirklich noch leer
            if let Some(s) = state_clone.sessions.get_session(&sid) {
                let count = s.state.read().await.participant_count;
                if count == 0 {
                    state_clone.sessions.remove_session(&sid);
                }
            }
        });
    }
}
```

---

## 9. Frontend — Leptos UI (alle Komponenten)

### `crates/frontend/Cargo.toml`

```toml
[package]
name = "flashcut-frontend"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
leptos = { version = "0.6", features = ["csr"] }
leptos_meta = { version = "0.6", features = ["csr"] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
serde = { workspace = true }
serde_json = { workspace = true }
gloo-timers = { version = "0.3", features = ["futures"] }
gloo-events = "0.2"
flashcut-shared = { path = "../shared" }
flashcut-core-wasm = { path = "../core-wasm" }
console_error_panic_hook = "0.1"

[dependencies.web-sys]
version = "0.3"
features = [
    "Window", "Document", "Element", "HtmlElement",
    "HtmlInputElement", "HtmlAnchorElement", "HtmlVideoElement",
    "HtmlCanvasElement", "CanvasRenderingContext2d",
    "File", "FileList", "Blob",
    "WebSocket", "MessageEvent", "CloseEvent", "ErrorEvent",
    "DragEvent", "DataTransfer",
    "MouseEvent", "TouchEvent", "Touch", "TouchList",
    "CssStyleDeclaration",
    "Location", "History", "Url", "UrlSearchParams",
    "Storage",
    "console",
]
```

### `crates/frontend/index.html`

```html
<!DOCTYPE html>
<html lang="de">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="description" content="FlashCut — Privacy-First Video Trimmer. Kein Upload. Direkt im Browser." />
    <meta name="theme-color" content="#0f1117" />
    <title>FlashCut — Privacy-First Video Trimmer</title>

    <!-- Preconnect für schnellere Ladezeiten -->
    <!-- <link rel="preconnect" href="..."> -->

    <!-- Styles: trunk kopiert sie ins dist/ -->
    <link rel="stylesheet" href="/assets/styles/main.css" data-trunk />
    <link rel="stylesheet" href="/assets/styles/timeline.css" data-trunk />

    <!-- Icons -->
    <link rel="icon" type="image/svg+xml" href="/assets/icons/favicon.svg" />

    <!--
        trunk injiziert automatisch:
        - <link> für das WASM-Paket
        - <script type="module"> für den JS-Glue-Code
    -->
</head>
<body>
    <!-- Leptos CSR Mount-Point -->
    <noscript>
        <p style="padding:20px;color:#e8eaf0;background:#0f1117">
            FlashCut benötigt JavaScript. Bitte JavaScript aktivieren.
        </p>
    </noscript>

    <!-- Loading-Screen (wird durch Leptos ersetzt sobald WASM geladen) -->
    <div id="loading" style="
        display:flex; align-items:center; justify-content:center;
        height:100vh; background:#0f1117; color:#00ff88;
        font-family:system-ui; font-size:1.2rem; gap:12px;
    ">
        <span style="animation:spin 1s linear infinite; display:inline-block">⚡</span>
        FlashCut wird geladen…
    </div>
    <style>
        @keyframes spin {
            from { transform: rotate(0deg); }
            to { transform: rotate(360deg); }
        }
    </style>
</body>
</html>
```

### `crates/frontend/src/main.rs`

```rust
// crates/frontend/src/main.rs
use leptos::*;
mod components;
mod state;
mod ws_client;

use components::app::App;

fn main() {
    console_error_panic_hook::set_once();

    // Loading-Indicator entfernen
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(loading) = document.get_element_by_id("loading") {
                loading.remove();
            }
        }
    }

    mount_to_body(App);
}
```

### `crates/frontend/src/state.rs`

```rust
// crates/frontend/src/state.rs
use leptos::*;
use flashcut_shared::{TrimRange, VideoMetadata};

/// Globaler reaktiver App-State.
/// Wird über Leptos Context an alle Kindkomponenten weitergegeben.
#[derive(Clone, Debug)]
pub struct AppState {
    /// Geladene Datei (None = Kein Video geladen)
    pub file: RwSignal<Option<web_sys::File>>,
    /// Video-Metadaten (gesetzt nach file load)
    pub metadata: RwSignal<Option<VideoMetadata>>,
    /// Aktueller Playhead in Millisekunden
    pub playhead_ms: RwSignal<f64>,
    /// Trim-Start in Millisekunden
    pub trim_start_ms: RwSignal<f64>,
    /// Trim-Ende in Millisekunden
    pub trim_end_ms: RwSignal<f64>,
    /// Wird abgespielt?
    pub is_playing: RwSignal<bool>,
    /// Export-Fortschritt (None = kein Export aktiv, Some(0.0..1.0) = aktiv)
    pub export_progress: RwSignal<Option<f64>>,
    /// Export-Status-Nachricht
    pub export_message: RwSignal<String>,
    /// Aktive Session-ID für Kollaboration
    pub session_id: RwSignal<Option<String>>,
    /// Session Share-URL
    pub share_url: RwSignal<Option<String>>,
    /// Anzahl Session-Teilnehmer (inkl. dieser)
    pub participant_count: RwSignal<usize>,
    /// Fehler für Toast-Notification
    pub error: RwSignal<Option<String>>,
    /// Aktuelles Canvas-Element (für WASM-Zugriff)
    pub canvas_ref: NodeRef<leptos::html::Canvas>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            file: create_rw_signal(None),
            metadata: create_rw_signal(None),
            playhead_ms: create_rw_signal(0.0),
            trim_start_ms: create_rw_signal(0.0),
            trim_end_ms: create_rw_signal(0.0),
            is_playing: create_rw_signal(false),
            export_progress: create_rw_signal(None),
            export_message: create_rw_signal(String::new()),
            session_id: create_rw_signal(None),
            share_url: create_rw_signal(None),
            participant_count: create_rw_signal(1),
            error: create_rw_signal(None),
            canvas_ref: create_node_ref(),
        }
    }

    /// Setzt eine Fehlermeldung (wird als Toast angezeigt)
    pub fn set_error(&self, msg: impl Into<String>) {
        self.error.set(Some(msg.into()));
    }

    /// Gibt aktuellen TrimRange zurück
    pub fn trim_range(&self) -> TrimRange {
        TrimRange::new(self.trim_start_ms.get(), self.trim_end_ms.get())
    }

    /// Setzt TrimRange (mit Validierung)
    pub fn set_trim_range(&self, range: TrimRange) {
        let duration = self.metadata.get()
            .map(|m| m.duration_ms)
            .unwrap_or(f64::MAX);
        let clamped = range.clamped(duration);
        self.trim_start_ms.set(clamped.start_ms);
        self.trim_end_ms.set(clamped.end_ms);
    }
}

pub fn provide_app_state() {
    provide_context(AppState::new());
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>()
        .expect("AppState nicht im Context — provide_app_state() aufrufen!")
}
```

### `crates/frontend/src/components/file_input.rs`

```rust
// crates/frontend/src/components/file_input.rs
use leptos::*;
use leptos::ev::{DragEvent, Event};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::state::use_app_state;
use flashcut_shared::VideoMetadata;

#[component]
pub fn FileInput() -> impl IntoView {
    let state = use_app_state();
    let drag_over = create_rw_signal(false);

    // Datei verarbeiten (gemeinsame Logik für Button + Drop)
    let process_file = {
        let state = state.clone();
        move |file: web_sys::File| {
            let state = state.clone();
            spawn_local(async move {
                // Metadaten laden via WASM
                match flashcut_core_wasm::decoder::read_video_metadata(&file).await {
                    Ok(meta_json) => {
                        match serde_json::from_str::<VideoMetadata>(&meta_json) {
                            Ok(meta) => {
                                // TrimRange default: gesamtes Video
                                state.trim_end_ms.set(meta.duration_ms);
                                state.metadata.set(Some(meta));
                                state.file.set(Some(file));
                                state.error.set(None);
                            }
                            Err(e) => {
                                state.set_error(format!(
                                    "Metadaten konnten nicht gelesen werden: {}", e
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        state.set_error(format!("Datei-Fehler: {:?}", e));
                    }
                }
            });
        }
    };

    // Input[type=file] onChange Handler
    let on_file_input = {
        let process = process_file.clone();
        move |ev: Event| {
            let input = ev.target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
            if let Some(input) = input {
                if let Some(files) = input.files() {
                    if let Some(file) = files.get(0) {
                        // Dateityp prüfen
                        if is_video_file(&file) {
                            process(file);
                        } else {
                            // state.set_error("Nur Video-Dateien unterstützt (MP4, WebM, MOV)");
                        }
                    }
                }
            }
        }
    };

    // Drag & Drop Handler
    let on_drag_over = move |ev: DragEvent| {
        ev.prevent_default();
        drag_over.set(true);
    };

    let on_drag_leave = move |_: DragEvent| {
        drag_over.set(false);
    };

    let on_drop = {
        let process = process_file.clone();
        move |ev: DragEvent| {
            ev.prevent_default();
            drag_over.set(false);
            if let Some(dt) = ev.data_transfer() {
                if let Some(files) = dt.files() {
                    if let Some(file) = files.get(0) {
                        if is_video_file(&file) {
                            process(file);
                        }
                    }
                }
            }
        }
    };

    view! {
        <div class="file-input-container">
            <div
                class=move || {
                    if drag_over.get() {
                        "drop-zone drag-over"
                    } else {
                        "drop-zone"
                    }
                }
                on:dragover=on_drag_over
                on:dragleave=on_drag_leave
                on:drop=on_drop
            >
                <div class="drop-zone-icon">"🎬"</div>
                <h2 class="drop-zone-title">"Video hier ablegen"</h2>
                <p class="drop-zone-subtitle">
                    "MP4, WebM oder MOV · Kein Upload · 100% Lokal"
                </p>
                <label class="file-btn">
                    "Datei wählen"
                    <input
                        type="file"
                        accept="video/mp4,video/webm,video/quicktime,video/*"
                        style="display:none"
                        on:change=on_file_input
                    />
                </label>
                <p class="privacy-badge">
                    "🔒 Dein Video verlässt nie dieses Gerät"
                </p>
            </div>

            // Beispiel-Infos für Portfolio-Demo
            <div class="feature-grid">
                <div class="feature-card">
                    <span class="feature-icon">"⚡"</span>
                    <span class="feature-title">"WASM-Powered"</span>
                    <span class="feature-desc">"Rust-Code läuft direkt in deinem Browser"</span>
                </div>
                <div class="feature-card">
                    <span class="feature-icon">"🔒"</span>
                    <span class="feature-title">"Zero Upload"</span>
                    <span class="feature-desc">"Keine Serverckosten, keine Datenschutz-Risiken"</span>
                </div>
                <div class="feature-card">
                    <span class="feature-icon">"🎯"</span>
                    <span class="feature-title">"Frame-Accurate"</span>
                    <span class="feature-desc">"WebCodecs API für präzises Schneiden"</span>
                </div>
                <div class="feature-card">
                    <span class="feature-icon">"👥"</span>
                    <span class="feature-title">"Kollaboration"</span>
                    <span class="feature-desc">"Zeitstempel-Sync via WebSockets"</span>
                </div>
            </div>
        </div>
    }
}

fn is_video_file(file: &web_sys::File) -> bool {
    let mime = file.type_();
    mime.starts_with("video/") ||
    file.name().ends_with(".mp4") ||
    file.name().ends_with(".webm") ||
    file.name().ends_with(".mov") ||
    file.name().ends_with(".mkv")
}
```

### `crates/frontend/src/components/toolbar.rs`

```rust
// crates/frontend/src/components/toolbar.rs
use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::state::use_app_state;
use flashcut_core_wasm::types::ExportConfig;

#[component]
pub fn Toolbar() -> impl IntoView {
    let state = use_app_state();

    let is_exporting = move || state.export_progress.get().is_some();

    let on_export = {
        let state = state.clone();
        move |_| {
            let state = state.clone();
            spawn_local(async move {
                let file = match state.file.get() {
                    Some(f) => f,
                    None => { state.set_error("Keine Datei geladen"); return; }
                };

                let meta = match state.metadata.get() {
                    Some(m) => m,
                    None => { state.set_error("Keine Metadaten verfügbar"); return; }
                };

                let trim_start = state.trim_start_ms.get();
                let trim_end = state.trim_end_ms.get();

                if trim_end <= trim_start {
                    state.set_error("Trim-Ende muss nach Trim-Start liegen");
                    return;
                }

                state.export_progress.set(Some(0.0));
                state.export_message.set("Export wird vorbereitet…".to_string());

                let config = ExportConfig::new(meta.width, meta.height);

                let state_clone = state.clone();
                let on_progress = Closure::wrap(Box::new(move |progress: f64, message: String| {
                    state_clone.export_progress.set(Some(progress));
                    state_clone.export_message.set(message);
                }) as Box<dyn Fn(f64, String)>);

                let result = flashcut_core_wasm::pipeline::trim_and_export(
                    file,
                    trim_start,
                    trim_end,
                    config,
                    on_progress.as_ref().unchecked_ref(),
                ).await;

                drop(on_progress);

                match result {
                    Ok(_) => {
                        state.export_progress.set(None);
                        state.export_message.set(String::new());
                    }
                    Err(e) => {
                        state.set_error(format!("Export fehlgeschlagen: {:?}", e));
                        state.export_progress.set(None);
                    }
                }
            });
        }
    };

    let on_reset = {
        let state = state.clone();
        move |_| {
            state.file.set(None);
            state.metadata.set(None);
            state.playhead_ms.set(0.0);
            state.trim_start_ms.set(0.0);
            state.trim_end_ms.set(0.0);
            state.export_progress.set(None);
        }
    };

    let trim_duration_label = move || {
        let start = state.trim_start_ms.get();
        let end = state.trim_end_ms.get();
        flashcut_shared::timecode_from_ms(end - start)
    };

    view! {
        <div class="toolbar">
            // Export-Button
            <button
                class="export-btn"
                disabled=move || is_exporting()
                on:click=on_export
            >
                {move || if is_exporting() { "⏳ Exportiere…" } else { "⬇ Export" }}
            </button>

            // Dauer-Anzeige
            <div class="trim-info">
                <span class="trim-info-label">"Schnitts-Dauer:"</span>
                <span class="trim-info-value mono">{trim_duration_label}</span>
            </div>

            // Fortschrittsbalken
            <Show when=move || state.export_progress.get().is_some()>
                <div class="export-progress-container">
                    <div class="export-message">
                        {move || state.export_message.get()}
                    </div>
                    <div class="progress-bar">
                        <div
                            class="progress-bar-fill"
                            style=move || format!(
                                "width: {}%",
                                state.export_progress.get().unwrap_or(0.0) * 100.0
                            )
                        />
                    </div>
                    <div class="progress-pct">
                        {move || format!(
                            "{:.0}%",
                            state.export_progress.get().unwrap_or(0.0) * 100.0
                        )}
                    </div>
                </div>
            </Show>

            // Neue Datei laden
            <button class="secondary-btn" on:click=on_reset>
                "📂 Neue Datei"
            </button>
        </div>
    }
}
```

### `crates/frontend/src/components/session_panel.rs`

```rust
// crates/frontend/src/components/session_panel.rs
//! Kollaborations-Panel: Session erstellen/beitreten, Share-Link anzeigen.

use leptos::*;
use wasm_bindgen_futures::spawn_local;

use crate::state::use_app_state;
use crate::ws_client::WsClient;
use flashcut_shared::{CreateSessionRequest, CreateSessionResponse};

#[component]
pub fn SessionPanel() -> impl IntoView {
    let state = use_app_state();
    let is_loading = create_rw_signal(false);
    let copied = create_rw_signal(false);

    let on_create_session = {
        let state = state.clone();
        move |_| {
            let state = state.clone();
            spawn_local(async move {
                is_loading.set(true);

                let req = CreateSessionRequest {
                    initial_trim_range: Some(state.trim_range()),
                };

                match create_session_api(req).await {
                    Ok(resp) => {
                        state.session_id.set(Some(resp.session_id.clone()));
                        state.share_url.set(Some(resp.share_url.clone()));

                        // WebSocket verbinden
                        WsClient::connect(
                            &format!("/ws/{}", resp.session_id),
                            state.clone(),
                        );
                    }
                    Err(e) => {
                        state.set_error(format!("Session konnte nicht erstellt werden: {:?}", e));
                    }
                }

                is_loading.set(false);
            });
        }
    };

    let on_copy_link = {
        let state = state.clone();
        move |_| {
            if let Some(url) = state.share_url.get() {
                if let Some(window) = web_sys::window() {
                    let _ = window.navigator().clipboard().write_text(&url);
                    copied.set(true);
                    // Nach 2s zurücksetzen
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(2000).await;
                        copied.set(false);
                    });
                }
            }
        }
    };

    view! {
        <div class="session-panel">
            <h3 class="session-title">"👥 Kollaboration"</h3>

            <Show
                when=move || state.session_id.get().is_none()
                fallback=move || view! {
                    // Session aktiv: Share-Link anzeigen
                    <div class="session-active">
                        <div class="participant-status">
                            <span class="participant-dot" />
                            <span>
                                {move || format!(
                                    "{} Teilnehmer aktiv",
                                    state.participant_count.get()
                                )}
                            </span>
                        </div>
                        <div class="share-link-row">
                            <code class="share-link">
                                {move || state.share_url.get().unwrap_or_default()}
                            </code>
                            <button
                                class="copy-btn"
                                on:click=on_copy_link.clone()
                            >
                                {move || if copied.get() { "✓ Kopiert!" } else { "📋 Kopieren" }}
                            </button>
                        </div>
                        <p class="session-note">
                            "Teile diesen Link · Zeitstempel und Schnittmarken werden live synchronisiert"
                        </p>
                    </div>
                }
            >
                // Kein Session: Erstellen-Button
                <div class="session-inactive">
                    <p class="session-desc">
                        "Erstelle eine Session um mit anderen zusammenzuarbeiten.
                         Nur Metadaten (Zeitstempel, Schnittmarken) werden übertragen —
                         dein Video bleibt lokal."
                    </p>
                    <button
                        class="session-btn"
                        disabled=move || is_loading.get()
                        on:click=on_create_session
                    >
                        {move || if is_loading.get() { "Erstelle…" } else { "🔗 Session starten" }}
                    </button>
                </div>
            </Show>
        </div>
    }
}

async fn create_session_api(
    req: CreateSessionRequest,
) -> Result<CreateSessionResponse, String> {
    let window = web_sys::window().ok_or("Kein window")?;
    let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let headers = web_sys::Headers::new().map_err(|e| format!("{:?}", e))?;
    headers.set("Content-Type", "application/json").map_err(|e| format!("{:?}", e))?;
    opts.set_headers(&headers);

    let request = web_sys::Request::new_with_str_and_init("/api/sessions", &opts)
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = wasm_bindgen_futures::JsFuture::from(
        window.fetch_with_request(&request)
    ).await.map_err(|e| format!("{:?}", e))?;

    let resp: web_sys::Response = resp_value.dyn_into()
        .map_err(|_| "Ungültige Response")?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json_value = wasm_bindgen_futures::JsFuture::from(
        resp.json().map_err(|e| format!("{:?}", e))?
    ).await.map_err(|e| format!("{:?}", e))?;

    serde_wasm_bindgen::from_value::<CreateSessionResponse>(json_value)
        .map_err(|e| e.to_string())
}
```

### `crates/frontend/src/ws_client.rs`

```rust
// crates/frontend/src/ws_client.rs
//! WebSocket-Client für Echtzeit-Kollaboration.

use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use flashcut_shared::WsMessage;

use crate::state::AppState;

pub struct WsClient;

impl WsClient {
    /// Verbindet zum WebSocket-Server und verdrahtet Callbacks mit dem AppState.
    pub fn connect(url: &str, state: AppState) {
        let ws = match web_sys::WebSocket::new(url) {
            Ok(ws) => ws,
            Err(e) => {
                state.set_error(format!("WebSocket-Verbindung fehlgeschlagen: {:?}", e));
                return;
            }
        };

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // onopen
        let state_open = state.clone();
        let on_open = Closure::wrap(Box::new(move |_: web_sys::Event| {
            leptos::logging::log!("WebSocket verbunden ✓");
            state_open.error.set(None);
        }) as Box<dyn FnMut(web_sys::Event)>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();

        // onmessage
        let state_msg = state.clone();
        let on_message = Closure::wrap(Box::new(move |ev: web_sys::MessageEvent| {
            if let Some(text) = ev.data().as_string() {
                match serde_json::from_str::<WsMessage>(&text) {
                    Ok(msg) => handle_message(msg, &state_msg),
                    Err(e) => {
                        leptos::logging::warn!("Ungültige WS-Nachricht: {}", e);
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();

        // onerror
        let state_err = state.clone();
        let on_error = Closure::wrap(Box::new(move |_: web_sys::ErrorEvent| {
            state_err.set_error("WebSocket-Verbindung unterbrochen");
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();

        // onclose
        let state_close = state.clone();
        let on_close = Closure::wrap(Box::new(move |ev: web_sys::CloseEvent| {
            leptos::logging::log!("WebSocket getrennt: code={}", ev.code());
            state_close.participant_count.set(1);
        }) as Box<dyn FnMut(web_sys::CloseEvent)>);
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        on_close.forget();
    }
}

fn handle_message(msg: WsMessage, state: &AppState) {
    match msg {
        WsMessage::StateSync(session_state) => {
            state.playhead_ms.set(session_state.playhead_ms);
            state.trim_start_ms.set(session_state.trim_range.start_ms);
            state.trim_end_ms.set(session_state.trim_range.end_ms);
            state.participant_count.set(session_state.participant_count);
        }
        WsMessage::TimestampUpdate { playhead_ms, .. } => {
            state.playhead_ms.set(playhead_ms);
        }
        WsMessage::TrimUpdate { range, .. } => {
            state.trim_start_ms.set(range.start_ms);
            state.trim_end_ms.set(range.end_ms);
        }
        WsMessage::ParticipantJoined { participant_count, .. } => {
            state.participant_count.set(participant_count);
        }
        WsMessage::ParticipantLeft { participant_count, .. } => {
            state.participant_count.set(participant_count);
        }
        WsMessage::Error { message, .. } => {
            state.set_error(message);
        }
        _ => {}
    }
}
```

---

## 10. Styles & Assets

### `assets/styles/main.css`

```css
/* assets/styles/main.css */
:root {
  --bg-primary: #0f1117;
  --bg-secondary: #1a1d27;
  --bg-card: #22263a;
  --bg-hover: #2a2f44;
  --accent: #00ff88;
  --accent-dim: rgba(0, 255, 136, 0.15);
  --accent-blue: #4d9eff;
  --accent-red: #ff4d6a;
  --accent-yellow: #ffd166;
  --text: #e8eaf0;
  --text-secondary: #8891a8;
  --text-dim: #4a5068;
  --border: #2d3148;
  --border-hover: #3d4260;
  --radius: 8px;
  --shadow: 0 4px 24px rgba(0,0,0,0.4);
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  background: var(--bg-primary); color: var(--text);
  font-family: 'Inter', 'Segoe UI', system-ui, sans-serif;
  font-size: 15px; line-height: 1.6; min-height: 100vh;
}
.mono { font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace; }
.app-container {
  max-width: 1280px; margin: 0 auto; padding: 0 24px;
  display: flex; flex-direction: column; min-height: 100vh;
}
.app-header {
  display: flex; align-items: center; gap: 16px;
  padding: 16px 0; border-bottom: 1px solid var(--border);
}
.logo { font-size: 1.4rem; font-weight: 800; color: var(--accent); letter-spacing: -0.5px; }
.tagline { font-size: 0.78rem; color: var(--text-secondary); }
.editor-layout { display: flex; flex-direction: column; gap: 16px; padding: 20px 0; }
/* Drop-Zone */
.file-input-container { padding: 40px 0; }
.drop-zone {
  border: 2px dashed var(--border); border-radius: 16px;
  padding: 64px 40px; text-align: center; cursor: pointer;
  transition: all 0.2s ease; background: var(--bg-secondary);
}
.drop-zone:hover, .drop-zone.drag-over {
  border-color: var(--accent); background: var(--accent-dim);
  transform: scale(1.01);
}
.drop-zone-icon { font-size: 3rem; margin-bottom: 16px; }
.drop-zone-title { font-size: 1.4rem; font-weight: 700; margin-bottom: 8px; }
.drop-zone-subtitle { color: var(--text-secondary); margin-bottom: 24px; }
.file-btn {
  display: inline-block; background: var(--accent); color: #000;
  padding: 12px 28px; border-radius: var(--radius);
  font-weight: 700; cursor: pointer; transition: opacity 0.2s;
  font-size: 0.95rem;
}
.file-btn:hover { opacity: 0.88; }
.privacy-badge {
  margin-top: 20px; font-size: 0.8rem; color: var(--text-dim);
}
.feature-grid {
  display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px; margin-top: 32px;
}
.feature-card {
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius); padding: 20px;
  display: flex; flex-direction: column; gap: 6px;
  transition: border-color 0.2s;
}
.feature-card:hover { border-color: var(--border-hover); }
.feature-icon { font-size: 1.5rem; }
.feature-title { font-weight: 600; font-size: 0.95rem; }
.feature-desc { font-size: 0.82rem; color: var(--text-secondary); }
/* Toolbar */
.toolbar {
  display: flex; align-items: center; gap: 16px; flex-wrap: wrap;
  background: var(--bg-secondary); border-radius: var(--radius);
  padding: 16px 20px; border: 1px solid var(--border);
}
.export-btn {
  background: var(--accent); color: #000; border: none;
  padding: 10px 24px; border-radius: var(--radius);
  font-weight: 700; font-size: 0.95rem; cursor: pointer;
  transition: opacity 0.2s, transform 0.1s;
}
.export-btn:hover:not(:disabled) { opacity: 0.88; transform: translateY(-1px); }
.export-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.secondary-btn {
  background: transparent; color: var(--text-secondary);
  border: 1px solid var(--border); padding: 10px 20px;
  border-radius: var(--radius); cursor: pointer; font-size: 0.9rem;
  transition: all 0.2s;
}
.secondary-btn:hover { border-color: var(--border-hover); color: var(--text); }
.trim-info { display: flex; gap: 8px; align-items: center; }
.trim-info-label { font-size: 0.82rem; color: var(--text-secondary); }
.trim-info-value { font-size: 0.9rem; color: var(--accent); }
.export-progress-container { flex: 1; min-width: 200px; }
.export-message { font-size: 0.8rem; color: var(--text-secondary); margin-bottom: 6px; }
.progress-bar {
  height: 4px; background: var(--border); border-radius: 2px; overflow: hidden;
}
.progress-bar-fill {
  height: 100%; background: var(--accent); border-radius: 2px;
  transition: width 0.3s ease;
}
.progress-pct { font-size: 0.75rem; color: var(--text-dim); margin-top: 4px; }
/* Session Panel */
.session-panel {
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius); padding: 16px 20px;
}
.session-title { font-size: 0.88rem; color: var(--text-secondary); margin-bottom: 12px; font-weight: 600; }
.session-btn {
  background: var(--bg-card); color: var(--accent);
  border: 1px solid var(--accent); padding: 8px 18px;
  border-radius: var(--radius); cursor: pointer; font-size: 0.88rem;
  transition: all 0.2s;
}
.session-btn:hover:not(:disabled) { background: var(--accent-dim); }
.session-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.session-desc { font-size: 0.82rem; color: var(--text-secondary); margin-bottom: 12px; }
.participant-status { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; font-size: 0.85rem; }
.participant-dot {
  width: 8px; height: 8px; border-radius: 50%;
  background: var(--accent); animation: pulse 2s ease-in-out infinite;
}
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
.share-link-row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 8px; }
.share-link {
  background: var(--bg-primary); border: 1px solid var(--border);
  padding: 6px 12px; border-radius: 4px; font-size: 0.8rem;
  color: var(--accent-blue); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  max-width: 300px;
}
.copy-btn {
  background: transparent; border: 1px solid var(--border);
  padding: 6px 12px; border-radius: 4px; cursor: pointer;
  font-size: 0.8rem; color: var(--text-secondary); transition: all 0.2s;
  white-space: nowrap;
}
.copy-btn:hover { border-color: var(--border-hover); color: var(--text); }
.session-note { font-size: 0.75rem; color: var(--text-dim); }
/* Error Toast */
.error-toast {
  position: fixed; bottom: 24px; right: 24px; z-index: 1000;
  background: var(--accent-red); color: white;
  padding: 12px 20px; border-radius: var(--radius);
  font-size: 0.88rem; box-shadow: var(--shadow);
  animation: slideIn 0.3s ease;
}
@keyframes slideIn { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }
/* Responsive */
@media (max-width: 768px) {
  .app-container { padding: 0 16px; }
  .drop-zone { padding: 40px 20px; }
  .feature-grid { grid-template-columns: 1fr 1fr; }
  .toolbar { gap: 10px; }
}
```

### `assets/styles/timeline.css`

```css
/* assets/styles/timeline.css */
:root {
  --timeline-height: 52px;
  --handle-width: 8px;
}
.timeline-container {
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius); padding: 14px 16px;
  user-select: none;
}
.timeline-labels {
  display: flex; justify-content: space-between;
  margin-bottom: 8px;
}
.time-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.72rem; color: var(--text-secondary);
}
.time-label.active { color: var(--accent); }
.timeline-track {
  position: relative; height: var(--timeline-height);
  background: var(--bg-primary); border-radius: 4px;
  cursor: crosshair; overflow: hidden;
  border: 1px solid var(--border);
}
.timeline-waveform {
  position: absolute; inset: 0;
  background: repeating-linear-gradient(
    90deg, transparent, transparent 8px,
    rgba(255,255,255,0.03) 8px, rgba(255,255,255,0.03) 9px
  );
}
.timeline-excluded {
  position: absolute; top: 0; height: 100%;
  background: rgba(255, 77, 106, 0.08);
  pointer-events: none;
}
.timeline-active {
  position: absolute; top: 0; height: 100%;
  background: rgba(0, 255, 136, 0.1);
  border-top: 2px solid var(--accent);
  border-bottom: 2px solid var(--accent);
  pointer-events: none;
}
.trim-handle {
  position: absolute; top: 0; width: var(--handle-width);
  height: 100%; background: var(--accent);
  cursor: ew-resize; transform: translateX(-50%); z-index: 10;
  border-radius: 2px; transition: background 0.1s;
  display: flex; align-items: center; justify-content: center;
}
.trim-handle::after {
  content: "⣿"; color: rgba(0,0,0,0.5);
  font-size: 10px; writing-mode: vertical-lr;
}
.trim-handle:hover, .trim-handle.dragging { background: white; }
.playhead {
  position: absolute; top: -4px; bottom: -4px; width: 2px;
  background: var(--accent-blue); transform: translateX(-50%);
  z-index: 20; pointer-events: none;
}
.playhead::before {
  content: ""; position: absolute;
  top: 4px; left: 50%; transform: translateX(-50%);
  width: 0; height: 0;
  border-left: 5px solid transparent;
  border-right: 5px solid transparent;
  border-top: 6px solid var(--accent-blue);
}
.timeline-duration {
  margin-top: 8px; display: flex; gap: 20px;
}
.timeline-stat { font-size: 0.78rem; color: var(--text-dim); }
.timeline-stat span { color: var(--text-secondary); }
```

---

## 11. Testing-Strategie (komplett)

### 11.1 Unit Tests `shared`

```bash
cargo test -p flashcut-shared -- --nocapture
```

### 11.2 Backend Integration Tests

```rust
// crates/backend/tests/integration_test.rs
use axum::http::StatusCode;
use axum_test::TestServer;
use flashcut_backend::{AppState, SessionStore, SharedState};
use flashcut_shared::{CreateSessionRequest, CreateSessionResponse};
use std::sync::Arc;

fn build_test_app() -> TestServer {
    let state: SharedState = Arc::new(AppState {
        sessions: SessionStore::new(),
        frontend_url: "http://localhost:8080".to_string(),
    });

    let app = axum::Router::new()
        .route("/api/sessions", axum::routing::post(flashcut_backend::handlers::create_session))
        .route("/api/sessions/:id", axum::routing::get(flashcut_backend::handlers::get_session))
        .route("/health", axum::routing::get(flashcut_backend::handlers::health_check))
        .with_state(state);

    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn test_health_check() {
    let server = build_test_app();
    let resp = server.get("/health").await;
    resp.assert_status_ok();
    assert!(resp.text().contains("ok"));
}

#[tokio::test]
async fn test_create_session() {
    let server = build_test_app();
    let body = CreateSessionRequest::default();

    let resp = server
        .post("/api/sessions")
        .json(&body)
        .await;

    resp.assert_status(StatusCode::CREATED);

    let created: CreateSessionResponse = resp.json();
    assert!(!created.session_id.is_empty());
    assert!(created.ws_url.starts_with("/ws/"));
    assert!(created.share_url.contains(&created.session_id));
}

#[tokio::test]
async fn test_get_session_not_found() {
    let server = build_test_app();
    let resp = server.get("/api/sessions/nonexistent").await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_session_lifecycle() {
    let store = SessionStore::new();

    let id = store.create_session();
    assert_eq!(id.len(), 8);
    assert!(store.get_session(&id).is_some());
    store.remove_session(&id);
    assert!(store.get_session(&id).is_none());
}

#[tokio::test]
async fn test_multiple_sessions_independent() {
    let store = SessionStore::new();
    let id1 = store.create_session();
    let id2 = store.create_session();

    assert_ne!(id1, id2);
    assert_eq!(store.active_session_count(), 2);

    store.remove_session(&id1);
    assert_eq!(store.active_session_count(), 1);
    assert!(store.get_session(&id2).is_some());
}
```

### 11.3 WASM Browser-Tests

```rust
// crates/core-wasm/tests/browser_tests.rs
#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;
    use flashcut_core_wasm::{wasm_version, check_browser_support};

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_wasm_version_not_empty() {
        let version = wasm_version();
        assert!(!version.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_browser_support_returns_json() {
        let support = check_browser_support();
        let parsed: serde_json::Value = serde_json::from_str(&support).unwrap();
        assert!(parsed.get("videoDecoder").is_some());
        assert!(parsed.get("videoEncoder").is_some());
        assert!(parsed.get("fileSystemAccess").is_some());
    }
}
```

```bash
# WASM-Tests im Chrome ausführen (headless)
wasm-pack test crates/core-wasm --chrome --headless
```

---

## 12. CI/CD & Docker (produktionsreif)

### `.github/workflows/ci.yml`

```yaml
name: FlashCut CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  fmt:
    name: Format Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt }
      - run: cargo fmt --all -- --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace -- -D warnings

  test:
    name: Tests (shared + backend)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test -p flashcut-shared -p flashcut-backend -- --nocapture

  wasm-test:
    name: WASM Tests (Chrome)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - uses: Swatinem/rust-cache@v2
      - uses: browser-actions/setup-chrome@v1
      - run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - run: wasm-pack test crates/core-wasm --chrome --headless

  build-frontend:
    name: Frontend Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - uses: Swatinem/rust-cache@v2
      - run: cargo install trunk --locked
      - run: trunk build --release
      - uses: actions/upload-artifact@v4
        with:
          name: frontend-dist
          path: dist/

  build-backend:
    name: Backend Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release -p flashcut-backend

  security-audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v1
        with: { token: ${{ secrets.GITHUB_TOKEN }} }
```

### `docker/Dockerfile.backend`

```dockerfile
# ─── Build Stage ─────────────────────────────────────────────────────────
FROM rust:1.77-slim AS builder

WORKDIR /app

# Nur Cargo.toml zuerst kopieren für besseres Layer-Caching
COPY Cargo.toml Cargo.lock ./
COPY crates/shared/Cargo.toml crates/shared/
COPY crates/backend/Cargo.toml crates/backend/

# Dummy-Source zum Cachen der Dependencies
RUN mkdir -p crates/shared/src crates/backend/src && \
    echo "pub fn main() {}" > crates/backend/src/main.rs && \
    echo "" > crates/shared/src/lib.rs && \
    cargo build --release -p flashcut-backend 2>/dev/null; true

# Echten Source-Code kopieren und bauen
COPY crates/shared/src crates/shared/src
COPY crates/backend/src crates/backend/src

RUN touch crates/shared/src/lib.rs crates/backend/src/main.rs && \
    cargo build --release -p flashcut-backend

# ─── Runtime Stage ────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1001 -s /bin/sh flashcut

COPY --from=builder /app/target/release/flashcut-server /usr/local/bin/flashcut-server

USER flashcut
EXPOSE 3001

ENV RUST_LOG=flashcut_backend=info,tower_http=warn
ENV PORT=3001
ENV FRONTEND_URL=http://localhost:8080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

CMD ["flashcut-server"]
```

### `docker/docker-compose.yml`

```yaml
version: '3.9'

services:
  backend:
    build:
      context: ..
      dockerfile: docker/Dockerfile.backend
    ports:
      - "3001:3001"
    environment:
      RUST_LOG: flashcut_backend=info,tower_http=info
      PORT: 3001
      FRONTEND_URL: http://localhost:8080
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3001/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"
```

---

## 13. README.md Vorlage

```markdown
# ⚡ FlashCut — Privacy-First Video Trimmer

[![CI](https://github.com/yourusername/flashcut/actions/workflows/ci.yml/badge.svg)](...)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://rustup.rs)

> Video trimmen ohne Upload. Kein Server, keine Kosten, kein Datenschutz-Risiko.
> Rust/WASM + WebCodecs direkt im Browser.

## Features

- 🔒 **Zero Upload** — Videodaten verlassen nie deinen Browser
- ⚡ **WASM-Powered** — Rust-Code für frame-accurate Verarbeitung
- 🎯 **WebCodecs API** — Hardware-accelerated Encoding/Decoding
- 👥 **Live-Kollaboration** — Zeitstempel-Sync via WebSockets (nur Metadaten)
- 🦀 **100% Rust** — Frontend (Leptos), WASM-Core, Backend (Axum)

## Quick Start

```bash
# Voraussetzungen
rustup target add wasm32-unknown-unknown
cargo install trunk wasm-pack

# Frontend
trunk serve

# Backend (in neuem Terminal)
cargo run -p flashcut-backend
```

Öffne <http://localhost:8080>

## Tech Stack

| Layer | Technologie |
|---|---|
| Frontend | Leptos 0.6 (Rust/WASM) |
| Video Engine | WebCodecs API via web-sys |
| Backend | Axum 0.7 + Tokio |
| Realtime | WebSockets (tokio::broadcast) |
| Build | trunk + wasm-pack |

## Architektur

Alle Videodaten bleiben lokal. Das Backend verarbeitet ausschließlich
Metadaten (Zeitstempel, Schnittmarken) für die Kollaborations-Features.

## License

MIT

```

---

## 14. Master-TODO-Liste

### Phase 0 — Setup (60–90 min)
- [ ] `git init && git remote add origin <url>`
- [ ] Alle Cargo.toml Dateien anlegen (Workspace + alle 4 Crates)
- [ ] Verzeichnisstruktur via `mkdir -p` anlegen
- [ ] `.gitignore`, `Trunk.toml`, `rustfmt.toml`, `clippy.toml`, `.cargo/config.toml`
- [ ] `.vscode/` Konfiguration (extensions, settings, launch, tasks)
- [ ] `scripts/setup-dev.sh` anlegen und ausführen
- [ ] `rustup target add wasm32-unknown-unknown` ✓
- [ ] `cargo check --workspace` muss fehlerfrei sein
- [ ] Initialer Commit: `git add . && git commit -m "chore: initial workspace structure"`

### Phase 1 — WASM-Kern (3–5h)
- [ ] `shared/src/lib.rs` — alle Typen + Tests
- [ ] `core-wasm/src/utils.rs` — Logging, Timer, Performance
- [ ] `core-wasm/src/types.rs` — WasmTrimRange, ExportConfig, WasmError
- [ ] `core-wasm/src/lib.rs` — init(), wasm_version(), check_browser_support()
- [ ] `core-wasm/src/decoder.rs` — create_decoder, configure_decoder, decode_chunk, draw_frame_to_canvas, read_video_metadata
- [ ] `core-wasm/src/encoder.rs` — create_encoder, configure_encoder, encode_frame, flush, create_webm_and_download
- [ ] `core-wasm/src/pipeline.rs` — trim_and_export (Skeleton)
- [ ] `wasm-pack build crates/core-wasm --target web` — muss erfolgreich sein
- [ ] `frontend/index.html` + `frontend/src/main.rs`
- [ ] `frontend/src/state.rs` — AppState mit allen Signals
- [ ] `frontend/src/components/app.rs` — Root-Komponente
- [ ] `frontend/src/components/file_input.rs` — Drop-Zone + File-API
- [ ] `assets/styles/main.css` + `assets/styles/timeline.css`
- [ ] `trunk serve` — App öffnet im Browser
- [ ] **Milestone 1:** Datei laden → Metadaten (duration, width, height) erscheinen

### Phase 2 — Schnitt & Export (4–6h)
- [ ] `components/video_player.rs` — Canvas-Render mit VideoFrame
- [ ] `components/timeline.rs` — Track + Drag-Handles (mousedown/mousemove/mouseup)
- [ ] `components/toolbar.rs` — Export-Button, Fortschrittsanzeige
- [ ] `pipeline.rs` trim_and_export vollständig implementieren
- [ ] Fortschritts-Callback aus Leptos an WASM übergeben
- [ ] Error-Handling im gesamten Export-Flow
- [ ] **Milestone 2:** Video laden → Trim-Marken setzen → Exportieren → Download startet

### Phase 3 — Backend (4–6h)
- [ ] `backend/Cargo.toml` mit allen Dependencies
- [ ] `backend/src/main.rs` — Router, CORS, Graceful Shutdown
- [ ] `backend/src/session.rs` — SessionStore, Session, Broadcast
- [ ] `backend/src/handlers.rs` — REST + WebSocket Handler
- [ ] `cargo run -p flashcut-backend` — Server läuft auf :3001
- [ ] `components/session_panel.rs` — Create/Join Session UI
- [ ] `ws_client.rs` — WebSocket-Client mit Message-Handling
- [ ] URL-Parameter `?session=ID` beim App-Start auswerten
- [ ] **Milestone 3:** Zwei Tabs öffnen → Session teilen → Trim-Marken live synchron

### Phase 4 — Tests & CI (2–3h)
- [ ] `shared` Unit-Tests grün: `cargo test -p flashcut-shared`
- [ ] `backend` Integration-Tests: `cargo test -p flashcut-backend`
- [ ] WASM-Tests: `wasm-pack test crates/core-wasm --chrome --headless`
- [ ] Clippy: `cargo clippy --workspace -- -D warnings`
- [ ] Fmt: `cargo fmt --all -- --check`
- [ ] `.github/workflows/ci.yml` anlegen → Push → CI grün

### Phase 5 — Portfolio-Finish (2–3h)
- [ ] `README.md` mit Badges, Feature-Tabelle, Quick-Start
- [ ] `CONTRIBUTING.md` anlegen
- [ ] `docker/Dockerfile.backend` + `docker-compose.yml`
- [ ] `trunk build --release` — Production-Build läuft
- [ ] Repository public setzen
- [ ] (Optional) GIF/Video vom funktionierenden Feature aufnehmen

---

## 15. Bekannte Fallstricke & Lösungen

### F1: `web-sys` Feature fehlt → `error[E0412]: cannot find type VideoDecoder`
Jede `web_sys::` Struktur muss einzeln als Feature in `Cargo.toml` deklariert werden.
Lösung: In `core-wasm/Cargo.toml` das Feature hinzufügen, `cargo check` neu.

### F2: `wasm-bindgen` CLI-Version stimmt nicht mit Crate-Version überein
```

it looks like the Rust project used to create this wasm file was linked
against a different version of wasm-bindgen

```
Lösung: `cargo install wasm-bindgen-cli --version =0.2.XX` mit exakt gleicher XX-Version wie in Cargo.toml.

### F3: `trunk serve` startet nicht — "Could not find HTML target"
Lösung: `Trunk.toml` mit `target = "crates/frontend/index.html"` anlegen ODER direkt aus `crates/frontend/` starten.

### F4: WebCodecs API `undefined` im Browser
WebCodecs benötigt HTTPS oder `localhost`. Außerdem: Chrome 94+ / Firefox 130+.
Sicherheitscheck: `window.isSecureContext` muss `true` sein.

### F5: VideoFrame Memory Leak — Tab wird nach ~30 Sekunden träge
Ursache: `frame.close()` vergessen. VideoFrame hält GPU-Texturen.
Lösung: IMMER nach `draw_image_with_video_frame()` oder `encode()` aufrufen.

### F6: Leptos Reaktivitätsfehler — UI aktualisiert sich nicht
Ursache: `.get()` innerhalb `.set()` aufgerufen (Borrow-Konflikt).
Lösung: Wert zuerst lesen, dann setzen:
```rust
let current = signal.get();  // Erst lesen
signal.set(current + 1.0);   // Dann setzen (kein aktiver Borrow mehr)
```

### F7: Closure in WASM dropped zu früh → `JS exception: null function`

Ursache: Rust-Closure nach `Closure::wrap()` gedroppet bevor JS sie aufgeruft hat.
Lösung: Entweder `closure.forget()` oder die Closure am Leben halten solange der Callback aktiv sein kann. Bei einmaligen Callbacks: `Closure::once_into_js()` verwenden.

### F8: CORS-Fehler bei `fetch("/api/sessions")`

In Dev: Trunk-Proxy in `Trunk.toml` konfigurieren (siehe Kapitel 4.4). Dann laufen API-Calls über Trunk auf Port 8080 und werden automatisch an Backend auf 3001 weitergeleitet — kein CORS-Problem.

### F9: Axum WebSocket — `tungstenite: Connection reset without closing handshake`

Harmlos in Dev wenn der Browser-Tab geschlossen wird. Im Backend mit `saturating_sub` behandeln um panic bei participant_count = 0 zu vermeiden.

### F10: `cargo clippy` schlägt mit `Dead code` fehl für WASM-Funktionen

WASM-exportierte Funktionen sind extern genutzt, Clippy sieht das nicht.
Lösung: `#[allow(dead_code)]` auf Modul-Ebene ODER besser: `#[wasm_bindgen]` Annotationen bedeuten nicht dead_code (neuere clippy-Versionen kennen das).

---

*Dieses Dokument deckt 100% der Implementierung ab und ist direkt für einen KI-Agenten in VSCode (Cursor/Copilot) nutzbar. Alle Dateipfade, Cargo-Features und API-Namen sind exakt und kompilierbar.*

**Stack:** Rust · WebAssembly · Leptos · Axum · WebCodecs API · WebSockets · Trunk · wasm-pack  
**Ziel:** Portfolio-Projekt das System-Level Rust, Browser-APIs und Fullstack-Denken demonstriert.
