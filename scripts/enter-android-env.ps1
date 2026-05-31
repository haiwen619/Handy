param()

$ErrorActionPreference = 'Stop'

function Add-ToPathIfExists {
  param([string]$Dir)
  if ([string]::IsNullOrWhiteSpace($Dir)) { return }
  if (Test-Path $Dir) {
    $parts = @($env:PATH -split ';') | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and $_ -ne $Dir }
    $env:PATH = (@($Dir) + $parts) -join ';'
  }
}

$isDotSourced = $MyInvocation.InvocationName -eq '.'
if (-not $isDotSourced) {
  Write-Warning "Dot-source this script to persist env: . .\scripts\enter-android-env.ps1"
}

# 1) ANDROID_HOME
$androidHomeCandidates = @(
  $env:ANDROID_HOME,
  (Join-Path $env:LOCALAPPDATA 'Android\Sdk'),
  'C:\Android\Sdk',
  'D:\Android\Sdk'
) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and (Test-Path $_) }
$androidHome = $androidHomeCandidates | Select-Object -First 1
if (-not $androidHome) {
  throw "ANDROID_HOME not found. Install Android Studio or set ANDROID_HOME. See docs/mobile/android-build.md."
}
$env:ANDROID_HOME = $androidHome
$env:ANDROID_SDK_ROOT = $androidHome

# 2) NDK — pick highest-versioned dir under $ANDROID_HOME\ndk
$ndkRoot = Join-Path $androidHome 'ndk'
if (-not (Test-Path $ndkRoot)) {
  throw "No NDK installed at $ndkRoot. Install NDK via Android Studio SDK Manager."
}
$ndk = Get-ChildItem $ndkRoot -Directory | Sort-Object Name -Descending | Select-Object -First 1
if (-not $ndk) {
  throw "No NDK subdirectory under $ndkRoot."
}
$env:ANDROID_NDK_HOME = $ndk.FullName
$env:ANDROID_NDK_ROOT = $ndk.FullName
$env:NDK_HOME = $ndk.FullName

# 3) JDK 17 — Android Studio's bundled JBR first, then Eclipse Temurin
# (winget installs Temurin to "C:\Program Files\Eclipse Adoptium\jdk-17.x.y-hotspot").
$jbr = 'C:\Program Files\Android\Android Studio\jbr'
$temurinRoot = 'C:\Program Files\Eclipse Adoptium'
if (Test-Path (Join-Path $jbr 'bin\java.exe')) {
  $env:JAVA_HOME = $jbr
} elseif (Test-Path $temurinRoot) {
  $temurin17 = Get-ChildItem $temurinRoot -Directory -Filter 'jdk-17*' |
    Sort-Object Name -Descending | Select-Object -First 1
  if ($temurin17 -and (Test-Path (Join-Path $temurin17.FullName 'bin\java.exe'))) {
    $env:JAVA_HOME = $temurin17.FullName
  }
}
if (-not $env:JAVA_HOME -or -not (Test-Path (Join-Path $env:JAVA_HOME 'bin\java.exe'))) {
  if ($env:JAVA_HOME -and (Test-Path (Join-Path $env:JAVA_HOME 'bin\java.exe'))) {
    # keep existing
  } else {
    throw "JDK 17 not found. Install Android Studio, or run: winget install EclipseAdoptium.Temurin.17.JDK"
  }
}

# Gradle's `gradlew.bat` splits GRADLE_OPTS on whitespace and chokes on
# "C:\Program Files\..." paths. Force it to use the JDK_HOME via the
# org.gradle.java.home system property, passed via env var that *we* know
# was constructed without unquoted spaces inside it (we always wrap in -D).
# If JAVA_HOME has spaces, GRADLE_OPTS won't work — fall back to leaving it
# unset and rely on the gradle.properties / PATH-based detection instead.
if ($env:JAVA_HOME -and ($env:JAVA_HOME -notmatch ' ')) {
  $env:GRADLE_OPTS = "-Dorg.gradle.java.home=$env:JAVA_HOME"
} else {
  Remove-Item Env:GRADLE_OPTS -ErrorAction SilentlyContinue
}

# Strip JRE 1.8 (no JAVA_COMPILER) and any stale jdk-17 paths from PATH so
# Gradle's toolchain auto-detect doesn't accidentally pick them.
$badJavaPaths = @(
  'C:\Program Files\Java\jre1.8.0_461\bin',
  'F:\VS2022\Android\jdk-17\bin'
)
$env:PATH = (($env:PATH -split ';') | Where-Object {
  $p = $_; -not [string]::IsNullOrWhiteSpace($p) -and ($badJavaPaths -notcontains $p)
}) -join ';'
Add-ToPathIfExists (Join-Path $env:JAVA_HOME 'bin')

# 4) Short cargo target dir for Android (avoid TEMP, mirrors enter-build-env.ps1)
if ([string]::IsNullOrWhiteSpace($env:CARGO_NDK_TARGET_DIR)) {
  $candidates = @('C:\handy-android-target', 'D:\handy-android-target', 'F:\handy-android-target')
  foreach ($c in $candidates) {
    $root = Split-Path $c -Qualifier
    if (Test-Path $root) {
      if (-not (Test-Path $c)) {
        try { New-Item -ItemType Directory -Path $c -Force | Out-Null } catch { continue }
      }
      $env:CARGO_NDK_TARGET_DIR = $c
      break
    }
  }
}

# 5) NDK toolchain bin on PATH
$ndkBin = Join-Path $env:ANDROID_NDK_HOME 'toolchains\llvm\prebuilt\windows-x86_64\bin'
Add-ToPathIfExists $ndkBin

# 6) Android SDK platform-tools (adb) on PATH
Add-ToPathIfExists (Join-Path $androidHome 'platform-tools')

# 6a) whisper.cpp cross-compile env (needed by whisper-rs-sys' build script).
# Without these, cmake defaults to Visual Studio 17 generator and fails
# building the aarch64-android whisper.cpp objects (VCTargetsPath error).
$env:CMAKE_GENERATOR = 'Ninja'
$env:ANDROID_PLATFORM = '24'
$env:WHISPER_CMAKE_ARGS = '-DGGML_OPENMP=OFF'
# $PSScriptRoot is the dir of THIS script ($repo\scripts). The cmake
# toolchain wrapper lives under $repo\scripts\android\.
if ($PSScriptRoot) {
  $repoToolchain = Join-Path $PSScriptRoot 'android\android-arm64.toolchain.cmake'
  if (Test-Path $repoToolchain) {
    $env:CMAKE_TOOLCHAIN_FILE_aarch64_linux_android = (Resolve-Path $repoToolchain).Path
  }
}

# 6b) Tauri mobile dev: force webview to talk to "localhost" so adb reverse
# can route to PC. Without this, tauri auto-picks a NIC IP (often a Docker
# bridge like 172.18.0.1) that the phone can't reach -> white screen.
$env:TAURI_DEV_HOST = 'localhost'

# 7) Ensure rustup targets installed
$installed = & rustup target list --installed 2>$null
foreach ($t in @('aarch64-linux-android', 'armv7-linux-androideabi', 'x86_64-linux-android', 'i686-linux-android')) {
  if ($installed -notcontains $t) {
    Write-Host "rustup target add $t"
    & rustup target add $t
  }
}

Write-Host "Loaded Android env:" -ForegroundColor Cyan
Write-Host "  ANDROID_HOME=$env:ANDROID_HOME"
Write-Host "  ANDROID_NDK_HOME=$env:ANDROID_NDK_HOME"
Write-Host "  JAVA_HOME=$env:JAVA_HOME"
Write-Host "  CARGO_NDK_TARGET_DIR=$env:CARGO_NDK_TARGET_DIR"
Write-Host "  CMAKE_GENERATOR=$env:CMAKE_GENERATOR"
Write-Host "  CMAKE_TOOLCHAIN_FILE_aarch64_linux_android=$env:CMAKE_TOOLCHAIN_FILE_aarch64_linux_android"
Write-Host "  TAURI_DEV_HOST=$env:TAURI_DEV_HOST"
Write-Host "  adb=" -NoNewline; (Get-Command adb -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source) | Write-Host
Write-Host "  java=" -NoNewline; (Get-Command java -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source) | Write-Host
Write-Host "  javac=" -NoNewline; (Get-Command javac -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source) | Write-Host
Write-Host "  ninja=" -NoNewline; (Get-Command ninja -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source) | Write-Host
