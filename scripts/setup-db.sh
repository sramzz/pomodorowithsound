#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../src-tauri"
mkdir -p .dev
cargo sqlx database create
cargo sqlx migrate run
echo "dev DB ready at src-tauri/.dev/dev.sqlite"
