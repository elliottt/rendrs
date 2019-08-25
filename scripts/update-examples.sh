#!/bin/bash

set -euo pipefail

for output in examples/*.png; do
  output_file=$(basename "$output")
  scene="scenes/${output_file%.png}.yaml"
  cargo run --release -- "$scene" "$output"
done
