; Claude usage: per-user installer (Inno Setup 6). Build with installer\build.ps1.
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
; No admin, ever: the app reads and writes this user's Claude files.
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\{#AppName}
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
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "ClaudeUsage"; ValueData: """{app}\{#AppExe}"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
; --open: the first run shows the dashboard; later starts stay in the tray.
Filename: "{app}\{#AppExe}"; Parameters: "--open"; Description: "Open Claude Usage now"; Flags: nowait postinstall skipifsilent
; An update from the app itself (/VERYSILENT /RELAUNCH=1): start the new version again.
Filename: "{app}\{#AppExe}"; Flags: nowait skipifnotsilent; Check: Relaunch

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
