#!/usr/bin/env bash
# RustFox 打包脚本（Linux / macOS）
#
# 用法:
#   scripts/package.sh            # 构建 + 打包当前平台
#   RUSTFOX_VERSION=1.2.3 scripts/package.sh   # 指定版本号
#
# 产物输出到 dist/ 目录:
#   Linux:   RustFox-<version>-linux-x86_64.tar.gz
#   macOS:   RustFox-<version>-macos-x86_64.tar.gz / ...-aarch64.tar.gz（内含 RustFox.app）

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
ICONS="$ROOT/assets/icons"
SCRIPTS="$ROOT/scripts"

# ---------- 版本号 ----------
if [[ -n "${RUSTFOX_VERSION:-}" ]]; then
    VERSION="$RUSTFOX_VERSION"
else
    VERSION="$(grep -A3 '^\[workspace.package\]' "$ROOT/Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p')"
fi
[[ -n "$VERSION" ]] || { echo "无法确定版本号" >&2; exit 1; }

echo "==> RustFox $VERSION"

# ---------- 构建 ----------
echo "==> 构建 release 二进制 (target/release/fox-desktop)"
cargo build --release -p fox-desktop
BIN="$ROOT/target/release/fox-desktop"
[[ -x "$BIN" ]] || { echo "构建失败：未找到 $BIN" >&2; exit 1; }

mkdir -p "$DIST"
PKG="$DIST/RustFox-$VERSION"

OS="$(uname -s)"
case "$OS" in
    Linux*)  OS=linux ;;
    Darwin*) OS=macos ;;
    *)       echo "不支持的平台: $OS（Windows 请用 scripts/package.bat）" >&2; exit 1 ;;
esac

ARCH="$(uname -m)"
[[ "$ARCH" == "arm64" ]] && ARCH=aarch64 || true
[[ "$ARCH" = "x86_64" || "$ARCH" = "aarch64" || "$ARCH" = "arm64" ]] || ARCH="$(uname -m)"

# ---------- 打包 ----------
if [[ "$OS" = "linux" ]]; then
    echo "==> 打包 Linux (tar.gz)"
    STAGE="$DIST/stage-linux"
    rm -rf "$STAGE"
    mkdir -p "$STAGE"

    install -Dm755 "$BIN"                          "$STAGE/fox-desktop"
    install -Dm644 "$ROOT/README.md"               "$STAGE/README.md"
    install -Dm644 "$ROOT/LICENSE"                 "$STAGE/LICENSE"
    install -Dm644 "$ROOT/docs/USER_GUIDE.md"      "$STAGE/USER_GUIDE.md"
    install -Dm644 "$ROOT/assets/icons/rustfox-256.png" "$STAGE/rustfox.png"
    install -Dm644 "$SCRIPTS/rustfox.desktop"      "$STAGE/rustfox.desktop"
    install -Dm755 "$SCRIPTS/install_linux.sh"     "$STAGE/install_linux.sh"
    TARBALL="$DIST/RustFox-$VERSION-linux-$ARCH.tar.gz"
    tar -C "$STAGE" -czf "$TARBALL" .
    echo "==> 产物: $TARBALL"

elif [[ "$OS" = "macos" ]]; then
    echo "==> 打包 macOS (.app bundle)"
    STAGE="$DIST/RustFox-$VERSION-mac"
    APP="$STAGE/RustFox.app"
    rm -rf "$STAGE"
    mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
    cp "$BIN" "$APP/Contents/MacOS/fox-desktop"
    cp "$ROOT/assets/icons/rustfox.icns" "$APP/Contents/Resources/rustfox.icns"
    sed "s/__VERSION__/$VERSION/g" "$SCRIPTS/macos/Info.plist" > "$APP/Contents/Info.plist"
    chmod +x "$APP/Contents/MacOS/fox-desktop"
    if command -v codesign >/dev/null 2>&1; then
        codesign --force --deep -s - "$APP" 2>/dev/null || true
    fi
    cp "$ROOT/README.md" "$STAGE/README.md"
    cp "$ROOT/docs/USER_GUIDE.md" "$STAGE/USER_GUIDE.md"
    ZBALL="$DIST/RustFox-$VERSION-macos-$ARCH.zip"
    (cd "$STAGE" && zip -qr "$ZBALL" RustFox.app README.md USER_GUIDE.md)
    rm -rf "$STAGE"
    echo "==> 产物: $ZBALL（内含 RustFox.app + README + 使用手册）"
    echo "    macOS 用户直接双击 RustFox.app，或拖入 /Applications"
fi

echo "==> 完成。所有产物在: $DIST/"