param([string]$GamePath='C:\Program Files (x86)\Steam\steamapps\common\Geometry Dash',[string]$PreviousDataDirectory='')
$ErrorActionPreference='Stop'
$taskGame=(Resolve-Path -LiteralPath $GamePath).Path
if (-not (Test-Path -LiteralPath (Join-Path $taskGame 'GeometryDash.exe'))) { throw 'GeometryDash.exe is missing from this directory.' }
if (-not (Test-Path -LiteralPath (Join-Path $taskGame 'Geode.dll'))) { throw 'Install Geode before installing FRAME.' }
$taskRunning=Get-Process -Name GeometryDash -ErrorAction SilentlyContinue
if ($taskRunning) { throw 'Close Geometry Dash before installing FRAME.' }
$taskVersion=(Get-Item -LiteralPath (Join-Path $taskGame 'Geode.dll')).VersionInfo
if ($taskVersion.FileMajorPart -ne 5 -or $taskVersion.FileMinorPart -lt 10 -or ($taskVersion.FileMinorPart -eq 10 -and $taskVersion.FileBuildPart -lt 1)) { throw 'FRAME requires Geode 5.10.1 or a compatible newer 5.x version.' }
$taskSources=@((Join-Path $PSScriptRoot 'dist\zemci.frame.geode'),(Join-Path $PSScriptRoot '..\zemci.frame.geode'))
$taskPackage=$taskSources | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $taskPackage) { throw 'zemci.frame.geode is missing. Build the project or use the release package.' }
Add-Type -AssemblyName System.IO.Compression.FileSystem
function Read-FrameManifest([string]$PackagePath) {
    $taskZip=[IO.Compression.ZipFile]::OpenRead($PackagePath)
    try {
        $taskEntry=$taskZip.GetEntry('mod.json')
        if(-not $taskEntry -or $taskEntry.Length -gt 65536){return $null}
        $taskReader=[IO.StreamReader]::new($taskEntry.Open())
        try{return ($taskReader.ReadToEnd() | ConvertFrom-Json)}finally{$taskReader.Dispose()}
    } finally {$taskZip.Dispose()}
}
$taskNewManifest=Read-FrameManifest $taskPackage
if($taskNewManifest.id -ne 'zemci.frame'){throw 'This package is not zemci.frame.'}
$taskMods=Join-Path $taskGame 'geode\mods'
New-Item -ItemType Directory -Path $taskMods -Force | Out-Null
$taskTarget=Join-Path $taskMods 'zemci.frame.geode'
$taskConfig=Join-Path $taskGame 'geode\config'
$taskData=Join-Path $taskConfig 'zemci.frame'
$taskPrevious=[Collections.Generic.List[object]]::new()
foreach($taskFile in Get-ChildItem -LiteralPath $taskMods -Filter '*.geode' -File){
    try{$taskMetadata=Read-FrameManifest $taskFile.FullName}catch{continue}
    if($taskMetadata.name -like 'FRAME*Screenshot Studio' -and $taskMetadata.id -match '^[a-z0-9_-]+\.[a-z0-9_.-]+$' -and $taskMetadata.id -notlike '*..*'){
        $taskPrevious.Add(@{Path=$taskFile.FullName;Id=$taskMetadata.id})
    }
}
. (Join-Path $PSScriptRoot 'scripts/import-data.ps1')
if($PreviousDataDirectory){
    $taskCount=Copy-FrameData -SourceDirectory $PreviousDataDirectory -DestinationDirectory $taskData
    Write-Host "Imported $taskCount previous data files."
}
foreach ($taskExisting in $taskPrevious) {
    $taskOldData=Join-Path $taskConfig $taskExisting.Id
    if(Test-Path -LiteralPath $taskOldData -PathType Container){
        $taskCount=Copy-FrameData -SourceDirectory $taskOldData -DestinationDirectory $taskData
        Write-Host "Imported $taskCount previous data files."
    }
    $taskBackups=Join-Path $taskConfig 'zemci.frame-install-backups'
    New-Item -ItemType Directory -Path $taskBackups -Force | Out-Null
    $taskBackupName=[IO.Path]::GetFileNameWithoutExtension($taskExisting.Path)+'-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff')+'.geode'
    Copy-Item -LiteralPath $taskExisting.Path -Destination (Join-Path $taskBackups $taskBackupName)
}
Copy-Item -LiteralPath $taskPackage -Destination $taskTarget -Force
foreach($taskExisting in $taskPrevious){
    if($taskExisting.Path -ne $taskTarget){Remove-Item -LiteralPath $taskExisting.Path}
}
Write-Host 'FRAME installed. Start GD: F6 opens the studio, F8 captures.' -ForegroundColor Cyan
