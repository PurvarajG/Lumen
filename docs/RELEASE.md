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
   version = "1.0.0-rc.3"
   ```
   All workspace crates inherit this version automatically via `version.workspace = true`.

---

## 3. Books (before packaging)

The book texts are copyright Anirdesh.com / BAPS and are included with their permission. They are never committed: fetch them into `.local/books` (gitignored), then the packaging script copies each `book.json` into the app. Keep a copy of the permission from Anirdesh/BAPS with your release records.

```sh
cargo run --release -p lumen-library --example fetch_books -- .local/books > .local/books/fetch.log 2>&1
```

It downloads the 15 catalog books one after another at about one request a second (about 1.5 hours the first time), skips books that are already complete and resumes interrupted ones. If a book fails, `fetch.log` names it; rerun to retry. `scripts/package-macos.sh --release` refuses to build when a book is missing. A book that is known to be broken can be left out on purpose with `ALLOW_MISSING_BOOKS="id id"` (the packager prints a note); pass `--without-books` for a quick development package.

---

## 3b. Automated Packaging

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
6. **Books and Notice:** Copies `.local/books/<id>/book.json` for all 15 catalog books into `Contents/Resources/Books/` and the copyright notice into `Contents/Resources/Licenses/Anirdesh-books.txt`.
6b. **Prohibited Files Assertion:** Verifies that no commercial fonts (`*chitra*`) are bundled inside the app.
7. **Ad-hoc Signing & Verification:** Signs the bundle ad-hoc and runs `codesign --verify --deep --strict`.
8. **DMG Disk Image Creation:** Stages the `.app` alongside an `/Applications` symlink and creates a compressed UDZO disk image:
   `.local/dist/Lumen-Presenter-<version>-macos-arm64.dmg`
9. **DMG Verification:** Validates the image checksum using `hdiutil verify`.

---

## 4. Checksum Publishing

Generate SHA-256 checksums for the disk image and application bundle:

```sh
cd .local/dist
shasum -a 256 Lumen-Presenter-1.0.0-rc.3-macos-arm64.dmg > Lumen-Presenter-1.0.0-rc.3-macos-arm64.dmg.sha256
```

Publish both the `.dmg` and `.sha256` files in the GitHub release or distribution channel.

---

## 4b. Publishing the Download

Each release is a GitHub release with its DMG attached; that page is the download link people are sent to.

1. Push the release commit to `main`.
2. Publish the release (pre-release while the version has `-rc`) with the DMG and its SHA-256 in the notes:
   ```sh
   gh release create v1.0.0-rc.5 .local/dist/Lumen-Presenter-1.0.0-rc.5-macos-arm64.dmg \
     --repo PurvarajG/Lumen --target main --prerelease \
     --title "Lumen Presenter 1.0.0-rc.5" --notes-file notes.md
   ```
3. Update the **Download** link at the top of `README.md` to the new version's DMG
   (`https://github.com/PurvarajG/Lumen/releases/download/v<version>/Lumen-Presenter-<version>-macos-arm64.dmg`).
   GitHub's `releases/latest` address skips pre-releases, so the README links the version directly.

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
