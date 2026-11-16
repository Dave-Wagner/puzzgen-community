[CmdletBinding()]
param(
    [string]$BaseUrl='https://puzzlegen.davewagner.org',
    [ValidateSet('arm64-v8a','armeabi-v7a','x86_64','x86')][string]$Abi='arm64-v8a',
    [ValidateRange(1,8)][int]$Jobs=2
)
# Public build-only helper: no credentials, provisioning, installation or uploads.
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
if(-not $env:JAVA_HOME){throw 'Set JAVA_HOME to your installed JDK 17+ directory.'}
if(-not $env:ANDROID_HOME){throw 'Set ANDROID_HOME to your Android SDK directory.'}
if(-not $env:ANDROID_NDK_HOME){throw 'Set ANDROID_NDK_HOME to Android NDK r30.'}
foreach($required in @("$env:JAVA_HOME/bin/java.exe","$env:ANDROID_HOME/platform-tools/adb.exe","$env:ANDROID_NDK_HOME/toolchains/llvm")){
    if(-not(Test-Path -LiteralPath $required)){throw "Required tool path is missing: $required"}
}
Push-Location $root
try {
    & cargo ndk -t $Abi -P 26 -o apps/android/native/jniLibs build --locked -p puzzgen-native --lib --release -j $Jobs
    if($LASTEXITCODE -ne 0){throw 'Native Android build failed.'}
    $triple=switch($Abi){'armeabi-v7a'{'arm-linux-androideabi'} 'x86_64'{'x86_64-linux-android'} 'x86'{'i686-linux-android'} default{'aarch64-linux-android'}}
    Copy-Item -LiteralPath "$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/sysroot/usr/lib/$triple/libc++_shared.so" -Destination "apps/android/native/jniLibs/$Abi/libc++_shared.so"
    & ./apps/android/gradlew.bat -p apps/android assembleDebug lintDebug "-PpuzzgenBaseUrl=$BaseUrl" "-PpuzzgenAbi=$Abi" --no-daemon --no-configuration-cache --max-workers=1
    if($LASTEXITCODE -ne 0){throw 'Android packaging/lint failed.'}
    Write-Host 'APK: apps/android/app/build/outputs/apk/debug/app-debug.apk'
    Write-Host 'Build only: nothing installed, provisioned, published or uploaded.'
} finally {Pop-Location}
