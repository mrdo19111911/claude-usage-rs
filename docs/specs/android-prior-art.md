# Android: ways to ship SCAS's dashboard (prior art, 2026-11-12)

Question: how to put SCAS (a Rust core + an HTML/JS dashboard) on an Android phone.
Searched GitHub and the web; four approaches, plus the reference companion app.

| Approach | Example projects | What it gives | Fit for SCAS | Cost on this PC |
|---|---|---|---|---|
| **Plain Java/Kotlin WebView shell, built with the SDK's own tools** (aapt2 → javac → d8 → apksigner, no Gradle) | hereket.github.io "Build android apk from command line without gradle"; shkhuz.github.io "An android app from scratch"; TheDotLabs/Bacol; ajinasokan/apkc | ~30 KB APK, builds in seconds, no Rust, no NDK | **Best for a companion**: the dashboard is already a web page served by SCAS on the PC; the phone only needs a window onto it | JDK 21 is here. Needs SDK cmdline-tools + build-tools + one platform (~300 MB, no admin) |
| **Kotlin app + Gradle** (Android Studio project) | home-assistant/android (Kotlin, WebView at its centre, native bridge to the HA frontend) | Full Android project, Play-store ready, JS bridge | Same design as above, more machinery. HA's companion is the model: a WebView on a server's own frontend | Gradle (~150 MB) + its caches (1 GB+); same SDK parts |
| **cargo-ndk + Kotlin/Java shell** (Rust compiled to a `.so`, called over JNI) | rust-mobile/android-view (`cargo ndk -t arm64-v8a -o app/src/main/jniLibs build`); davehadley/exampleAndroidRust | Rust core runs on the phone | Only useful if SCAS's core runs on the phone (option c); it is built around `~/.claude` and the CLI, which a phone does not have | NDK (~1.5 GB unpacked) + `rustup target add aarch64-linux-android` + `cargo install cargo-ndk` |
| **Tauri 2 mobile / wry on Android** | tauri-apps/wry MOBILE.md (`android_binding!`, generated `WryActivity`, cargo-mobile2) | Rust app + system WebView, one codebase with desktop | SCAS already uses wry on Windows, but the mobile path needs cargo-mobile2's generated Gradle project; heavy for a viewer | NDK (tauri-cli pins 29.0.13846066) + Android Studio/Gradle + Java 17 + Rust targets |
| ~~cargo-apk / ndk-build~~ → **xbuild** | rust-mobile/cargo-apk (deprecated in favour of xbuild); rust-mobile/xbuild | Rust-only APK (NativeActivity) | No WebView story; cargo-apk is deprecated, xbuild is thinly maintained | NDK + xbuild's own SDK download |

Reaching the PC's server from the phone: Android's own docs
("Access a local development server from WebView") recommend `adb reverse tcp:P tcp:P`
over USB, so the WebView loads `http://localhost:P` and the request reaches the PC's
`localhost:P`. SCAS's Host check (`server.rs`: only `127.0.0.1:P` / `localhost:P`)
accepts that unchanged; a LAN or Tailscale address is refused (403), on purpose.

## Choice

**Plain Java WebView shell built with the SDK's tools (row 1)**, laid out like the Home
Assistant companion (row 2) but without Gradle. Reasons: the product is a companion
(docs/specs/android.md), so no Rust runs on the phone and the NDK, cargo-ndk and Tauri
are not needed; the whole build is four SDK commands this PC can run without admin;
moving to Gradle/Kotlin later only adds a build file, the sources stay.

## Sources

- https://hereket.github.io/posts/android_from_command_line
- https://shkhuz.github.io/blog/an-android-app-from-scratch.html
- https://github.com/TheDotLabs/Bacol
- https://developers.home-assistant.io/docs/android/architecture/overview
- https://github.com/rust-mobile/android-view
- https://github.com/davehadley/exampleAndroidRust
- https://github.com/tauri-apps/wry/blob/dev/MOBILE.md
- https://docs.rs/crate/tauri-cli/2.11.2/source/src/mobile/android/mod.rs
- https://github.com/rust-mobile/xbuild
- https://docs.rs/crate/cargo-apk/latest
- https://developer.android.com/develop/ui/views/layout/webapps/access-local-server
