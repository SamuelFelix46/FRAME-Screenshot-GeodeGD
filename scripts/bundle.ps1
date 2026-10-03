param([string]$PackageDirectory=(Join-Path $PSScriptRoot '../dist'),[string]$OutputDirectory=(Join-Path $PSScriptRoot '../dist'))
$ErrorActionPreference='Stop'
$taskProject=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$taskMeta=Get-Content -LiteralPath (Join-Path $taskProject 'mod.json') -Raw | ConvertFrom-Json
$taskPackage=Join-Path ([IO.Path]::GetFullPath($PackageDirectory)) 'zemci.frame.geode'
if(-not(Test-Path -LiteralPath $taskPackage -PathType Leaf)){throw 'The tested .geode package is missing.'}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$taskOutput=[IO.Path]::GetFullPath($OutputDirectory)
$taskSourceName='FRAME-source-'+$taskMeta.version+'.zip'
$taskBundleName='FRAME-'+$taskMeta.version+'.zip'
$taskSourceZip=Join-Path $taskOutput $taskSourceName
$taskBundleZip=Join-Path $taskOutput $taskBundleName
Push-Location $taskProject
try{
    $taskChanges=git status --porcelain
    if($LASTEXITCODE -ne 0 -or $taskChanges){throw 'Commit all source changes before assembling a release.'}
    git archive --format=zip --output=$taskSourceZip HEAD
    if($LASTEXITCODE -ne 0){throw 'Could not archive the committed source.'}
}finally{Pop-Location}
$taskCopy=Join-Path $taskOutput 'zemci.frame.geode'
if($taskCopy -ne $taskPackage){Copy-Item -LiteralPath $taskPackage -Destination $taskCopy -Force}
$taskPackageHash=(Get-FileHash -LiteralPath $taskCopy -Algorithm SHA256).Hash.ToLowerInvariant()
Add-Type -AssemblyName System.IO.Compression.FileSystem
$taskModZip=[IO.Compression.ZipFile]::OpenRead($taskCopy)
try{
    $taskManifestEntry=$taskModZip.GetEntry('mod.json')
    if(-not $taskManifestEntry -or $taskManifestEntry.Length -gt 65536){throw 'Package manifest missing or oversized.'}
    $taskReader=[IO.StreamReader]::new($taskManifestEntry.Open())
    try{$taskPackagedMeta=$taskReader.ReadToEnd() | ConvertFrom-Json}finally{$taskReader.Dispose()}
    if($taskPackagedMeta.id -ne $taskMeta.id -or $taskPackagedMeta.version -ne $taskMeta.version -or $taskPackagedMeta.developer -ne 'zemci' -or -not $taskModZip.GetEntry('zemci.frame.dll')){throw 'The tested package does not match this source version.'}
}finally{$taskModZip.Dispose()}
if(Test-Path -LiteralPath $taskBundleZip){Remove-Item -LiteralPath $taskBundleZip}
$taskZip=[IO.Compression.ZipFile]::Open($taskBundleZip,[IO.Compression.ZipArchiveMode]::Create)
$taskSource=[IO.Compression.ZipFile]::OpenRead($taskSourceZip)
function Write-BundleText([string]$Name,[string]$Text){
    $taskWriter=[IO.StreamWriter]::new($taskZip.CreateEntry($Name).Open(),[Text.UTF8Encoding]::new($false))
    try{$taskWriter.Write($Text)}finally{$taskWriter.Dispose()}
}
try{
    [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($taskZip,$taskCopy,'zemci.frame.geode') | Out-Null
    Write-BundleText 'SHA256.txt' ($taskPackageHash+"  zemci.frame.geode`n")
    Write-BundleText 'INSTALLATION.txt' @"
FRAME Screenshot Studio $($taskMeta.version) by zemci
Windows x64 / GD 2.2081 / Geode 5.10.1 or compatible 5.x

Close Geometry Dash. Copy zemci.frame.geode into Geometry Dash/geode/mods.
F6 opens the studio. F8 captures. English is the default; French is included.

Updating an earlier FRAME installation:
Run source/install.ps1 in PowerShell before launching the new version.
It imports missing old data, backs up old FRAME packages and installs one mod.
Use -GamePath for another game location, or -PreviousDataDirectory for a backup.
Previous data and existing destination files remain intact.

See source/README.md or source/README-fr.md for usage and build instructions.
"@
    foreach($taskEntry in $taskSource.Entries){
        $taskNew=$taskZip.CreateEntry('source/'+$taskEntry.FullName)
        if($taskEntry.FullName.EndsWith('/')){continue}
        $taskInput=$taskEntry.Open();$taskDestination=$taskNew.Open()
        try{$taskInput.CopyTo($taskDestination)}finally{$taskInput.Dispose();$taskDestination.Dispose()}
    }
}finally{$taskSource.Dispose();$taskZip.Dispose()}
$taskHashes=foreach($taskName in @('zemci.frame.geode',$taskSourceName,$taskBundleName)){
    (Get-FileHash -LiteralPath (Join-Path $taskOutput $taskName) -Algorithm SHA256).Hash.ToLowerInvariant()+'  '+$taskName
}
Set-Content -LiteralPath (Join-Path $taskOutput 'SHA256.txt') -Value $taskHashes -Encoding ascii
Write-Host "Clean release files created in $taskOutput" -ForegroundColor Cyan
