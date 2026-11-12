# SCAS on Android: the companion app

What and why: `docs/specs/android.md`. How it is built: `docs/specs/android-prior-art.md`.

The app is a window onto the SCAS dashboard running on your PC. It holds no Claude
tokens; switching still happens on the PC.

## Tests (no Android SDK needed)

```powershell
android\scas-companion\test.ps1
```

## One-time setup on Windows (no admin; about 300 MB)

1. Download "Command line tools only" for Windows from
   https://developer.android.com/studio#command-line-tools-only
   (`commandlinetools-win-*_latest.zip`, about 150 MB).
2. Unzip so that this file exists:
   `%LOCALAPPDATA%\Android\Sdk\cmdline-tools\latest\bin\sdkmanager.bat`
3. In PowerShell:
   ```powershell
   $env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
   [Environment]::SetEnvironmentVariable('ANDROID_HOME', $env:ANDROID_HOME, 'User')
   & "$env:ANDROID_HOME\cmdline-tools\latest\bin\sdkmanager.bat" --licenses
   & "$env:ANDROID_HOME\cmdline-tools\latest\bin\sdkmanager.bat" "platform-tools" "build-tools;35.0.0" "platforms;android-35"
   ```
   `--licenses` asks you to read and accept Google's Android SDK licence.
   JDK 21 (Temurin) is already on this PC; no NDK, no Gradle, no Rust target needed.

## Build

```powershell
android\scas-companion\build.ps1
```

Output: `target\android\scas-companion.apk` (debug-signed).

## Use it

1. On the phone: Settings → About → tap Build number 7 times; Developer options →
   USB debugging on. Plug it into the PC and allow the PC.
2. Install: `& "$env:ANDROID_HOME\platform-tools\adb.exe" install -r target\android\scas-companion.apk`
3. Find SCAS's port: macOS/Linux `4777`; Windows the number in
   `%USERPROFILE%\.claude-usage-dashboard\port`.
4. Tunnel the phone's localhost to the PC's: `adb reverse tcp:PORT tcp:PORT`
   (again after each replug).
5. Open SCAS on the phone, type `PORT` (or `http://127.0.0.1:PORT`), Open.

Over Wi-Fi or Tailscale SCAS answers 403 "forbidden host" on purpose (its DNS-rebinding
guard). Opening it to the network needs a pairing step on the PC first: see "Reaching
the PC" in the spec.
