# Builds target\installer\ClaudeUsage-Setup-<version>.exe: the release exe, then Inno Setup.
# Run from anywhere: powershell -ExecutionPolicy Bypass -File installer\build.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
Set-Location $root

# The version from Cargo.toml, so the setup and the exe always agree.
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

# The running app locks its exe when started from target\release: ask it to exit first.
if (Test-Path target\release\claude-usage.exe) { & target\release\claude-usage.exe --quit | Out-Null }
# Rust keeps source paths for its panic messages: hide this computer's (user name, folders).
# The encoded form (0x1f between flags): the paths may hold spaces.
$env:CARGO_ENCODED_RUSTFLAGS = "--remap-path-prefix=$env:USERPROFILE=~" + [char]0x1f + "--remap-path-prefix=$root=."
cargo build --release
if ($LASTEXITCODE) { throw "cargo build failed" }

$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 not found: winget install JRSoftware.InnoSetup" }
& $iscc /Q "/DAppVersion=$version" installer\claude-usage.iss
if ($LASTEXITCODE) { throw "ISCC failed" }
Get-Item "target\installer\ClaudeUsage-Setup-$version.exe"
