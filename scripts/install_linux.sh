#!/usr/bin/env bash
# RustFox Linux 安装脚本（解压 tar.gz 包后运行）
#
# 用法:
#   ./install_linux.sh                # 用户级安装（~/.local，无需 sudo）
#   sudo ./install_linux.sh --system  # 系统级安装（/usr/local/share，添加到应用菜单）
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_SRC="$HERE/fox-desktop"
ICON_SRC="$HERE/rustfox.png"
DESKTOP_SRC="$HERE/rustfox.desktop"
[[ -f "$BIN_SRC" ]] || { echo "缺少 fox-desktop，请先运行 scripts/package.sh 或下载完整压缩包" >&2; exit 1; }

if [[ "${1:-}" = "--system" ]]; then
    BIN_DIR="/usr/local/bin"
    APPS_DIR="/usr/share/applications"
    ICON_DIR="/usr/share/icons/hicolor/256x256/apps"
    BIN_PATH="$BIN_DIR/fox-desktop"
else
    BIN_DIR="$HOME/.local/bin"
    APPS_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
    BIN_PATH="$HOME/.local/bin/fox-desktop"
fi

echo "==> 安装到: $BIN_PATH"
mkdir -p "$BIN_DIR" "$APPS_DIR" "$ICON_DIR"
install -m755 "$BIN_SRC" "$BIN_PATH"
install -m644 "$ICON_SRC" "$ICON_DIR/rustfox.png"

DESKTOP="$APPS_DIR/rustfox.desktop"
if [[ "${1:-}" = "--system" ]]; then
    sed "s|@BIN@|$BIN_PATH|; s|Icon=[^ ]*|Icon=rustfox|" "$DESKTOP_SRC" > "$DESKTOP"
else
    sed "s|@BIN@|$BIN_PATH|; s|Icon=[^ ]*|Icon=rustfox|" "$DESKTOP_SRC" > "$DESKTOP"
fi
chmod 644 "$DESKTOP"

echo "==> RustFox 已安装。"
echo "    请到「应用菜单」搜索 RustFox 启动；"
[[ "${1:-}" != "--system" ]] && echo "    或运行: $BIN_PATH"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS_DIR" 2>/dev/null || true