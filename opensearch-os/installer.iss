[Setup]
AppName=OpenSearch OS
AppVersion=0.1.0
DefaultDirName={localappdata}\Programs\OpenSearch OS
DefaultGroupName=OpenSearch OS
UninstallDisplayIcon={app}\opensearch-os.exe
Compression=lzma2
SolidCompression=yes
OutputDir=setup
OutputBaseFilename=OpenSearchOSSetup
PrivilegesRequired=lowest

[Files]
Source: "target\release\opensearch-os.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\model_int8.onnx"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\DirectML.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\OpenSearch OS"; Filename: "{app}\opensearch-os.exe"
Name: "{userstartup}\OpenSearch OS"; Filename: "{app}\opensearch-os.exe"
Name: "{userdesktop}\OpenSearch OS"; Filename: "{app}\opensearch-os.exe"

[Run]
Filename: "{app}\opensearch-os.exe"; Description: "Launch OpenSearch OS"; Flags: nowait postinstall skipifsilent
