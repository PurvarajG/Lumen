# Cutting a Lumen Presenter Release

This guide covers the end-to-end process of packaging, verifying, and publishing release artifacts for Lumen Presenter.

---

## 1. Prerequisites

Before cutting a release:
- macOS 12.0+ on Apple Silicon (arm64).
- Rust stable toolchain (`rustup update stable`).
- Xcode Command Line Tools (`xcode-select --install`).
- Clean Git working tree on the release branch.
- External dependencies installed:
  ```sh
  brew install ffmpeg
  ```

---

## 2. Release Preparation

1. **Verify quality gates:**
   Ensure all automated formatting, workspace tests, and Clippy linting pass without warnings:
   ```sh
   cargo fmt --all -- --check
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   ```

2. **Bump workspace version:**
   Update the `version` field in the root `Cargo.toml` under `[workspace.package]`:
   ```toml
   [workspace.package]
   version = "1.0.0-rc.1"
   ```
   All workspace crates inherit this version automatically via `version.workspace = true`.

---

## 3. Automated Packaging

Run the packaging script with the `--release` flag:

```sh
scripts/package-macos.sh --release
```

### What the Script Performs
1. **Optimized Compilation:** Compiles `lumen-app` with the `[profile.release]` configuration (`lto = "thin"`, `codegen-units = 1`, `strip = true`).
2. **App Bundle Assembly:** Creates `.local/dist/Lumen Presenter.app` with standard macOS directory structure (`Contents/MacOS`, `Contents/Resources`).
3. **License Bundling:** Copies all third-party and application licenses into `Contents/Resources/Licenses/`:
   - `Poppins-OFL.txt` (SIL Open Font License)
   - `NotoSansGujarati-OFL.txt` (SIL Open Font License)
   - `Phosphor-MIT.txt` (Phosphor icons MIT License)
   - `Lumen-MIT.txt` (Lumen Presenter MIT License)
4. **App Icon Generation:** Ensures `AppIcon.icns` is built from the authored vector graphic (`crates/lumen-app/assets/icon.svg`) and placed in `Contents/Resources/AppIcon.icns`.
5. **Metadata Ingestion:** Generates `Contents/Info.plist` with the exact workspace version and icon reference.
6. **Prohibited Files Assertion:** Verifies that no commercial fonts (`*chitra*`) or scripture book texts are bundled inside the app.
7. **Ad-hoc Signing & Verification:** Signs the bundle ad-hoc and runs `codesign --verify --deep --strict`.
8. **DMG Disk Image Creation:** Stages the `.app` alongside an `/Applications` symlink and creates a compressed UDZO disk image:
   `.local/dist/Lumen-Presenter-<version>-macos-arm64.dmg`
9. **DMG Verification:** Validates the image checksum using `hdiutil verify`.

---

## 4. Checksum Publishing

Generate SHA-256 checksums for the disk image and application bundle:

```sh
cd .local/dist
shasum -a 256 Lumen-Presenter-1.0.0-rc.1-macos-arm64.dmg > Lumen-Presenter-1.0.0-rc.1-macos-arm64.dmg.sha256
```

Publish both the `.dmg` and `.sha256` files in the GitHub release or distribution channel.

---

## 5. Code Signing & Apple Notarization

### Current Status: Ad-hoc Signed
Lumen Presenter release builds are currently **ad-hoc signed** (`codesign -s -`). They run out of the box for local development and direct transfers, but macOS Gatekeeper will flag the app when downloaded from the internet because it has not been signed with a paid Apple Developer certificate.

**For operators installing ad-hoc builds:**
Either right-click (or Control-click) **Lumen Presenter.app** in Finder and select **Open**, or clear the quarantine attribute via Terminal:
```sh
xattr -cr "/Applications/Lumen Presenter.app"
```

### Future Distribution with Apple Developer ID
To produce fully notarized, warning-free macOS builds for public distribution, the following steps are required:

1. **Obtain Apple Developer ID:** Enroll in the Apple Developer Program ($99/year) and install a "Developer ID Application" certificate in your macOS Keychain.
2. **Sign with Hardened Runtime:**
   ```sh
   codesign --force --options runtime --deep --sign "Developer ID Application: <Team Name> (<Team ID>)" "$APP"
   ```
3. **Submit to Apple Notary Service:**
   ```sh
   xcrun notarytool submit "$DMG_PATH" \
     --keychain-profile "AC_PASSWORD" \
     --wait
   ```
4. **Staple Notarization Ticket:**
   ```sh
   xcrun stapler staple "$DMG_PATH"
   ```
5. **Verify Stapled Image:**
   ```sh
   spctl -a -t open --context context:primary-signature -v "$DMG_PATH"
   ```
