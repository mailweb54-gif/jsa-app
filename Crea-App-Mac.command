#!/bin/bash
# JSA Channels - crea l'app per Mac (doppio clic dal Finder).
# Serve una sola volta su UN Mac: il .dmg che esce funziona su tutti i Mac (Intel e Apple Silicon).
# Durata: la prima volta 10-20 minuti (scarica gli strumenti), poi pochi minuti.
set -e
cd "$(dirname "$0")"
echo "=== JSA Channels: creazione app per Mac ==="

# 1) Strumenti Apple (compilatore)
if ! xcode-select -p >/dev/null 2>&1; then
  echo "Installo gli Strumenti da riga di comando di Apple: conferma nella finestra che si apre,"
  echo "aspetta la fine dell'installazione e poi rilancia questo file."
  xcode-select --install || true
  read -n 1 -s -r -p "Premi un tasto per chiudere"; exit 0
fi

# 2) Rust
if ! command -v cargo >/dev/null 2>&1; then
  [ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "Installo Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  source "$HOME/.cargo/env"
fi
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# 3) Strumento Tauri
if ! cargo tauri --version >/dev/null 2>&1; then
  echo "Installo lo strumento Tauri (qualche minuto)..."
  cargo install tauri-cli --version "^2" --locked
fi

# 4) Compilazione (app unica per Mac Intel e Apple Silicon)
cargo tauri build --target universal-apple-darwin --bundles dmg

OUT="src-tauri/target/universal-apple-darwin/release/bundle/dmg"
DMG=$(ls -t "$OUT"/*.dmg | head -1)
cp "$DMG" "$HOME/Desktop/JSA-Channels-Mac.dmg"
echo
echo "FATTO: sulla Scrivania c'e' JSA-Channels-Mac.dmg"
echo "Aprilo e trascina JSA Channels in Applicazioni."
open -R "$HOME/Desktop/JSA-Channels-Mac.dmg"
read -n 1 -s -r -p "Premi un tasto per chiudere"
