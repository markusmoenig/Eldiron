#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
artifact_dir="${1:-$repo_root/target/wasm-examples/eldiron-client}"
web_root="$repo_root/docs/static/play"

if [[ ! -f "$artifact_dir/eldiron-client.js" || ! -f "$artifact_dir/eldiron-client_bg.wasm" ]]; then
  echo "WASM client assets were not found in: $artifact_dir" >&2
  echo "Build them with: cargo run-wasm --release --package eldiron-client --bin eldiron-client --build-only" >&2
  exit 1
fi

case "$web_root" in
  "$repo_root"/docs/static/play) ;;
  *)
    echo "Refusing to replace an unexpected web-game directory: $web_root" >&2
    exit 1
    ;;
esac

rm -rf "$web_root"
mkdir -p "$web_root/runtime" "$web_root/hideout" "$web_root/gate" "$web_root/stonefall"

cp "$artifact_dir/eldiron-client.js" "$web_root/runtime/"
cp "$artifact_dir/eldiron-client_bg.wasm" "$web_root/runtime/"

for game in hideout gate stonefall; do
  cp "$repo_root/docs/web-player/index.html" "$web_root/$game/index.html"
done

cp "$repo_root/starters/projects/Hideout2D.eldiron" "$web_root/hideout/game.eldiron"
cp "$repo_root/starters/projects/Gate.eldiron" "$web_root/gate/game.eldiron"
cp "$repo_root/source_projects/stonefall-dungeon/build/stonefall-dungeon.eldiron" \
  "$web_root/stonefall/game.eldiron"

echo "Staged the Eldiron web client and 3 games in $web_root"
