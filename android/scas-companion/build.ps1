# Builds target/android/scas-companion.apk with the SDK's own tools, no Gradle, no NDK
# (docs/specs/android.md "Build"). Needs ANDROID_HOME with build-tools;35.0.0 and
# platforms;android-35 (android/README.md). Runs test.ps1 first.
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$repo = Resolve-Path (Join-Path $here '..\..')
$sdk = $env:ANDROID_HOME
if (-not $sdk) { $sdk = Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
$bt = Join-Path $sdk 'build-tools\35.0.0'
$jar = Join-Path $sdk 'platforms\android-35\android.jar'
foreach ($p in @($bt, $jar)) {
    if (-not (Test-Path $p)) { Write-Error "Missing $p - see android/README.md" }
}

& (Join-Path $here 'test.ps1')
if ($LASTEXITCODE -ne 0) { Write-Error 'AddressTest failed' }

$out = Join-Path $repo 'target\android'
$obj = Join-Path $out 'obj'
if (Test-Path $obj) { Remove-Item -Recurse -Force $obj }
New-Item -ItemType Directory -Force (Join-Path $obj 'classes') | Out-Null

function Run($exe, [string[]]$argv) {
    & $exe @argv
    if ($LASTEXITCODE -ne 0) { Write-Error "$exe failed ($LASTEXITCODE)" }
}

# 1. Resources and manifest -> base.apk
Run (Join-Path $bt 'aapt2.exe') @('compile', '--dir', (Join-Path $here 'res'), '-o', (Join-Path $obj 'res.zip'))
Run (Join-Path $bt 'aapt2.exe') @('link', '-o', (Join-Path $obj 'base.apk'), '-I', $jar,
    '--manifest', (Join-Path $here 'AndroidManifest.xml'), (Join-Path $obj 'res.zip'))

# 2. Java -> classes -> classes.dex
$java = Get-ChildItem -Recurse (Join-Path $here 'src') -Filter *.java | ForEach-Object { $_.FullName }
Run 'javac' (@('--release', '11', '-classpath', $jar, '-d', (Join-Path $obj 'classes')) + $java)
$classes = Get-ChildItem -Recurse (Join-Path $obj 'classes') -Filter *.class | ForEach-Object { $_.FullName }
Run (Join-Path $bt 'd8.bat') (@('--release', '--min-api', '24', '--lib', $jar, '--output', $obj) + $classes)

# 3. classes.dex into the apk
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::Open((Join-Path $obj 'base.apk'), 'Update')
try {
    [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, (Join-Path $obj 'classes.dex'), 'classes.dex') | Out-Null
} finally { $zip.Dispose() }

# 4. Align, then sign with a debug key (made once, kept in target/android)
Run (Join-Path $bt 'zipalign.exe') @('-f', '-p', '4', (Join-Path $obj 'base.apk'), (Join-Path $obj 'aligned.apk'))
$ks = Join-Path $out 'debug.keystore'
if (-not (Test-Path $ks)) {
    Run 'keytool' @('-genkeypair', '-keystore', $ks, '-storepass', 'android', '-alias', 'androiddebugkey',
        '-keypass', 'android', '-keyalg', 'RSA', '-keysize', '2048', '-validity', '10000',
        '-dname', 'CN=Android Debug,O=Android,C=US')
}
$apk = Join-Path $out 'scas-companion.apk'
Run (Join-Path $bt 'apksigner.bat') @('sign', '--ks', $ks, '--ks-pass', 'pass:android', '--out', $apk,
    (Join-Path $obj 'aligned.apk'))
Run (Join-Path $bt 'apksigner.bat') @('verify', $apk)
Write-Output "APK: $apk ($((Get-Item $apk).Length) bytes)"
