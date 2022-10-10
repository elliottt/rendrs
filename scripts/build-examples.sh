#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

scenes=( $(grep '\.png' README.md | sed 's/.*(examples\/\(.*\)\.png).*$/..\/scenes\/\1.scene/') )

mkdir -p examples
cd examples

for scene in "${scenes[@]}"; do
  if [ ! -f "$scene" ]; then
    continue
  fi

  cargo run --release -- render "$scene"
done
