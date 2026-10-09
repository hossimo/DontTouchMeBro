; Inno Setup script for DontTouchMeBro
; ---------------------------------------------------------------------------
; Per-machine install into Program Files (admin required to install).
;
; Why per-machine: the app's manifest is requireAdministrator, so every launch
; runs elevated. Installing it into a user-writable folder (the old per-user
; %LocalAppData%\Programs layout) would let any non-elevated process running as
; that user swap the exe and have it run as admin on the next launch. Program
; Files is only writable by administrators.
;
; Optional task: "Start automatically when I sign in" registers a Scheduled
; Task (\DontTouchMeBro) that starts the app at sign-in with highest privileges,
; so there is no UAC prompt at every logon. (The HKCU\...\Run key can't be used:
; Windows silently skips requireAdministrator apps listed there.)
;
; Prereq: run build.ps1 first so the published exe exists at
;   ..\dist\win-x64\DontTouchMeBro.exe
;
; Compile:
;   "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\DontTouchMeBro.iss
; (or just run .\make-installer.ps1). Output is written to installer\Output\.
; ---------------------------------------------------------------------------

#define AppName "Don't Touch Me Bro"
#define AppPublisher "Down Right Technical Inc."
#define AppExeName "DontTouchMeBro.exe"
#define AppSource "..\dist\win-x64\DontTouchMeBro.exe"
; Pull the version straight from the built exe so it never drifts.
#define AppVersion GetVersionNumbersString(AppSource)
; A stable AppId keeps upgrades/uninstall tracking consistent across versions.
#define AppGuid "{E6B1FF93-DF37-41C2-947B-24E29F06CAA3}"
; Name of the Scheduled Task created by the "autostart" task.
#define LogonTaskName "DontTouchMeBro"

[Setup]
; "{{" escapes the brace for Inno; the value is still the GUID above.
AppId={#StringChange(AppGuid, '{', '{{')}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
; With PrivilegesRequired=admin, {autopf} resolves to C:\Program Files.
DefaultDirName={autopf}\DontTouchMeBro
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; Per-machine install: the elevated exe must live somewhere only admins can write.
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; The running tray app (elevated) holds the exe open during upgrades. It has no
; unsaved state, so let Restart Manager force it closed rather than failing.
; Don't have Restart Manager relaunch it; the "Launch" checkbox handles that.
CloseApplications=force
RestartApplications=no
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
; Checked by default: a tray toggle is only useful if it's running, and without
; this the user gets a UAC prompt every time they start it by hand. Scoped to
; the account that approved setup's UAC prompt (see CreateLogonTask below).
Name: "autostart"; Description: "Start automatically when I sign in (no UAC prompt at sign-in)"; GroupDescription: "Startup:"

[Files]
Source: "{#AppSource}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start Menu shortcut. In admin install mode {autoprograms} is the all-users
; Start Menu.
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: startmenuicon

[Run]
; Offer to launch after install. Setup is already elevated, so runascurrentuser
; starts the app with setup's elevated token: no second UAC prompt, and no
; ERROR_ELEVATION_REQUIRED (runasoriginaluser would start it un-elevated, which a
; requireAdministrator exe can't do without another prompt). It runs as the same
; account the logon task and the %APPDATA% config belong to.
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent runascurrentuser

[UninstallRun]
; Runs before files are removed. Stop any running (elevated) instance so the exe
; isn't locked, then remove the sign-in task. Both are harmless if there's
; nothing to stop/delete.
Filename: "{sys}\taskkill.exe"; Parameters: "/IM {#AppExeName} /F"; Flags: runhidden; RunOnceId: "StopApp"
Filename: "{sys}\schtasks.exe"; Parameters: "/Delete /TN ""{#LogonTaskName}"" /F"; Flags: runhidden; RunOnceId: "DeleteLogonTask"

[Code]
const
  // Uninstall key written by the old per-user (PrivilegesRequired=lowest)
  // installer. Same AppId, but registered under HKCU instead of HKLM, so the
  // per-machine install doesn't see it as a previous version.
  OldPerUserUninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#AppGuid}_is1';

// Escapes S for XML element text AND makes it pure ASCII: anything above 0x7F
// becomes a numeric character reference (&#NNN;). schtasks /XML rejects UTF-8
// files that carry a BOM or an encoding declaration, and decodes BOM-less files
// with the ANSI code page, so an ASCII-only file is the one form that survives
// non-ASCII user names or install paths intact. (Verified against schtasks.exe
// on Windows 11.)
function XmlEscape(const S: String): String;
var
  I, C, Lo: Integer;
begin
  Result := '';
  I := 1;
  while I <= Length(S) do
  begin
    C := Ord(S[I]);
    if C = Ord('&') then Result := Result + '&amp;'
    else if C = Ord('<') then Result := Result + '&lt;'
    else if C = Ord('>') then Result := Result + '&gt;'
    else if C = Ord('"') then Result := Result + '&quot;'
    else if C = Ord('''') then Result := Result + '&apos;'
    else if C < 128 then Result := Result + S[I]
    else
    begin
      // Combine a UTF-16 surrogate pair into one code point.
      if (C >= $D800) and (C <= $DBFF) and (I < Length(S)) then
      begin
        Lo := Ord(S[I + 1]);
        if (Lo >= $DC00) and (Lo <= $DFFF) then
        begin
          C := $10000 + ((C - $D800) * 1024) + (Lo - $DC00);
          I := I + 1;
        end;
      end;
      Result := Result + '&#' + IntToStr(C) + ';';
    end;
    I := I + 1;
  end;
end;

// DOMAIN\user of the account setup is running as, i.e. the account that
// approved the UAC prompt. Normally that's the signed-in user. With
// over-the-shoulder UAC (a standard user typing an admin's credentials) it's
// the admin account -- which is deliberate: a standard user's "highest
// privileges" token is still non-admin, so a task for them could never start
// this requireAdministrator app. See README "Known limitations".
function TaskUserId: String;
var
  Domain: String;
begin
  Domain := GetEnv('USERDOMAIN');
  if Domain <> '' then
    Result := Domain + '\' + GetUserNameString
  else
    Result := GetUserNameString;
end;

// Task Scheduler definition. Using XML (rather than plain schtasks switches)
// so we can turn off the defaults that would kill or block a long-running
// tray app: the 72-hour ExecutionTimeLimit and the battery restrictions.
// No <?xml ... encoding=...?> declaration on purpose: schtasks fails with
// "unable to switch the encoding" when one is present in a non-UTF-16 file.
function BuildLogonTaskXml(const UserId, ExePath, WorkDir: String): String;
var
  U: String;
begin
  U := XmlEscape(UserId);
  Result :=
    '<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">' + #13#10 +
    '  <RegistrationInfo>' + #13#10 +
    '    <Author>{#AppPublisher}</Author>' + #13#10 +
    '    <Description>Starts DontTouchMeBro (tray device toggle) elevated when this user signs in. Created by the DontTouchMeBro installer; removed on uninstall.</Description>' + #13#10 +
    '  </RegistrationInfo>' + #13#10 +
    '  <Triggers>' + #13#10 +
    '    <LogonTrigger>' + #13#10 +
    '      <Enabled>true</Enabled>' + #13#10 +
    '      <UserId>' + U + '</UserId>' + #13#10 +
    '    </LogonTrigger>' + #13#10 +
    '  </Triggers>' + #13#10 +
    '  <Principals>' + #13#10 +
    '    <Principal id="Author">' + #13#10 +
    '      <UserId>' + U + '</UserId>' + #13#10 +
    '      <LogonType>InteractiveToken</LogonType>' + #13#10 +
    '      <RunLevel>HighestAvailable</RunLevel>' + #13#10 +
    '    </Principal>' + #13#10 +
    '  </Principals>' + #13#10 +
    '  <Settings>' + #13#10 +
    '    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>' + #13#10 +
    '    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>' + #13#10 +
    '    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>' + #13#10 +
    '    <AllowHardTerminate>true</AllowHardTerminate>' + #13#10 +
    '    <StartWhenAvailable>false</StartWhenAvailable>' + #13#10 +
    '    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>' + #13#10 +
    '    <IdleSettings>' + #13#10 +
    '      <StopOnIdleEnd>false</StopOnIdleEnd>' + #13#10 +
    '      <RestartOnIdle>false</RestartOnIdle>' + #13#10 +
    '    </IdleSettings>' + #13#10 +
    '    <AllowStartOnDemand>true</AllowStartOnDemand>' + #13#10 +
    '    <Enabled>true</Enabled>' + #13#10 +
    '    <Hidden>false</Hidden>' + #13#10 +
    '    <RunOnlyIfIdle>false</RunOnlyIfIdle>' + #13#10 +
    '    <WakeToRun>false</WakeToRun>' + #13#10 +
    '    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>' + #13#10 +
    '    <Priority>4</Priority>' + #13#10 +
    '  </Settings>' + #13#10 +
    '  <Actions Context="Author">' + #13#10 +
    '    <Exec>' + #13#10 +
    '      <Command>' + XmlEscape(ExePath) + '</Command>' + #13#10 +
    '      <WorkingDirectory>' + XmlEscape(WorkDir) + '</WorkingDirectory>' + #13#10 +
    '    </Exec>' + #13#10 +
    '  </Actions>' + #13#10 +
    '</Task>';
end;

procedure DeleteLogonTask;
var
  ResultCode: Integer;
begin
  // Fails harmlessly (non-zero exit) when the task doesn't exist.
  Exec(ExpandConstant('{sys}\schtasks.exe'), '/Delete /TN "{#LogonTaskName}" /F',
    '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Log(Format('schtasks /Delete exit code: %d', [ResultCode]));
end;

procedure CreateLogonTask;
var
  XmlFile, UserId, Xml: String;
  ResultCode: Integer;
begin
  UserId := TaskUserId;
  XmlFile := ExpandConstant('{tmp}\DontTouchMeBro-task.xml');
  Xml := BuildLogonTaskXml(UserId, ExpandConstant('{app}\{#AppExeName}'), ExpandConstant('{app}'));

  // XmlEscape guarantees pure ASCII, so the AnsiString write is lossless and
  // produces no BOM -- the form schtasks /XML accepts.
  if not SaveStringToFile(XmlFile, Xml, False) then
  begin
    Log('Could not write task XML to ' + XmlFile);
    SuppressibleMsgBox('Setup could not create the "start at sign-in" task (failed to write its definition). ' +
      'The app is installed; you can start it from the Start Menu.', mbError, MB_OK, IDOK);
    Exit;
  end;

  Log('Creating logon task for ' + UserId);
  if (not Exec(ExpandConstant('{sys}\schtasks.exe'),
        '/Create /TN "{#LogonTaskName}" /XML "' + XmlFile + '" /F',
        '', SW_HIDE, ewWaitUntilTerminated, ResultCode)) or (ResultCode <> 0) then
  begin
    Log(Format('schtasks /Create failed, exit code: %d', [ResultCode]));
    SuppressibleMsgBox('Setup could not create the "start at sign-in" scheduled task for ' + UserId +
      ' (schtasks exit code ' + IntToStr(ResultCode) + '). ' +
      'The app is installed; you can start it from the Start Menu.', mbError, MB_OK, IDOK);
  end;

  DeleteFile(XmlFile);
end;

// Remove a copy installed by the old per-user installer
// (%LocalAppData%\Programs\DontTouchMeBro), otherwise it's left behind with its
// own Start Menu entry and a user-writable exe that still auto-elevates.
//
// Note: setup runs elevated, so HKCU here is the hive of the account that
// approved UAC. In the normal case that's the signed-in user. With
// over-the-shoulder UAC it's the admin's hive, so a standard user's old
// per-user copy isn't found here -- README documents removing it manually.
procedure RemoveOldPerUserInstall;
var
  Uninstaller: String;
  ResultCode: Integer;
begin
  if not RegQueryStringValue(HKCU, OldPerUserUninstallKey, 'UninstallString', Uninstaller) then
    Exit;

  Uninstaller := RemoveQuotes(Uninstaller);
  Log('Found previous per-user install, uninstaller: ' + Uninstaller);
  if not FileExists(Uninstaller) then
  begin
    Log('Previous per-user uninstaller is missing; skipping.');
    Exit;
  end;

  if SuppressibleMsgBox('An older per-user copy of Don''t Touch Me Bro is installed in your user profile. ' +
      'This version installs for all users in Program Files instead.' + #13#10#13#10 +
      'Remove the old copy now? (Recommended. Your device setting is kept.)',
      mbConfirmation, MB_YESNO, IDYES) <> IDYES then
  begin
    Log('User chose to keep the previous per-user install.');
    Exit;
  end;

  // The old copy may be running (elevated); stop it so its files can go.
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM {#AppExeName} /F',
    '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

  if (not Exec(Uninstaller, '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART', '',
        SW_HIDE, ewWaitUntilTerminated, ResultCode)) or (ResultCode <> 0) then
  begin
    Log(Format('Previous per-user uninstall failed, exit code: %d', [ResultCode]));
    SuppressibleMsgBox('The old per-user copy could not be removed automatically (exit code ' +
      IntToStr(ResultCode) + '). Setup will continue; you can remove it later from ' +
      'Settings > Apps.', mbInformation, MB_OK, IDOK);
  end
  else
    Log('Previous per-user install removed.');
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  RemoveOldPerUserInstall;
  Result := '';
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    // Re-running setup reflects the current choice: create/refresh the task
    // when selected, remove a previously created one when not.
    if WizardIsTaskSelected('autostart') then
      CreateLogonTask
    else
      DeleteLogonTask;
  end;
end;
