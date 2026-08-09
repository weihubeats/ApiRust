Unicode true

!define APP_NAME "RustFox"
!define APP_EXE "fox-desktop.exe"
!define APP_ICON "..\assets\icons\rustfox.ico"
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"

!include "MUI2.nsh"

!ifndef VERSION
!define VERSION "0.0.0"
!endif

Name "${APP_NAME} ${VERSION}"
OutFile "..\dist\RustFox-${VERSION}-setup.exe"
InstallDir "$LOCALAPPDATA\RustFox"
InstallDirRegKey HKCU "Software\${APP_NAME}" ""
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "0.0.0.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${APP_NAME} API 调试与 Mock 工具"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!define MUI_LANGDLL_ALWAYSSHOW
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  !insertmacro MUI_LANGDLL_DISPLAY
FunctionEnd

Section "Install"
  SetOutPath "$INSTDIR"

  File "..\target\release\${APP_EXE}"
  File "..\assets\icons\rustfox.ico"
  File "..\README.md"
  File "..\LICENSE"

  SetOutPath "$INSTDIR\docs"
  File "..\docs\USER_GUIDE.md"

  ; 桌面快捷方式
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\rustfox.ico"
  ; 开始菜单
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\rustfox.ico"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\用户手册.lnk" "$INSTDIR\docs\USER_GUIDE.md"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\卸载 ${APP_NAME}.lnk" "$INSTDIR\Uninstall.exe"

  ; 卸载程序
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\${APP_NAME}" "" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayIcon" "$INSTDIR\rustfox.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "Publisher" "RustFox"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\docs\USER_GUIDE.md"
  RMDir "$INSTDIR\docs"
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\rustfox.ico"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\用户手册.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\卸载 ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  DeleteRegKey HKCU "Software\${APP_NAME}"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
SectionEnd