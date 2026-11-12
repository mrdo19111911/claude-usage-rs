# Runs the pure-Java tests (no Android SDK needed): compiles Address + AddressTest
# into target/android/test-classes and runs them. Exit code 1 on a failure.
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$repo = Resolve-Path (Join-Path $here '..\..')
$out = Join-Path $repo 'target\android\test-classes'
New-Item -ItemType Directory -Force $out | Out-Null
$src = @(
    (Join-Path $here 'src\vn\smartlog\scas\companion\Address.java'),
    (Join-Path $here 'test\AddressTest.java')
)
& javac --release 11 -d $out @src
if ($LASTEXITCODE -ne 0) { exit 1 }
& java -cp $out AddressTest
exit $LASTEXITCODE
