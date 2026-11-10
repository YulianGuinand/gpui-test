<#
.SYNOPSIS
  Build the Rust native library, assemble the APK, install and launch it.

.EXAMPLE
  .\scripts\build-android.ps1                 # debug build, x86_64 emulator
  .\scripts\build-android.ps1 -Release        # release build (real perf numbers)
  .\scripts\build-android.ps1 -Abi arm64-v8a  # physical device
  .\scripts\build-android.ps1 -NoRun          # build only
  .\scripts\build-android.ps1 -Check          # fast cargo check, no APK
#>
param(
    [ValidateSet("x86_64", "arm64-v8a")]
    [string]$Abi = "x86_64",
    [switch]$Release,
    [switch]$NoRun,
    [switch]$Check
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$androidDir = Join-Path $root "android"
$jniLibs = Join-Path $androidDir "app\src\main\jniLibs"

# Gradle must run on a supported JDK: the system `java` here is Java 27,
# which Gradle 8.13 cannot use. Prefer Android Studio's bundled JBR.
if (-not $env:JAVA_HOME) {
    $jbr = "C:\Program Files\Android\Android Studio\jbr"
    if (Test-Path (Join-Path $jbr "bin\java.exe")) {
        $env:JAVA_HOME = $jbr
        Write-Host "[INFO] JAVA_HOME -> $env:JAVA_HOME"
    }
}

Push-Location $root
try {
    if ($Check) {
        Write-Host "[STEP] cargo check --target $Abi-linux-android"
        cargo check --target "$Abi-linux-android"
        if ($LASTEXITCODE -ne 0) { throw "cargo check failed" }
        return
    }

    # 1. Rust shared library
    $profile = if ($Release) { "--release" } else { "" }
    Write-Host "[STEP] cargo ndk -t $Abi ($(if ($Release) { 'release' } else { 'debug' }))"
    cargo ndk -t $Abi -o $jniLibs --platform 26 build $profile
    if ($LASTEXITCODE -ne 0) { throw "cargo ndk build failed" }

    # cargo-ndk also copies the cdylib of the `gpui-pre-mobile` dependency
    # (lib name `gpui_mobile`), which the NativeActivity never loads.
    Get-ChildItem -Recurse -Path $jniLibs -Filter *.so |
        Where-Object { $_.Name -ne "libmy_gpui_mobile.so" } |
        ForEach-Object { Write-Host "[INFO] removing stray $_.Name"; Remove-Item -Force $_.FullName }

    # 2. APK — remove any stale output first: a previous oversized APK left
    # ~1 GB of trailing bytes when the new zip was written over it.
    $gradleTask = if ($Release) { "assembleRelease" } else { "assembleDebug" }
    $variant = if ($Release) { "release" } else { "debug" }
    $apk = Join-Path $androidDir "app\build\outputs\apk\$variant\app-$variant.apk"
    if (Test-Path $apk) { Remove-Item -Force $apk }

    Write-Host "[STEP] gradlew $gradleTask"
    Push-Location $androidDir
    try {
        & .\gradlew.bat $gradleTask
        if ($LASTEXITCODE -ne 0) { throw "gradle build failed" }
    }
    finally { Pop-Location }

    $variant = if ($Release) { "release" } else { "debug" }
    $apk = Join-Path $androidDir "app\build\outputs\apk\$variant\app-$variant.apk"
    if (-not (Test-Path $apk)) { throw "APK not found: $apk" }
    Write-Host "[INFO] APK: $apk"

    if ($NoRun) { return }

    # 3. Install + launch
    Write-Host "[STEP] adb install -r"
    adb install -r $apk
    if ($LASTEXITCODE -ne 0) { throw "adb install failed" }

    Write-Host "[STEP] launch"
    adb shell am start -n "com.example.my_gpui_mobile/dev.gpui.mobile.GpuiActivity"
    Write-Host "[INFO] logs: adb logcat -s gpui-msg:D"
}
finally { Pop-Location }
