# Android Build — Windows

## Prerequisites

1. **Rust** stable — `rustup install stable`
2. **Android SDK** — either Android Studio (Hedgehog+) or the VS2022-bundled SDK at `F:\VS2022\Android\android-sdk` work. The repo's env script auto-detects `%LOCALAPPDATA%\Android\Sdk`, `C:\Android\Sdk`, `D:\Android\Sdk`; for any other location pre-set `$env:ANDROID_HOME` before dot-sourcing.
3. In the SDK Manager (Android Studio or `sdkmanager` CLI), install:
   - Android SDK Platform 24+ (API 24)
   - **NDK (Side by side)** — 27.x
   - **CMake 3.31.x** — required for `transcribe-rs` / whisper.cpp cross-compile (bundles ninja)
4. **JDK 17** — Gradle Android plugin requires JDK 17. Android Studio's bundled JBR (`C:\Program Files\Android\Android Studio\jbr`) is auto-detected by the env script. Otherwise pre-set `$env:JAVA_HOME` to a JDK 17 install (e.g. `F:\VS2022\Android\openjdk\jdk-17.0.14`). The script's fallback to `java` on PATH may resolve to a JRE 8 — pre-set explicitly to avoid that.
5. **Bun** — https://bun.sh

## First-time setup

```powershell
# In a fresh PowerShell at repo root:
. .\scripts\enter-build-env.ps1     # MSVC + cmake for desktop side of workspace
. .\scripts\enter-android-env.ps1   # ANDROID_HOME, NDK, JDK
cargo install tauri-cli --version "^2" --locked
cargo install cargo-ndk --locked
cd src-mobile-ui
bun install
```

## Build debug APK

```powershell
. .\scripts\enter-android-env.ps1

# Extra env vars required for transcribe-rs / whisper.cpp cross-compile (see
# "Android cross-compile notes" in docs/mobile/README.md for the why):
$env:PATH = "$env:ANDROID_HOME\cmake\3.31.6\bin;$env:PATH"
$env:CMAKE_GENERATOR = "Ninja"
$env:CMAKE_TOOLCHAIN_FILE_aarch64_linux_android = "$PWD\scripts\android\android-arm64.toolchain.cmake"

cd src-mobile
cargo tauri android build --apk --debug
```

Output: `src-mobile/gen/android/app/build/outputs/apk/...`.

## Install on a device

```powershell
adb install -r .\gen\android\app\build\outputs\apk\universal\debug\app-universal-debug.apk
```

## Troubleshooting

- **`transcribe-rs` whisper.cpp cmake fails**: confirm `GGML_OPENMP=OFF` is set; NDK toolchain lacks OpenMP libs. The Android target dep in `src-tauri/vendor/transcribe-rs/Cargo.toml` already pins this.
- **`clang: error: unsupported argument 'armv7-a' to option '-march='`**: the NDK toolchain defaulted to armv7-a. Confirm `CMAKE_TOOLCHAIN_FILE_aarch64_linux_android` points at the repo's wrapper at `scripts/android/android-arm64.toolchain.cmake` — without it the wrapper's `ANDROID_ABI=arm64-v8a` pin never gets applied.
- **`cargo tauri android init` fails on Windows**: ensure no path component contains spaces or Chinese characters; rerun with `enter-android-env.ps1` dot-sourced.
- **APK installs but immediately crashes**: `adb logcat | grep -i handy` for the Rust panic line.
- **Model download stuck**: app downloads `ggml-tiny-q5_1.bin` from HuggingFace (`huggingface.co/ggerganov/whisper.cpp`). In restricted networks, manually copy the file to `/sdcard/Android/data/com.handy.mobile/files/models/`.
- **`os error 112` / `LNK1201` during cargo install**: out of disk space on `%TEMP%`'s drive. Redirect with `$env:TEMP = 'D:\handy-temp'; $env:TMP = 'D:\handy-temp'` before retrying.
