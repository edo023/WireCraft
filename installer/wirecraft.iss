; Installatore di WireCraft per Windows (Inno Setup 6).
; La CI lo compila con: ISCC.exe /DVersione=<versione> installer\wirecraft.iss
; Installa per l'utente corrente senza permessi di amministratore (o per tutti, a scelta).

#ifndef Versione
  #define Versione "0.0.0"
#endif

[Setup]
AppId={{6E2B9C41-7F3A-4D58-9B0E-2A1C5D8F4E73}
AppName=WireCraft
AppVersion={#Versione}
AppVerName=WireCraft {#Versione}
AppPublisher=WireCraft contributors
AppPublisherURL=https://github.com/edo023/WireCraft
AppSupportURL=https://github.com/edo023/WireCraft/issues
DefaultDirName={autopf}\WireCraft
DefaultGroupName=WireCraft
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\wirecraft.exe
OutputDir=..\dist
OutputBaseFilename=wirecraft-setup-{#Versione}
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
WizardStyle=modern
LicenseFile=..\LICENSE-MIT

[Languages]
Name: "it"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\wirecraft.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\WireCraft"; Filename: "{app}\wirecraft.exe"
Name: "{group}\{cm:UninstallProgram,WireCraft}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\WireCraft"; Filename: "{app}\wirecraft.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\wirecraft.exe"; Description: "{cm:LaunchProgram,WireCraft}"; Flags: nowait postinstall skipifsilent
