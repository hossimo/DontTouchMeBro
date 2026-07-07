; Inno Setup script for DontTouchMeBro
; ---------------------------------------------------------------------------
; Per-user install (no admin required to install), adds a Start Menu shortcut.
;
; Prereq: run build.ps1 first so the published exe exists at
;   ..\dist\win-x64\DontTouchMeBro.exe
;
; Compile:
;   "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\DontTouchMeBro.iss
; Output installer is written to installer\Output\.
; ---------------------------------------------------------------------------

#define AppName "Don't Touch Me Bro"
#define AppPublisher "Down Right Technical Inc."
#define AppExeName "DontTouchMeBro.exe"
#define AppSource "..\dist\win-x64\DontTouchMeBro.exe"
; Pull the version straight from the built exe so it never drifts.
#define AppVersion GetFileVersion(AppSource)

[Setup]
; A stable AppId keeps upgrades/uninstall tracking consistent across versions.
AppId={{E6B1FF93-DF37-41C2-947B-24E29F06CAA3}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\DontTouchMeBro
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; Install for the current user only; no elevation needed to install.
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=Output
OutputBaseFilename=DontTouchMeBro-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#AppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startmenuicon"; Description: "Create a Start Menu shortcut"; GroupDescription: "Shortcuts:"

[Files]
Source: "{#AppSource}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start Menu shortcut (under the current user's Programs when non-elevated).
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: startmenuicon

[Run]
; Offer to launch after install. The app requires admin, so this shows a UAC
; prompt (runascurrentuser keeps it tied to the installing user's session).
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent runascurrentuser
