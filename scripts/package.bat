@echo off
setlocal EnableDelayedExpansion
rem RustFox Windows 打包脚本
rem 用法: scripts\package.bat
rem 产物输出到 dist\：
rem   RustFox-<version>-windows-x86_64.zip   （便携版：exe + 阅读文档 + 图标）
rem   RustFox-<version>-setup.exe            （NSIS 安装包，安装后自动创建开始菜单与桌面快捷方式）

set ROOT=%~dp0..
cd /d "%ROOT%"

rem ---------- 检查 cargo ----------
where cargo >nul 2>nul
if errorlevel 1 (
    echo [错误] 未找到 cargo。请先安装 Rust: https://rustup.rs
    exit /b 1
)

rem ---------- 版本号 ----------
set VERSION=
for /f "usebackq tokens=3" %%v in (`findstr /r /c:"^version" Cargo.toml`) do set VERSION=%%v
if not defined VERSION set VERSION=0.0.0
set VERSION=%VERSION:"=%
echo ==^> RustFox %VERSION%

rem ---------- 构建 ----------
echo ==^> 构建 release 二进制
cargo build --release -p fox-desktop
if errorlevel 1 exit /b 1

set BIN=target\release\fox-desktop.exe
if not exist "%BIN%" (
    echo [错误] 构建失败：未找到 %BIN%
    exit /b 1
)

set DIST=dist\RustFox-%VERSION%-windows-x86_64
if exist "%DIST%" rmdir /s /q "%DIST%"
mkdir "%DIST%"
copy /y "%BIN%" "%DIST%\fox-desktop.exe" >nul
copy /y README.md "%DIST%\README.md" >nul
copy /y LICENSE "%DIST%\LICENSE" >nul
copy /y docs\USER_GUIDE.md "%DIST%\USER_GUIDE.md" >nul
copy /y assets\icons\rustfox.ico "%DIST%\rustfox.ico" >nul
copy /y scripts\create_shortcuts.vbs "%DIST%\create_shortcuts.vbs" >nul

rem ---------- 便携 zip ----------
powershell -NoProfile -Command "Compress-Archive -Path '%DIST%' -DestinationPath 'dist\RustFox-%VERSION%-windows-x86_64.zip' -Force"
if errorlevel 1 (
    echo [警告] zip 打包失败，%DIST% 目录保留
) else (
    echo ==^> 便携包: dist\RustFox-%VERSION%-windows-x86_64.zip
)

rem ---------- NSIS 安装包 ----------
where makensis >nul 2>nul
if errorlevel 1 (
    echo [提示] 未找到 makensis，跳过安装包生成（仅生成便携包）。
    echo        在 CI 中会自动安装 NSIS 生成 setup.exe。
) else (
    echo ==^> 生成 NSIS 安装包
    makensis /DVERSION="%VERSION%" scripts\rustfox.nsi
    if errorlevel 1 (
        echo [警告] NSIS 打包失败
    ) else (
        echo ==^> 安装包: dist\RustFox-%VERSION%-setup.exe
    )
)

echo ==^> 完成。
endlocal