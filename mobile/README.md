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

Verified on the Android 15 x86_64 emulator (2026-10-16): pairing with a desktop by
ticket, bringing a workspace over, sync both ways, Files and the text preview (Japanese
included), and no socket anywhere under the app's storage.

- **Typing** goes through the framework's own bridge from GameActivity's text buffer
  (winit 0.30 drops those events). Latin text, deletion and moving between fields work;
  a Japanese IME's composition is wired (preedit, then commit) but not yet tried on a
  device with a Japanese keyboard.
- **Paste** reads the clipboard through JNI. `adb shell input text` drops characters past
  ~150 at once; send long strings in pieces when testing.
- AndroidX's GameTextInput logs injected key events at debug level (`gti.InputConnection`)
  with their text. The app's own logs keep typed text out.
- Some framework screens are laid out for a desktop (Workgroup puts two forms side by
  side); they work but are cramped on a phone.
- Sync runs only while the app is open.
- The release `.so` is ~60 MB per ABI before the APK compresses it.
