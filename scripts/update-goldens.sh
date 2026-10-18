#!/usr/bin/env bash
# Intentionally updates the golden screenshots used by the CI visual regression guard.
# After running, commit docs/screenshots/golden and put [update-goldens] in the commit message.
set -euo pipefail
cd "$(dirname "$0")/.."
rm -rf docs/screenshots/golden
cargo run --release -p snapshot -- docs/screenshots/golden
echo "Goldens updated. Commit them with [update-goldens] in the commit message."
