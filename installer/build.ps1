# Builds target\installer\SCAS-Setup-<version>.exe: the release exe, then Inno Setup.
# Run from anywhere: powershell -ExecutionPolicy Bypass -File installer\build.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
Set-Location $root

# The version from Cargo.toml, so the setup and the exe always agree.
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

# The running app locks its exe when started from target\release: ask it to exit first.
# Only that one: --quit ends any copy, and the installed one must keep running.
$dev = (Resolve-Path target\release\scas.exe -ErrorAction SilentlyContinue).Path
if ($dev -and (Get-Process scas -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $dev })) { & $dev --quit | Out-Null }
# Rust keeps source paths for its panic messages: hide this computer's (user name, folders).
# The encoded form (0x1f between flags): the paths may hold spaces.
# Static C runtime (+crt-static): scas.exe must not import VCRUNTIME140.dll, so a VC++
# Runtime update can't shut it down via Restart Manager (docs/specs/static-crt.md). This
# env overrides .cargo/config rustflags, so the flag is repeated here.
$env:CARGO_ENCODED_RUSTFLAGS = "--remap-path-prefix=$env:USERPROFILE=~" + [char]0x1f + "--remap-path-prefix=$root=." + [char]0x1f + "-C" + [char]0x1f + "target-feature=+crt-static"
cargo build --release
if ($LASTEXITCODE) { throw "cargo build failed" }

# Guard: the release exe must be statically linked to the C runtime.
findstr /M /C:"VCRUNTIME140" target\release\scas.exe > $null 2>&1
if ($LASTEXITCODE -eq 0) { throw "static CRT failed: target\release\scas.exe still imports VCRUNTIME140" }

$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 not found: winget install JRSoftware.InnoSetup" }
& $iscc /Q "/DAppVersion=$version" installer\scas.iss
if ($LASTEXITCODE) { throw "ISCC failed" }
Get-Item "target\installer\SCAS-Setup-$version.exe"
