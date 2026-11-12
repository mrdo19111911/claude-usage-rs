; Claude usage installer (Inno Setup 6): all users or only me. Build with installer\build.ps1.
; Why these directives: SO_SANH_INNO.md (Zed's [Setup], Greenshot's autostart, ShareX's
; [Code] with our --quit in place of taskkill). Spec: docs/specs/installer.md.

#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#define AppName "Claude Usage"
#define AppExe "claude-usage.exe"

[Setup]
; Never change: upgrades find the installed copy by it.
AppId={{63905BC9-FAEF-420D-8FC1-DF04B915B4AF}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=mrdo19111911
AppPublisherURL=https://github.com/mrdo19111911/claude-usage-rs
; All users by default: the shortcut goes to the common Start menu, which every Windows
; lists (a per-user Start menu shortcut can be left out, see docs/specs/installer.md).
; The first page offers "only for me" (no admin). An upgrade keeps the mode of the copy
; installed (UsePreviousPrivileges, on by default); /ALLUSERS or /CURRENTUSER picks one.
; Only the files are machine-wide: the app still reads and writes each user's own data.
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
; Program Files for all users, %LOCALAPPDATA%\Programs for one user.
DefaultDirName={autopf}\{#AppName}
DisableDirPage=yes
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
OutputDir=..\target\installer
OutputBaseFilename=ClaudeUsage-Setup-{#AppVersion}
; Not AppMutex (checked before PrepareToInstall, so it would ask the user to close the app
; by hand) nor the Restart Manager (a tray app with only hidden windows): see [Code].
CloseApplications=no

[Tasks]
Name: autostart; Description: "Start with Windows (the icon waits in the tray)"

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"

[Registry]
; HKA: HKLM Run (every user's sign-in) for all users, HKCU Run for one user.
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "ClaudeUsage"; ValueData: """{app}\{#AppExe}"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
; runasoriginaluser: the app runs as the user who started Setup, never as the elevated
; admin account (it would read that account's Claude files).
; --open: the first run shows the dashboard; later starts stay in the tray.
Filename: "{app}\{#AppExe}"; Parameters: "--open"; Description: "Open Claude Usage now"; Flags: nowait postinstall skipifsilent runasoriginaluser
; An update from the app itself (/VERYSILENT /RELAUNCH=1): start the new version again.
Filename: "{app}\{#AppExe}"; Flags: nowait skipifnotsilent runasoriginaluser; Check: Relaunch

; User data (%USERPROFILE%\.claude-usage-dashboard, the saved sign-ins) is kept on uninstall.

[Code]
const
  Mutex = 'Local\claude-usage-dashboard';

// Asks the running copy (installed or not) to exit as its Exit menu does; a Claude Desktop
// switch under way finishes first. The exe waits up to a minute for it to end.
procedure QuitRunning(Exe: String);
var
  Rc: Integer;
begin
  if FileExists(Exe) then
    Exec(Exe, '--quit', '', SW_HIDE, ewWaitUntilTerminated, Rc);
end;

function Relaunch(): Boolean;
begin
  Result := ExpandConstant('{param:RELAUNCH|0}') = '1';
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  // The new exe: it can quit a copy started from anywhere (the same window class).
  ExtractTemporaryFile('{#AppExe}');
  QuitRunning(ExpandConstant('{tmp}\{#AppExe}'));
  if CheckForMutexes(Mutex) then
    Result := 'Claude Usage is still running. Right-click its tray icon, choose Exit, then run Setup again.';
end;

function InitializeUninstall(): Boolean;
begin
  QuitRunning(ExpandConstant('{app}\{#AppExe}'));
  Result := not CheckForMutexes(Mutex);
  if not Result then
    MsgBox('Claude Usage is still running. Right-click its tray icon, choose Exit, then uninstall again.', mbError, MB_OK);
end;
