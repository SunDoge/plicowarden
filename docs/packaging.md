# Packaging

Plicowarden uses `cargo-packager` 0.11.8 for native desktop packages. The
package metadata lives in `Cargo.toml`, so release-plz remains the single source
of truth for the application version.

Install the pinned packager once:

```sh
cargo install cargo-packager --locked --version 0.11.8
```

Build and package on the target operating system:

```sh
# Debian, Ubuntu and portable Linux
cargo build --locked --release --package plicowarden \
  --features linux-bundled-freetype
cargo packager --release --formats deb,appimage --out-dir dist

# Windows, from PowerShell
cargo build --locked --release --package plicowarden
cargo packager --release --formats nsis,wix --out-dir dist

# macOS
cargo build --locked --release --package plicowarden
cargo packager --release --formats app,dmg --out-dir dist
```

Packages are native artifacts: build Windows installers on Windows and macOS
bundles on macOS. The checked-in macOS configuration uses ad-hoc signing for
development releases. Public downloads still need a Developer ID certificate
and Apple notarization. Windows packages are currently unsigned and can trigger
SmartScreen until a signing certificate is configured.

The release workflow keeps the portable archives alongside the `.deb`,
AppImage and native platform installers. Run an AppImage after making it
executable with `chmod +x Plicowarden*.AppImage`. RPM is intentionally outside
the first integration.
