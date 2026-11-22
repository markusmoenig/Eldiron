#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
artifact_dir="${1:-$repo_root/target/wasm-examples/eldiron-client}"
web_root="$repo_root/docs/static/play"

runtime_files=(
  eldiron-client.js
  eldiron-client_bg.wasm
  eldiron-bake-codec-worker-bootstrap.js
  eldiron-bake-codec-worker.js
  eldiron-bake-codec-worker_bg.wasm
)

for file in "${runtime_files[@]}"; do
  if [[ ! -f "$artifact_dir/$file" ]]; then
    echo "WASM client asset missing: $artifact_dir/$file" >&2
    echo "Build the assets with: python3 tools/package_web.py player target/wasm-examples/eldiron-client" >&2
    exit 1
  fi
done

case "$web_root" in
  "$repo_root"/docs/static/play) ;;
  *)
    echo "Refusing to replace an unexpected web-game directory: $web_root" >&2
    exit 1
    ;;
esac

rm -rf "$web_root"
mkdir -p "$web_root/runtime"
for file in "${runtime_files[@]}"; do
  cp "$artifact_dir/$file" "$web_root/runtime/"
done
if [[ -d "$artifact_dir/snippets" ]]; then
  cp -R "$artifact_dir/snippets" "$web_root/runtime/"
fi

for game in hideout wayfarers-rest stonefall gate; do
  mkdir -p "$web_root/$game"
  cp "$repo_root/docs/web-player/index.html" "$web_root/$game/index.html"
  printf '%s\n' 'import "../runtime/eldiron-bake-codec-worker-bootstrap.js";' \
    > "$web_root/$game/eldiron-bake-codec-worker-bootstrap.js"
done

cp "$repo_root/starters/projects/Hideout2D.eldiron" "$web_root/hideout/game.eldiron"
cp "$repo_root/test_projects/Village2DIso.eldiron" "$web_root/wayfarers-rest/game.eldiron"
cp "$repo_root/starters/projects/Gate.eldiron" "$web_root/gate/game.eldiron"
cp "$repo_root/test_projects/StonefallDungeon.eldiron" \
  "$web_root/stonefall/game.eldiron"

echo "Staged the Eldiron web client and 4 games in $web_root"
