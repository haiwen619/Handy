# Android debugging — Windows host

Reinstalling a fresh debug APK on a phone every time you change a line of code
is slow and painful. This doc covers two faster loops.

## 1. In-app log panel (no host required)

The mobile UI ships a `LogPanel` at the bottom of the screen that subscribes
to the backend's `app/log` event. The Rust commands in
`src-mobile/src/commands/transcription.rs` emit a line at every meaningful
step (`start_recording invoked`, `audio capture started`, `drainer exited`
with frame/sample counts, `clip too short`, `whisper engine loaded`,
`whisper output`, etc.).

This is the fastest signal when something visibly misbehaves on the device.
**No PC connection needed** — read the panel directly on the phone.

To emit an extra log from anywhere with access to an `AppHandle`:

```rust
let _ = app.emit("app/log", serde_json::json!({
    "level": "info",       // or "warn" / "error"
    "msg":   "your text",
    "ts_ms": <millis>,
}));
```

## 2. `adb logcat` from Windows (tethered phone)

When you do have the phone plugged in, logcat shows the full Rust `log::*`
output plus any panics, JNI crashes, native ABORT, OOM, etc. — things that
never reach the in-app panel because the process is already dying.

```powershell
# Make sure platform-tools is on PATH
$env:Path += ";$env:ANDROID_HOME\platform-tools"

# Confirm the device is visible
adb devices

# Stream just our app's output. Tag "handy" is what tauri-plugin-log uses;
# RustStdoutStderr catches panics. Adjust if you change the app's Java package.
adb logcat -v color -s handy:V RustStdoutStderr:V *:E

# Or filter by PID for the running app:
adb logcat --pid=$(adb shell pidof -s computer.handy.app)
```

If you don't know the package id, check
`src-mobile/gen/android/app/build.gradle.kts` (`applicationId`).

## 3. Android emulator on Windows (no phone needed)

The Android emulator runs `arm64-v8a` system images on x86_64 hosts via
binary translation (slow but functional). Faster: `x86_64` system images
— but that requires building the APK for `x86_64` too, which Phase 2a's
CI does **not** do (`--target aarch64` only).

### Option A: arm64 emulator (works with our current APK)

```powershell
# Install an arm64 system image (one time)
sdkmanager --install "system-images;android-34;google_apis;arm64-v8a"

# Create an AVD
avdmanager create avd `
    --name handy-arm64 `
    --package "system-images;android-34;google_apis;arm64-v8a" `
    --device "pixel_6"

# Boot it (cold boot recommended after creation)
emulator -avd handy-arm64 -no-snapshot-load
```

Then `adb install -r path\to\app-arm64-v8a-debug.apk`.

Expect ~5–20× slower whisper inference vs. real device (it's emulated).
For pure UI / event-wiring debugging this is fine; for measuring
transcription latency it is misleading.

### Option B: native x86_64 emulator (fastest, requires extra build)

To go this route, build a second APK locally for `x86_64-linux-android`:

```powershell
# In src-mobile/  (with enter-android-env.ps1 sourced)
rustup target add x86_64-linux-android
cargo tauri android build --debug --target x86_64
adb install -r src-mobile\gen\android\app\build\outputs\apk\x86_64\debug\app-x86_64-debug.apk
```

This is **not** wired into CI and is intentional — Phase 2a is arm64-only
and adding x86_64 to CI would double the whisper.cpp / ggml compile time
on every push.

## 4. `tauri android dev` (live-reload, advanced)

```powershell
# Sourcing enter-android-env.ps1 must export ANDROID_HOME / NDK_HOME / JAVA_HOME
. .\enter-android-env.ps1
cargo tauri android dev --target aarch64 --open
```

This deploys + connects the Tauri devserver over USB and gives you React
HMR on the phone. The Rust side still has to be rebuilt + reinstalled
when you change Rust code (no Rust HMR), but the frontend loop is
seconds, not minutes.

Caveats on Windows:
- `enter-android-env.ps1` must be sourced first (sets `ANDROID_HOME`,
  `JAVA_HOME=jdk-17`, `TEMP/TMP=D:\handy-temp` to dodge the full C: drive
  and the MSVC GBK code-page issue).
- USB debugging must be enabled on the phone and the prompt accepted.
- The first deploy of a session is slow; subsequent rebuilds reuse the
  whisper.cpp object cache.

## When to use what

| Symptom                                           | Tool                                |
| ------------------------------------------------- | ----------------------------------- |
| Empty transcript, button states wrong             | **LogPanel** in the app             |
| App crashes / closes immediately                  | `adb logcat`                        |
| Permission denied / capability error              | `adb logcat`, grep "Permission"     |
| UI tweak / styling iteration                      | `tauri android dev` (HMR)           |
| Don't have phone today, just want to compile-test | CI (push to branch)                 |
| Want to repro on a clean device                   | arm64 emulator                      |
