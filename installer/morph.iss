; Inno Setup Script for Morph
; Builds standalone single-file Windows installer: MorphSetup-v0.1.0-windows-x64.exe

#define MyAppName "Morph"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Morph Open Source"
#define MyAppURL "https://github.com/Morph"
#define MyAppExeName "morph-ui.exe"
#define MyAppCliName "morph.exe"

[Setup]
AppId={{E68F8425-46D9-4FE7-9F9C-B3061C8B4907}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\Morph
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=MorphSetup-v0.1.0-windows-x64
SetupIconFile=..\assets\logo.ico
UninstallDisplayIcon={app}\assets\logo.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
DisableWelcomePage=no
WizardImageFile=
WizardSmallImageFile=

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addtopath"; Description: "Add Morph to user PATH (for CLI and AI agent access)"; GroupDescription: "System Integration:"; Flags: checkedonce

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\{#MyAppCliName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\logo.ico"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\assets\logo_fluent.png"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; DestName: "README.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName} Editor"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\logo.ico"
Name: "{autodesktop}\{#MyAppName} Editor"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\logo.ico"; Tasks: desktopicon

[Run]
; Automatically register Windows Explorer context menu right after installation
Filename: "{app}\{#MyAppCliName}"; Parameters: "register"; Flags: runhidden; Description: "Register Windows File Explorer context menu"
; Optional launch option on completion
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Automatically unregister Windows Explorer context menu before file removal
Filename: "{app}\{#MyAppCliName}"; Parameters: "unregister"; Flags: runhidden; RunOnceId: "MorphUnregister"

[UninstallDelete]
Type: filesandordirs; Name: "{app}"

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(OrigPath) + ';') = 0;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDir: string;
  OrigPath: string;
  TempDir: string;
begin
  if CurUninstallStep = usUninstall then
  begin
    AppDir := ExpandConstant('{app}');
    if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
    begin
      StringChangeEx(OrigPath, ';' + AppDir, '', True);
      StringChangeEx(OrigPath, AppDir + ';', '', True);
      StringChangeEx(OrigPath, AppDir, '', True);
      RegWriteStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath);
    end;
    
    // Clean up temporary preview thumbnail cache
    TempDir := GetEnv('TEMP');
    if TempDir <> '' then
      DelTree(TempDir + '\morph_thumbs', True, True, True);
  end;
end;
