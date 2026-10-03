param([switch]$SkipTests)
$ErrorActionPreference='Stop'
$taskProject=$PSScriptRoot
if(-not(Get-Command cargo -ErrorAction SilentlyContinue)){throw 'Install Rust stable x64 MSVC with rustfmt: https://rustup.rs/'}
if(-not $env:LIBCLANG_PATH){
    foreach($taskCandidate in @('C:\Program Files\LLVM\bin',(Join-Path $env:USERPROFILE 'scoop\apps\llvm\current\bin'))){
        if(Test-Path -LiteralPath (Join-Path $taskCandidate 'libclang.dll')){$env:LIBCLANG_PATH=$taskCandidate;break}
    }
}
if(-not $env:LIBCLANG_PATH -or -not(Test-Path -LiteralPath (Join-Path $env:LIBCLANG_PATH 'libclang.dll'))){throw 'LLVM 21+ is required. Set LIBCLANG_PATH to its bin directory.'}
if(-not $env:CARGO_TARGET_DIR){$env:CARGO_TARGET_DIR=Join-Path $taskProject 'target'}
$taskOldFlags=$env:CARGO_ENCODED_RUSTFLAGS
$taskFlags=[Collections.Generic.List[string]]::new()
if($taskOldFlags){foreach($taskFlag in $taskOldFlags.Split([char]31)){$taskFlags.Add($taskFlag)}}
elseif($env:RUSTFLAGS){foreach($taskFlag in ($env:RUSTFLAGS -split '\s+' | Where-Object {$_})){$taskFlags.Add($taskFlag)}}
# Keep local build paths out of the distributed DLL's panic messages.
foreach($taskMapping in @(@($env:USERPROFILE,'/build'),@($env:CARGO_HOME,'/cargo'),@($taskProject,'/frame'))){
    if($taskMapping[0]){
        $taskPath=[IO.Path]::GetFullPath($taskMapping[0]).TrimEnd('\','/')
        $taskFlags.Add('--remap-path-prefix='+$taskPath+'='+$taskMapping[1])
        $taskFlags.Add('--remap-path-prefix='+$taskPath.Replace('\','/')+'='+$taskMapping[1])
    }
}
$env:CARGO_ENCODED_RUSTFLAGS=$taskFlags -join [char]31
Push-Location $taskProject
try {
    if(-not $SkipTests){
        cargo fmt --check
        if($LASTEXITCODE -ne 0){throw 'Rust formatting check failed.'}
        & (Join-Path $taskProject 'scripts/test-import.ps1')
        cargo test --locked --no-default-features
        if($LASTEXITCODE -ne 0){throw 'Core and file tests failed.'}
        cargo test --locked -p geode-codegen frame_layout_tests
        if($LASTEXITCODE -ne 0){throw 'Native layout generator tests failed.'}
    }
    cargo build --locked --release
    if($LASTEXITCODE -ne 0){throw 'The release build failed.'}
    if(-not $SkipTests){
        cargo test --locked --release --test native_layout
        if($LASTEXITCODE -ne 0){throw 'Native fields do not match GD 2.2081.'}
        cargo test --locked --release --lib
        if($LASTEXITCODE -ne 0){throw 'Native capture and editor interaction tests failed.'}
    }
    & (Join-Path $taskProject 'scripts/package.ps1') -DllPath (Join-Path $env:CARGO_TARGET_DIR 'release/frame_studio.dll')
} finally {
    Pop-Location
    $env:CARGO_ENCODED_RUSTFLAGS=$taskOldFlags
}
