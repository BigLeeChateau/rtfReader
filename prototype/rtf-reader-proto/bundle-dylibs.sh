#!/bin/bash
set -euo pipefail

# Bundle third-party dynamic libraries into the macOS .app bundle so it can run
# on machines without Homebrew or the local deps/ build tree.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_PATH="${1:-$SCRIPT_DIR/src-tauri/target/release/bundle/macos/rtf-reader-proto.app}"

if [[ ! -d "$APP_PATH" ]]; then
    echo "ERROR: app bundle not found: $APP_PATH"
    echo "Usage: $0 [path/to/rtf-reader-proto.app]"
    exit 1
fi

CONTENTS="$APP_PATH/Contents"
FRAMEWORKS="$CONTENTS/Frameworks"
BINARY="$CONTENTS/MacOS/rtf-reader-proto"

mkdir -p "$FRAMEWORKS"

# Libraries to bundle.
DEPS_BUILD="$SCRIPT_DIR/deps/libemf2svg/build/lib"
BREW="/opt/homebrew/opt"

declare -a LIBS=(
    "$DEPS_BUILD/libemf2svg.1.8.1.dylib"
    "$BREW/libpng/lib/libpng16.16.dylib"
    "$BREW/freetype/lib/libfreetype.6.dylib"
    "$BREW/fontconfig/lib/libfontconfig.1.dylib"
    "$BREW/gettext/lib/libintl.8.dylib"
)

# Copy each library (resolving symlinks) into Frameworks.
for src in "${LIBS[@]}"; do
    name="$(basename "$src")"
    dst="$FRAMEWORKS/$name"
    if [[ ! -f "$src" ]]; then
        echo "WARNING: missing $src — skipping"
        continue
    fi
    cp -af "$src" "$dst"
    echo "Copied $name -> Frameworks/"
done

# Re-create the libemf2svg compatibility symlinks so the binary's @rpath
# reference resolves.
if [[ -f "$FRAMEWORKS/libemf2svg.1.8.1.dylib" ]]; then
    (cd "$FRAMEWORKS" && \
        ln -sf libemf2svg.1.8.1.dylib libemf2svg.1.dylib && \
        ln -sf libemf2svg.1.dylib libemf2svg.dylib)
fi

# Update a copied dylib's own ID and its loader references to bundled libs.
update_dylib() {
    local lib="$1"
    local name="$(basename "$lib")"
    install_name_tool -id "@rpath/$name" "$lib" 2>/dev/null || true

    otool -L "$lib" | awk '/^\t/ {print $1}' | while read -r ref; do
        local base="$(basename "$ref")"
        if [[ -f "$FRAMEWORKS/$base" ]]; then
            install_name_tool -change "$ref" "@rpath/$base" "$lib" 2>/dev/null || true
        fi
    done
}

for lib in "$FRAMEWORKS"/*.dylib; do
    [[ -f "$lib" ]] || continue
    update_dylib "$lib"
done

# Rewrite the main binary rpath so it finds the bundled Frameworks.
install_name_tool -delete_rpath "@loader_path/../lib" "$BINARY" 2>/dev/null || true
if ! otool -l "$BINARY" | grep -q '@executable_path/../Frameworks'; then
    install_name_tool -add_rpath "@executable_path/../Frameworks" "$BINARY"
fi

echo "Updated binary rpath:"
otool -l "$BINARY" | grep -A2 LC_RPATH | grep 'path' || true

# Re-sign the app bundle ad-hoc so modified dylibs and the binary remain valid
# on Apple Silicon.
if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "$APP_PATH"
    echo "Re-signed bundle"
fi

echo "Done. Bundled libraries:"
ls -1 "$FRAMEWORKS"
