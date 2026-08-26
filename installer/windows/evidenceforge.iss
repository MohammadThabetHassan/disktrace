#define AppName "DiskTrace Recovery"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef SourceDir
  #define SourceDir "."
#endif

[Setup]
AppId={{3D932D2B-3AE9-421C-8C77-565609744BFC}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=DiskTrace Project
DefaultDirName={localappdata}\Programs\DiskTrace Recovery
DefaultGroupName=DiskTrace Recovery
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputBaseFilename=DiskTrace-{#AppVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName=DiskTrace Recovery
UninstallDisplayIcon={app}\bin\evidenceforge-desktop.exe
VersionInfoVersion={#AppVersion}
VersionInfoProductName={#AppName}
VersionInfoDescription=Local-first forensic recovery workspace

[Files]
Source: "{#SourceDir}\bin\*"; DestDir: "{app}\bin"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#SourceDir}\docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#SourceDir}\launch-evidenceforge.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\Start DiskTrace.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\release-manifest.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\SHA256SUMS"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\DiskTrace Recovery"; Filename: "{app}\Start DiskTrace.cmd"; WorkingDir: "{app}"
Name: "{autodesktop}\DiskTrace Recovery"; Filename: "{app}\Start DiskTrace.cmd"; WorkingDir: "{app}"; Tasks: desktopicon
Name: "{autoprograms}\DiskTrace Recovery\Command line"; Filename: "{app}\bin\evidenceforge.exe"; WorkingDir: "{app}"
Name: "{autoprograms}\DiskTrace Recovery\Safety and evidence boundaries"; Filename: "{app}\docs\safety-and-evidence.md"; WorkingDir: "{app}"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Run]
Filename: "{app}\Start DiskTrace.cmd"; Description: "Launch DiskTrace Recovery"; Flags: nowait postinstall skipifsilent

[Code]
function InitializeSetup(): Boolean;
begin
  Result := True;
end;
