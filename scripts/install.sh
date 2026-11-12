#!/bin/sh
# Tests, builds and installs Myosotis into /Applications, replacing the old version.
# Your cards live in ~/Library/Application Support/dev.myosotis, not in the app, so they
# stay as they are; if the update changes the database, the app backs it up first.
set -e
cd "$(dirname "$0")/.."

cargo test --workspace --quiet
cargo tauri build --bundles app

if pgrep -f "Myosotis.app/Contents/MacOS/myosotis-app" >/dev/null; then
    echo "Quitting Myosotis…"
    osascript -e 'tell application "Myosotis" to quit'
    while pgrep -f "Myosotis.app/Contents/MacOS/myosotis-app" >/dev/null; do sleep 1; done
fi

rm -rf /Applications/Myosotis.app
ditto target/release/bundle/macos/Myosotis.app /Applications/Myosotis.app
echo "Installed. Open Myosotis from Applications."
