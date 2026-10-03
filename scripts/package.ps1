param([string]$OutputDirectory=(Join-Path $PSScriptRoot '../dist'),[string]$DllPath=(Join-Path $PSScriptRoot '../target/release/frame_studio.dll'))
$ErrorActionPreference='Stop'
$taskProject=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$taskMeta=Get-Content -LiteralPath (Join-Path $taskProject 'mod.json') -Raw | ConvertFrom-Json
if($taskMeta.id -ne 'zemci.frame' -or $taskMeta.developer -ne 'zemci'){throw 'Incorrect FRAME identity.'}
if($taskMeta.version -notmatch '^\d+\.\d+\.\d+$' -or $taskMeta.geode -ne '5.10.1' -or $taskMeta.gd.win -ne '2.2081'){throw 'Incorrect version metadata.'}
if($taskMeta.name -ne 'FRAME Screenshot Studio' -or $taskMeta.description -match '[\r\n]'){throw 'Incorrect name or short description.'}
if($taskMeta.links.source -ne 'https://github.com/SamuelFelix46/FRAME-Screenshot-GeodeGD'){throw 'Incorrect source URL.'}
if(-not(Test-Path -LiteralPath $DllPath -PathType Leaf)){throw 'The compiled FRAME DLL is missing.'}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$taskPackage=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ($taskMeta.id+'.geode')
if(Test-Path -LiteralPath $taskPackage){Remove-Item -LiteralPath $taskPackage}
Add-Type -AssemblyName System.IO.Compression.FileSystem
$taskZip=[IO.Compression.ZipFile]::Open($taskPackage,[IO.Compression.ZipArchiveMode]::Create)
try {
    [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($taskZip,$DllPath,($taskMeta.id+'.dll')) | Out-Null
    foreach($taskName in @('mod.json','logo.png','about.md','CHANGELOG.md','LICENSE','THIRD_PARTY.md')){
        [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($taskZip,(Join-Path $taskProject $taskName),$taskName) | Out-Null
    }
    [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($taskZip,(Join-Path $taskProject 'vendor/geode-rs/LICENSE'),'licenses/geode-rs.txt') | Out-Null
    foreach($taskFolder in @('font-licenses','dependency-licenses')){
        Get-ChildItem -LiteralPath (Join-Path $taskProject $taskFolder) -File -Recurse | ForEach-Object {
            $taskRelative=$_.FullName.Substring($taskProject.Length+1).Replace('\','/')
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($taskZip,$_.FullName,('licenses/'+$taskRelative)) | Out-Null
        }
    }
} finally {$taskZip.Dispose()}
$taskHash=(Get-FileHash -LiteralPath $taskPackage -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath (Join-Path $OutputDirectory 'SHA256.txt') -Value ($taskHash+'  '+[IO.Path]::GetFileName($taskPackage)) -Encoding ascii
Write-Host "Package created: $taskPackage" -ForegroundColor Cyan
