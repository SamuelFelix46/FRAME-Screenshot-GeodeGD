function Copy-FrameData {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$SourceDirectory,[Parameter(Mandatory)][string]$DestinationDirectory)
    $taskSource=(Resolve-Path -LiteralPath $SourceDirectory).Path.TrimEnd('\','/')
    $taskDestination=[IO.Path]::GetFullPath($DestinationDirectory).TrimEnd('\','/')
    if($taskSource -eq $taskDestination){return 0}
    if($taskDestination.StartsWith($taskSource+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){
        throw 'The destination must be outside the previous data directory.'
    }
    if((Get-Item -LiteralPath $taskSource).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Linked data directories are not supported.'}
    if(Test-Path -LiteralPath $taskDestination){
        if((Get-Item -LiteralPath $taskDestination).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Linked destination directories are not supported.'}
    }
    New-Item -ItemType Directory -Path $taskDestination -Force | Out-Null
    $taskCopied=0
    $taskQueue=[Collections.Generic.Queue[object]]::new()
    foreach($taskFolder in @('captures','edits','exports','trash')){
        $taskPart=Join-Path $taskSource $taskFolder
        if(Test-Path -LiteralPath $taskPart -PathType Container){$taskQueue.Enqueue(@($taskPart,(Join-Path $taskDestination $taskFolder)))}
    }
    while($taskQueue.Count -gt 0){
        $taskPair=$taskQueue.Dequeue()
        $taskFrom=[string]$taskPair[0];$taskTo=[string]$taskPair[1]
        if((Get-Item -LiteralPath $taskFrom).Attributes -band [IO.FileAttributes]::ReparsePoint){continue}
        if(Test-Path -LiteralPath $taskTo){
            if((Get-Item -LiteralPath $taskTo).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'An import destination is a filesystem link.'}
        }
        New-Item -ItemType Directory -Path $taskTo -Force | Out-Null
        foreach($taskItem in Get-ChildItem -LiteralPath $taskFrom -Force){
            if($taskItem.Attributes -band [IO.FileAttributes]::ReparsePoint){continue}
            $taskNext=Join-Path $taskTo $taskItem.Name
            if($taskItem.PSIsContainer){$taskQueue.Enqueue(@($taskItem.FullName,$taskNext));continue}
            if(-not(Test-Path -LiteralPath $taskNext)){
                Copy-Item -LiteralPath $taskItem.FullName -Destination $taskNext -ErrorAction Stop
                $taskCopied++
            }
        }
    }
    $taskSettings=Join-Path $taskSource 'settings.json'
    $taskNewSettings=Join-Path $taskDestination 'settings.json'
    if((Test-Path -LiteralPath $taskSettings -PathType Leaf) -and -not(Test-Path -LiteralPath $taskNewSettings)){
        if(-not((Get-Item -LiteralPath $taskSettings).Attributes -band [IO.FileAttributes]::ReparsePoint)){
            Copy-Item -LiteralPath $taskSettings -Destination $taskNewSettings -ErrorAction Stop
            $taskCopied++
        }
    }
    return $taskCopied
}
