# Wrapper for android.toolchain.cmake that pins ANDROID_ABI/ANDROID_PLATFORM
# before delegating, because cmake-rs (used by whisper-rs-sys) cannot pass
# -DANDROID_ABI directly (only `CMAKE_*` env vars are forwarded). Without this
# the toolchain defaults to armv7-a, clashing with --target=aarch64-linux-android
# that cargo-ndk injects via CMAKE_C_FLAGS.
set(ANDROID_ABI arm64-v8a CACHE STRING "" FORCE)
set(ANDROID_PLATFORM android-24 CACHE STRING "" FORCE)
include("$ENV{ANDROID_NDK_HOME}/build/cmake/android.toolchain.cmake")