; Per user installation. Keep AppId stable across every future version.
#ifndef AppVersion
  #define AppVersion "0.1.7"
#endif

[Setup]
AppId=Noctistt.EmOrbit
AppName=EmOrbit
AppVersion={#AppVersion}
AppPublisher=Noctistt
AppPublisherURL=https://github.com/noctistt/emorbit
AppSupportURL=https://github.com/noctistt/emorbit/issues
AppUpdatesURL=https://github.com/noctistt/emorbit/releases
DefaultDirName={localappdata}\Programs\EmOrbit
DefaultGroupName=EmOrbit
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.22000
SourceDir=..
OutputDir=dist
OutputBaseFilename=EmOrbit-Setup
LicenseFile=LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes
DisableDirPage=yes
UninstallDisplayIcon={app}\emorbit.exe
CloseApplications=yes
RestartApplications=no
SetupMutex=Noctistt.EmOrbit.Setup

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "target\release\emorbit.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "docs\TESTING.md"; DestDir: "{app}\docs"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\EmOrbit\EmOrbit"; Filename: "{app}\emorbit.exe"
Name: "{userprograms}\EmOrbit\Uninstall EmOrbit"; Filename: "{uninstallexe}"
Name: "{userdesktop}\EmOrbit"; Filename: "{app}\emorbit.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\emorbit.exe"; Description: "Launch EmOrbit (Ctrl + Alt + Space opens the wheel)"; Flags: nowait postinstall skipifsilent

[Code]
function CloseRunningEmOrbit: String;
var
  Window: HWND;
  Attempt: Integer;
begin
  Result := '';
  Window := FindWindowByClassName('EmOrbit.Prototype');
  if Window = 0 then Exit;
  { Use EmOrbit's graceful exit message. WM_CLOSE only hides the picker. }
  if not PostMessage(Window, $8000 + 7, 0, 0) then
  begin
    Result := 'Please exit EmOrbit from its tray menu, then try again.';
    Exit;
  end;
  for Attempt := 1 to 100 do
  begin
    if FindWindowByClassName('EmOrbit.Prototype') = 0 then Exit;
    Sleep(50);
  end;
  Result := 'EmOrbit is still running. Please exit it from its tray menu, then try again.';
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := CloseRunningEmOrbit;
end;

function InitializeUninstall: Boolean;
var
  Error: String;
begin
  Error := CloseRunningEmOrbit;
  Result := Error = '';
  if not Result then
    SuppressibleMsgBox(Error, mbError, MB_OK, IDOK);
end;
