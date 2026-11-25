; FontCraft for Windows: one installer for every 64-bit PC. It carries FontCraft built for x64
; (Intel and AMD) and for ARM64 (Snapdragon and other ARM PCs), sees which processor the PC has and
; installs only the one made for it. Compiled with Inno Setup 6.3 or later (the first that knows
; ARM64 PCs) by build-installer.ps1 or the GitHub build, which pass the version:
;   ISCC.exe /DAppVersion=0.1.0 packaging\windows\fontcraft.iss
; Paths are relative to this file; both programs must be built first.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#define Root "..\.."

[Setup]
; The same id as the earlier installers (one per processor), so this one updates them.
AppId={{6C1E2B7A-4D0F-4E5B-9F3A-FC0FC0FC0001}
AppName=FontCraft
AppVersion={#AppVersion}
AppVerName=FontCraft {#AppVersion}
AppPublisher=FontCraft
AppComments=Explora, previsualiza, imprime y activa fuentes
DefaultDirName={autopf}\FontCraft
DefaultGroupName=FontCraft
DisableProgramGroupPage=yes
; Only for this user (no administrator needed) or, if chosen, for everyone.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline dialog
; Windows 10 22H2 (2022) or newer and Windows 11, 64-bit, on x64 or ARM64; a 64-bit program on both.
MinVersion=10.0.19045
ArchitecturesAllowed=x64compatible or arm64
ArchitecturesInstallIn64BitMode=x64compatible or arm64
OutputDir={#Root}\target\dist
OutputBaseFilename=FontCraft-{#AppVersion}-Windows-Setup
SetupIconFile={#Root}\assets\icons\fontcraft.ico
UninstallDisplayIcon={app}\FontCraft.exe
UninstallDisplayName=FontCraft
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; FontCraft keeps running beside the clock: it is closed to update or uninstall it.
CloseApplications=yes
ChangesAssociations=yes

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Exactly one of the two programs is installed, as FontCraft.exe.
Source: "{#Root}\target\x86_64-pc-windows-msvc\release\FontCraft.exe"; DestDir: "{app}"; Check: InstallX64; Flags: ignoreversion
Source: "{#Root}\target\aarch64-pc-windows-msvc\release\FontCraft.exe"; DestDir: "{app}"; Check: InstallArm64; Flags: ignoreversion
Source: "{#Root}\assets\icons\fontcraft.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\assets\icons\fontcraft-library.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\FontCraft"; Filename: "{app}\FontCraft.exe"; Comment: "Explora, previsualiza, imprime y activa fuentes"
Name: "{autodesktop}\FontCraft"; Filename: "{app}\FontCraft.exe"; Tasks: desktopicon

[Registry]
; FontCraft libraries (.fcl) open with FontCraft and show its document icon.
Root: HKA; Subkey: "Software\Classes\.fcl"; ValueType: string; ValueName: ""; ValueData: "FontCraft.Library"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\FontCraft.Library"; ValueType: string; ValueName: ""; ValueData: "FontCraft Library"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\FontCraft.Library\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\fontcraft-library.ico"
Root: HKA; Subkey: "Software\Classes\FontCraft.Library\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\FontCraft.exe"" ""%1"""
; The "start with Windows" entry FontCraft may have added goes when it is uninstalled.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "FontCraft"; Flags: uninsdeletevalue dontcreatekey

[Run]
Filename: "{app}\FontCraft.exe"; Description: "{cm:LaunchProgram,FontCraft}"; Flags: nowait postinstall skipifsilent

[Code]
// The PC's own processor decides: ARM64 PCs get the ARM64 build (they could run the x64 one only
// through emulation, more slowly); every other 64-bit PC gets the x64 build.
function InstallArm64: Boolean;
begin
  Result := Is64BitInstallMode and (ProcessorArchitecture = paArm64);
end;

function InstallX64: Boolean;
begin
  Result := Is64BitInstallMode and (ProcessorArchitecture <> paArm64);
end;
