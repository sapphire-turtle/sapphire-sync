# sapphire-sync on a phone

Experimental. Android first; iOS is the target this is shaped for.

The phone app is the desktop's screens (`crates/sapphire-sync-ui`) in a window the
framework opens on the phone (`sapphire-framework`'s `gui::mobile`). A phone has no
service manager, so the app hosts `sapphire-bridge` and the `sapphire-sync` server
**in its own process**, and they talk over the framework's in-process IPC carrier — no
Unix socket (an iOS container path overflows one), no file, nothing another app sees.
Sync runs while the app is open.

Everything is Rust and shared; the platform part is the entry point
(`src/lib.rs`: `android_main`) and the Android project around the library
(`android/`: GameActivity, no code of its own).

## Toolchain

- JDK 17 (`JAVA_HOME`)
- Android SDK (`ANDROID_HOME`) with `platforms;android-35`, `build-tools;35.0.0`,
  `platform-tools`, and an NDK (`ndk;28.2.13676358`, or `ANDROID_NDK_HOME`)
- For the emulator: `emulator` and `system-images;android-35;google_apis;x86_64`
- `rustup target add aarch64-linux-android x86_64-linux-android`
- `cargo install cargo-ndk`

## Build and run

```sh
mobile/build-android.sh                 # release Rust, arm64-v8a + x86_64
ABIS=x86_64 mobile/build-android.sh     # emulator only
adb install -r mobile/android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n io.github.sapphireturtle.sync/com.google.androidgamesdk.GameActivity
adb logcat -s sapphire-sync:* RustStdoutStderr:*
```

The app keeps everything under its internal storage
(`/data/data/io.github.sapphireturtle.sync/files`): `bridge/` (the device's identity and
workgroup), `data/`, `cache/`, `config/`, and `workspaces/<name>` — the synced folders,
which the **Files** screen shows. `adb shell run-as io.github.sapphireturtle.sync ls files`
looks inside on a debug build.

## Using it

1. On a desktop in the workgroup: **Settings → Devices → Invite** (or
   `sapphire-bridge device invite <name>`), and copy the ticket.
2. On the phone: **Settings → Workgroup → Join with a ticket**, **Paste**, the device name
   the ticket was issued for, **Join**.
3. **Settings → Workspace**: bring a workspace of the workgroup to the phone, or name a new
   one. It lands in `workspaces/<name>`.
4. **Files** browses it; text files preview, read-only.

## iOS (not yet)

The same crate builds as a static library (`crate-type` has `staticlib`). What is left:
an iOS entry in the framework's `gui::mobile` (the window through winit on UIKit, the
app's Application Support directory as the base, `UIPasteboard` for Paste), and a small
Xcode project under `mobile/ios/` that links `libsapphire_sync_mobile.a` and calls it.
The IPC carrier, the hosting, the directories and the screens are already the ones
Android runs.

## Known issues

See the end-to-end notes in the framework's plan
(`docs/superpowers/plans/2026-10-16-mobile-android.md`) and the PR.
