#!/bin/bash
# Build script for Beep
# Usage:
#   ./build.sh                  Build all targets, all platforms
#   ./build.sh cli              Build CLI only, all platforms
#   ./build.sh gui              Build GUI only, all platforms
#   ./build.sh tui              Build TUI only, all platforms
#   ./build.sh gui windows      Build GUI only, Windows
#   ./build.sh cli linux        Build CLI only, Linux
#   ./build.sh all macos        Build all targets, macOS only

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"

# Validation

if ! command -v rustc &> /dev/null; then
    echo "Error: Rust is not installed. Please install from https://rustup.rs/"
    exit 1
fi

# Platform & target mappings

declare -A PLATFORM_TARGET
PLATFORM_TARGET[windows]="x86_64-pc-windows-msvc"
PLATFORM_TARGET[linux]="x86_64-unknown-linux-gnu"
PLATFORM_TARGET[macos]="x86_64-apple-darwin"

PLATFORMS=(windows linux macos)
TARGETS=(cli gui tui)

# Argument parsing

TARGET="${1:-all}"
PLATFORM="${2:-all}"

if [ "$TARGET" = "all" ]; then
    SELECTED_TARGETS=("${TARGETS[@]}")
elif [[ " ${TARGETS[*]} " =~ " $TARGET " ]]; then
    SELECTED_TARGETS=("$TARGET")
else
    echo "Error: unknown target '$TARGET'"
    echo "Valid targets: all, ${TARGETS[*]}"
    exit 1
fi

if [ "$PLATFORM" = "all" ]; then
    SELECTED_PLATFORMS=("${PLATFORMS[@]}")
elif [[ " ${PLATFORMS[*]} " =~ " $PLATFORM " ]]; then
    SELECTED_PLATFORMS=("$PLATFORM")
else
    echo "Error: unknown platform '$PLATFORM'"
    echo "Valid platforms: all, ${PLATFORMS[*]}"
    exit 1
fi

# Node.js is only required for GUI builds (Tauri frontend)
if [[ " ${SELECTED_TARGETS[*]} " =~ " gui " ]]; then
    if ! command -v node &> /dev/null; then
        echo "Error: Node.js is not installed (required for GUI/Tauri build)."
        echo "Install from https://nodejs.org/"
        exit 1
    fi
    if ! command -v npm &> /dev/null; then
        echo "Error: npm is not installed (required for GUI/Tauri build)."
        exit 1
    fi
fi

GUI_DIR="$SCRIPT_DIR/crates/beep-gui"

# Helpers

ensure_rust_target() {
    local target="$1"
    if ! rustup target list | grep -q "^$target"; then
        echo "Installing target: $target"
        rustup target add "$target"
    fi
}

get_binary_name() {
    local binary="$1"
    local platform="$2"
    if [ "$platform" = "windows" ]; then
        echo "$binary.exe"
    else
        echo "$binary"
    fi
}

get_file_size() {
    local file="$1"
    if [ "$(uname)" = "Darwin" ]; then
        stat -f%z "$file" 2>/dev/null
    else
        stat -c%s "$file" 2>/dev/null
    fi
}

copy_binary() {
    local src="$1"
    local dst="$2"
    local label="$3"

    if [ -f "$src" ]; then
        mkdir -p "$(dirname "$dst")"
        cp "$src" "$dst"
        local size
        size=$(get_file_size "$dst")
        echo "✓ Built: $dst ($size bytes)"
    else
        echo "❌ Build failed for $label"
        exit 1
    fi
}

# Build functions

build_target() {
    local crate="$1"
    local binary="$2"
    local target="$3"
    local output_dir="$4"
    local platform="$5"
    local label="$6"

    echo "Building $label for $platform x64..."

    ensure_rust_target "$target"

    cargo build --release -p "$crate" --target "$target" 2>&1

    local bin_name
    bin_name=$(get_binary_name "$binary" "$platform")
    copy_binary "target/$target/release/$bin_name" "$output_dir/$bin_name" "$label on $platform"
    echo
}

build_gui() {
    local target="$1"
    local output_dir="$2"
    local platform="$3"
    local label="beep-gui (GUI)"

    echo "Building $label for $platform x64 (Tauri)..."

    ensure_rust_target "$target"

    cd "$GUI_DIR"

    # Tauri build handles: npm install -> frontend build -> Rust compile -> bundle
    npx tauri build --target "$target" 2>&1

    cd "$SCRIPT_DIR"

    # Tauri outputs binaries to the workspace target/ (src-tauri is a workspace member)
    local bin_name
    bin_name=$(get_binary_name "beep-gui" "$platform")
    copy_binary \
        "target/$target/release/$bin_name" \
        "$output_dir/$bin_name" \
        "$label on $platform"
    echo
}

# Summary helpers

declare -a BUILT_LIST=()

record_build() {
    local platform="$1"
    local label="$2"
    local binary="$3"
    BUILT_LIST+=("  $platform/$binary ($label)")
}

print_summary() {
    echo "Build Complete!"
    echo "Build artifacts:"

    local last_plat=""
    for entry in "${BUILT_LIST[@]}"; do
        local plat="${entry%%/*}"
        if [ "$plat" != "$last_plat" ]; then
            echo
            echo "  ${plat^}:"
            last_plat="$plat"
        fi
        echo "$entry"
    done
    echo
}

# Build loop

echo "Beep Build Script"
echo "  Targets: ${SELECTED_TARGETS[*]}"
echo "  Platforms: ${SELECTED_PLATFORMS[*]}"
echo "  Rust: $(rustc --version)"
echo

for platform in "${SELECTED_PLATFORMS[@]}"; do
    target="${PLATFORM_TARGET[$platform]}"
    out="$BUILD_DIR/${platform}-x64"

    for t in "${SELECTED_TARGETS[@]}"; do
        case "$t" in
            cli)
                build_target "beep" "beep" "$target" "$out" "$platform" "beep (CLI)"
                record_build "$platform" "CLI" "beep"
                ;;
            gui)
                build_gui "$target" "$out" "$platform"
                record_build "$platform" "GUI" "beep-gui"
                ;;
            tui)
                build_target "beep-tui" "beep-tui" "$target" "$out" "$platform" "beep-tui (TUI)"
                record_build "$platform" "TUI" "beep-tui"
                ;;
        esac
    done
done

print_summary
