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

# 3) JDK 17 — Android Studio's bundled JBR first
$jbr = 'C:\Program Files\Android\Android Studio\jbr'
if (Test-Path (Join-Path $jbr 'bin\java.exe')) {
  $env:JAVA_HOME = $jbr
} elseif ($env:JAVA_HOME -and (Test-Path (Join-Path $env:JAVA_HOME 'bin\java.exe'))) {
  # keep existing
} else {
  throw "JDK 17 not found. Install Android Studio or set JAVA_HOME to a JDK 17 install."
}
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
Write-Host "  adb=" -NoNewline; (Get-Command adb -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source) | Write-Host
Write-Host "  java=" -NoNewline; (Get-Command java -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source) | Write-Host
