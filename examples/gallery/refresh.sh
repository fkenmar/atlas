#!/usr/bin/env bash
# Regenerate every committed example map in this gallery.
#
# Run from the repo root after building atlas (`cargo build --release`), or with
# any `atlas` on your PATH:
#
#   ATLAS=target/release/atlas ./examples/gallery/refresh.sh
#
# Each map is generated with a small, fixed budget so the committed output stays
# tiny and diffs are readable. Keep the commands here in sync with the headers
# shown in README.md.
set -euo pipefail

ATLAS="${ATLAS:-atlas}"
cd "$(dirname "$0")"

"$ATLAS" python-service  --budget 400 -o python-service/atlas-map.md
"$ATLAS" typescript-app  --budget 400 -o typescript-app/atlas-map.md
"$ATLAS" mixed-repo      --budget 400 -o mixed-repo/atlas-map.md

echo "Regenerated gallery maps. Review the diff before committing."
