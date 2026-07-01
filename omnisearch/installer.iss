[Setup]
AppName=omnisearch
AppVersion=1.0.0
DefaultDirName={localappdata}\Programs\omnisearch
DefaultGroupName=omnisearch
UninstallDisplayIcon={app}\omnisearch.exe
SetupIconFile=..\icons\OmniSearchTrans.ico
Compression=lzma2
SolidCompression=yes
OutputDir=setup
OutputBaseFilename=omnisearchsetup
PrivilegesRequired=lowest

[Files]
Source: "target\release\omnisearch.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\uninstall.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\omnisearch"; Filename: "{app}\omnisearch.exe"
Name: "{userdesktop}\omnisearch"; Filename: "{app}\omnisearch.exe"
Name: "{group}\Uninstall omnisearch"; Filename: "{app}\uninstall.exe"

[Run]
Filename: "{app}\omnisearch.exe"; Description: "Launch omnisearch"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill"; Parameters: "/F /IM omnisearch.exe"; Flags: runhidden; RunOnceId: "KillApp"
Filename: "taskkill"; Parameters: "/F /IM hermes.exe"; Flags: runhidden; RunOnceId: "KillHermes"

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\omnisearch"
