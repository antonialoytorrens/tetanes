# Building for Android

TetaNES supports building for Android using `xbuild`.

## Prerequisites

1.  **Rust Toolchain**: Ensure you have a working Rust installation.
2.  **Android SDK/NDK**: Install Android Studio or the command-line tools.
    *   Set `ANDROID_HOME` environment variable to your SDK location.
    *   Set `ANDROID_NDK_ROOT` to your NDK location.
3.  **xbuild**: Install the `xbuild` tool.

```bash
cargo install xbuild
```

## Building

To build the APK for a specific architecture (e.g., `aarch64`):

```bash
# Navigate to the package directory
cd tetanes

# Build release APK
xbuild build --release --target aarch64-linux-android
```

Supported targets:
*   `aarch64-linux-android` (Most modern devices)
*   `armv7-linux-androideabi` (Older devices)
*   `x86_64-linux-android` (Emulators)
*   `i686-linux-android` (Older emulators)

## running

 connect your device or start an emulator, then run:

```bash
xbuild run --release --target aarch64-linux-android
```

## Troubleshooting

*   **Linker Errors**: If you encounter linker errors related to `aaudio`, ensure your NDK is properly configured and that `Xbuild.toml` specifies a `compile-sdk-version` of at least 30.
