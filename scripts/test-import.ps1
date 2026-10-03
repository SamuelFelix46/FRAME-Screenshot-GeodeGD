$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'import-data.ps1')
$taskTempRoot=[IO.Path]::GetFullPath($env:TEMP)
$taskFixture=[IO.Path]::GetFullPath((Join-Path $taskTempRoot ('frame-import-test-'+[guid]::NewGuid().ToString('N'))))
if(-not $taskFixture.StartsWith($taskTempRoot+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Unsafe test directory.'}
try {
    $taskSource=Join-Path $taskFixture 'previous'
    $taskTarget=Join-Path $taskFixture 'current'
    New-Item -ItemType Directory -Path (Join-Path $taskSource 'captures'),(Join-Path $taskSource 'edits'),(Join-Path $taskSource 'trash/123-1'),(Join-Path $taskTarget 'captures') -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $taskSource 'captures/123-1.png'),'old original bytes')
    [IO.File]::WriteAllText((Join-Path $taskSource 'captures/456-1.png'),'another original')
    [IO.File]::WriteAllText((Join-Path $taskSource 'edits/123-1.json'),'{"annotations":[],"crop":null}')
    [IO.File]::WriteAllText((Join-Path $taskSource 'trash/123-1/original.png'),'recoverable original')
    [IO.File]::WriteAllText((Join-Path $taskSource 'settings.json'),'{"language":"French","capture_key":"Ctrl+P"}')
    [IO.File]::WriteAllText((Join-Path $taskTarget 'captures/123-1.png'),'current original bytes')
    [IO.File]::WriteAllText((Join-Path $taskTarget 'settings.json'),'{"language":"English"}')
    $taskCount=Copy-FrameData -SourceDirectory $taskSource -DestinationDirectory $taskTarget
    if($taskCount -ne 3){throw 'Only missing files should be imported.'}
    if([IO.File]::ReadAllText((Join-Path $taskTarget 'captures/123-1.png')) -ne 'current original bytes'){throw 'An existing original was overwritten.'}
    if([IO.File]::ReadAllText((Join-Path $taskTarget 'settings.json')) -ne '{"language":"English"}'){throw 'Existing settings were overwritten.'}
    if([IO.File]::ReadAllText((Join-Path $taskTarget 'trash/123-1/original.png')) -ne 'recoverable original'){throw 'Trash original was not preserved.'}
    if([IO.File]::ReadAllText((Join-Path $taskSource 'captures/123-1.png')) -ne 'old original bytes'){throw 'Previous data was changed.'}
    if((Copy-FrameData $taskSource $taskTarget) -ne 0){throw 'Repeated imports should not rewrite data.'}
    $taskFresh=Join-Path $taskFixture 'fresh'
    if((Copy-FrameData $taskSource $taskFresh) -ne 5){throw 'A fresh import should include settings.'}
    $taskSettings=Get-Content -LiteralPath (Join-Path $taskFresh 'settings.json') -Raw | ConvertFrom-Json
    if($taskSettings.language -ne 'French' -or $taskSettings.capture_key -ne 'Ctrl+P'){throw 'Previous language or shortcut was lost.'}
    $taskRejected=$false
    try{Copy-FrameData $taskSource (Join-Path $taskSource 'nested') | Out-Null}catch{$taskRejected=$true}
    if(-not $taskRejected){throw 'Import inside the source must be rejected.'}
    Write-Output 'Data import verified: originals, settings, edits, trash, idempotency and safe destination.'
} finally {
    if(Test-Path -LiteralPath $taskFixture){
        $taskResolved=(Resolve-Path -LiteralPath $taskFixture).Path
        if($taskResolved -ne $taskFixture -or -not $taskResolved.StartsWith($taskTempRoot+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Unsafe cleanup path.'}
        Remove-Item -LiteralPath $taskResolved -Recurse -Force
    }
}
