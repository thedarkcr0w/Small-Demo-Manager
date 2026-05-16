#!/usr/bin/env bash
# Pre-compile JSX → minified JS so the app loads without Babel-in-browser.
# Re-run after editing any .jsx file.
set -euo pipefail
cd "$(dirname "$0")"
for f in icons tweaks-panel details app; do
  npx --yes esbuild@0.24.2 \
    --loader:.jsx=jsx \
    --target=es2020 \
    --minify \
    "${f}.jsx" \
    --outfile="${f}.js"
done
echo "Built: $(ls -1 icons.js tweaks-panel.js details.js app.js | tr '\n' ' ')"
