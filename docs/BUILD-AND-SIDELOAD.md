# Build and sideload Beacon & Bough

Beacon & Bough is a free, ad-free native Rust/Bevy Android game and a companion
to the Puzzgen learning API. No WebView is used by the current native app.
Source publication is pending owner review; cloning requires access until then.

## Windows prerequisites

Use PowerShell 7 (`pwsh`), Git, Rust stable with MSVC build tools, a JDK 17 or
newer, and Android command-line SDK tools. Android Studio is optional once the
SDK/JDK are installed. Set `JAVA_HOME`, `ANDROID_HOME` and `ANDROID_NDK_HOME` to
their actual directories. Install SDK platform 36, build-tools 36.0.0,
platform-tools and Android NDK r30. The public helper requires all three
environment variables and never provisions keys, installs or uploads anything.

```powershell
git clone https://github.com/Dave-Wagner/puzzgen.git
Set-Location puzzgen
rustup target add aarch64-linux-android
cargo install cargo-ndk --version 4.1.2 --locked
pwsh -NoProfile -File .\scripts\Build-AndroidPublic.ps1 -Jobs 2
```

This compiles native Rust locally and packages/lints the debug APK. It can take
substantial time on the first build; it does not start cloud jobs or publish.
The helper is Windows-specific. Linux server builds are documented in README;
an equivalent Linux Android packaging workflow is not verified here.

Output: `apps/android/app/build/outputs/apk/debug/app-debug.apk` (ARM64, Android 8/API 26
or later). Do not assume ARM64 support from the Android version alone.

The workspace requires `apps/android/{native,game,weather,progression}` and
`third_party/{bevy_render,bevy_pbr,wgpu-hal}`, Cargo.lock, selected runtime assets,
and their licenses. A public snapshot missing these cannot build. See the local
publication review for the recorded clean-checkout build result.

## Install your own build

Enable Developer options/USB debugging on a device you own, connect and authorize
it, then use an explicit serial to avoid installing on the wrong device:

```powershell
$adb = Join-Path $env:ANDROID_HOME 'platform-tools/adb.exe'
& $adb devices
& $adb -s YOUR_DEVICE_SERIAL install -r .\apps\android\app\build\outputs\apk\debug\app-debug.apk
```

Open **Beacon & Bough Dev** yourself after installation. ADB does not launch it.
Alternatively transfer your APK to your device, open it, and temporarily allow
installation from that file-manager source. Disable that permission afterward.

If Android reports a signature mismatch, stop. Do not uninstall an existing app
to force an update: uninstalling can erase saves and its encrypted API key.
Use the original signer, or separately plan a different application ID. Each
builder's debug key differs. `install -r` preserves data only when Android accepts
the same package/signing identity; it cannot guarantee save compatibility.

## Online service and environments

Practice fixtures are bundled; fresh puzzles require an available backend.
The development build defaults to the personal dev service, with anonymous
Firebase auth and server-enforced quotas. Public Firebase client configuration
is identification, not a backend secret or an unlimited-access credential.
Independent deployments should use their own backend and Firebase projects.
Do not expect production capacity or service availability from the dev endpoint.

You do not need the maintainer's `.env`, family API key, upload keystore, or
password to build. Do not use `-ProvisionDevKey` in public setup instructions.
If setting up your own Firebase, replace the public per-environment client JSON
and configure the matching verifier on your own Rust server. Never copy a
Firebase Admin/service-account private key into the app.

Debug uses `org.davewagner.puzzlegen.beaconandbough.dev`; internal/production use
`org.davewagner.puzzlegen.beaconandbough` with separate service configurations.
They do not share saves. This command does not produce an owner-signed public
release or Play bundle. Future official GitHub APKs should use Play-signed
distribution APKs to preserve the official update identity; independent source
builds use the builder's own key. An AAB is not directly sideloadable.

For the API and browser playground, see the root README and `.env.example`.
For credits and reuse terms, see THIRD-PARTY-NOTICES.md. No ads, in-app purchases,
medical effectiveness, or cloud-save recovery are promised by this preview.

