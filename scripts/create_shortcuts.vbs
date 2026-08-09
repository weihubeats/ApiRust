' RustFox 快捷方式创建脚本（便携版使用）
' 用法: 双击 create_shortcuts.vbs，或在命令行运行 cscript create_shortcuts.vbs
' 会在桌面和开始菜单创建 RustFox 快捷方式。
Option Explicit

Dim fso, shell, wsh
Set fso = CreateObject("Scripting.FileSystemObject")
Set shell = CreateObject("WScript.Shell")
Set wsh = CreateObject("WScript.Shell")

Dim dir, exe, desktop, startmenu
dir = fso.GetParentFolderName(WScript.ScriptFullName)
exe = dir & "\fox-desktop.exe"

If Not fso.FileExists(exe) Then
    MsgBox "未找到 fox-desktop.exe，请把本脚本与 fox-desktop.exe 放在同一目录。", 48, "RustFox"
    WScript.Quit 1
End If

desktop = wsh.SpecialFolders("Desktop")
startmenu = wsh.SpecialFolders("StartMenu") & "\Programs\RustFox"

If Not fso.FolderExists(startmenu) Then
    fso.CreateFolder(startmenu)
End If

CreateShortcut desktop & "\RustFox.lnk", exe, dir
CreateShortcut startmenu & "\RustFox.lnk", exe, dir

MsgBox "已创建 RustFox 桌面快捷方式与开始菜单项。", 64, "RustFox"

Sub CreateShortcut(linkPath, target, workingDir)
    Dim lnk
    Set lnk = wsh.CreateShortcut(linkPath)
    lnk.TargetPath = target
    lnk.WorkingDirectory = workingDir
    lnk.IconLocation = target & ",0"
    lnk.Description = "RustFox API 调试与 Mock 工具"
    lnk.Save
End Sub