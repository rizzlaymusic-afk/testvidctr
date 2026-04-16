#!/usr/bin/env bash
set -euo pipefail

echo "=== FlashCut Dev-Setup (partial) ==="

# 1. Ensure Rust and toolchain
if ! command -v rustup &> /dev/null; then
  echo "rustup missing — please install Rust from https://rustup.rs"
  exit 1
fi

echo "→ Ensuring wasm target"
rustup target add wasm32-unknown-unknown || true

echo "→ Installing trunk (if missing)"
if ! command -v trunk &> /dev/null; then
  cargo install trunk --locked
fi

echo "→ Installing wasm-bindgen-cli (optional)"
if ! command -v wasm-bindgen &> /dev/null; then
  cargo install wasm-bindgen-cli --locked || true
fi

echo "Setup helper finished. Run 'trunk serve' in crates/frontend to start the dev server."