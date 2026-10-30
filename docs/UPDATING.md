# GitHub Auto-Updater

## Using it

The app checks the official GitHub releases when it starts, independently of demo
parsing. Disable **Automatically check for updates when the app starts** in
**Settings → App updates** if preferred, then save settings. The manual check
remains available.

For a newer compatible release:

1. Choose **Download update** in the banner or Settings. Progress and cancellation
   are available; network operations have timeouts. The installed files are not
   changed during download or verification.
2. Read the release notes if wanted, then choose **Restart & install** and confirm.
   Finish or cancel any active demo operation first.
3. The app saves the library. A copied Rust helper opens a handle to the app
   process and signals readiness; only then does the app close.
4. The helper waits for that process to exit, backs up files it will replace,
   installs the package's managed files, and launches the new executable.
5. The replacement checks its executable hash/version and acknowledges opening
   its window and library. Installation or startup failure restores the previous
   managed files and attempts to reopen the previous app. A failure dialog gives
   the backup/log location if manual recovery is needed.

The app folder must be writable, and the executable must be named
`SmallDemoManager.exe`. There is no administrator-elevation prompt, PowerShell
updater script, or extra runtime to install. Move the app to a folder you own or
extract the release ZIP manually if permissions prevent updating.

Updates do not replace your demos, `Library.native.json`, notes, tags, favorites,
or saved audio. Unmanaged installation files are left alone. The package's
README, patch notes, license notices, and docs are updated with its executable.

## Publishing a compatible release

This implementation does **not** publish releases automatically. For a release
you intend to publish:

1. Set the intended version in `Cargo.toml`, complete release validation, review
   dependency notices, and build with `scripts/package.ps1`.
2. Use a matching **three-part SemVer tag**, for example `v2.0.0` for version
   `2.0.0`. Old tags such as `v.1.0.18.0` are not this update format.
3. Attach both generated files to that same GitHub release:
   * `SmallDemoManager-native-windows-x64.zip`
   * `SmallDemoManager-native-windows-x64.zip.sha256`
4. Publish it as a stable release. Drafts and prereleases are ignored; there is no
   preview channel or downgrade installation.

The ZIP must have its executable at the root, not inside another folder. The
package script generates `update-manifest.json` containing:

* `schema`: `1`
* `application`: `small-demo-manager`
* `target`: `windows-x64`
* `version`: the exact Cargo package version
* `executable_sha256`: the packaged executable's SHA-256

Do not edit the ZIP after generating its checksum. The checksum sidecar contains
the archive hash and filename. Required original/dependency license notices must
remain in the package. Uploading the old `SmallDemoManager-shipping.zip` does not
make an update eligible for this app.

## Trust and verification

* Metadata comes only from the official
  `thedarkcr0w/Small-Demo-Manager` GitHub releases API. The updater chooses the
  highest compatible stable version among the API's most recent 100 releases.
* Initial asset URLs must belong to that repository's release downloads. Only
  HTTPS and an explicit GitHub/CDN redirect-host allowlist are accepted.
* Both the archive and sidecar are size-bounded. The archive must match its
  sidecar, and GitHub's asset digest is cross-checked when provided.
* Extraction rejects traversal, absolute paths, alternate data streams,
  case-insensitive duplicates, symlinks, unexpected root files, and excessive
  file counts or unpacked sizes. Windows ZIP separators are normalized before
  validation. Destination reparse points are refused.
* The app/platform/version manifest, required notices, executable hash, and x64
  PE header are validated. Staged hashes are checked again before replacement.
* Installation uses atomic per-file replacement and verified backups. The app
  executable is replaced last. A per-installation lock prevents two helpers from
  updating the same installation at once. The helper never kills an existing
  user process; only its own failed replacement process may be terminated during
  rollback.

SHA-256 detects corruption or mismatched assets; it is **not** a publisher
signature independent of GitHub. The trust boundary is the official GitHub
account plus HTTPS. The Windows build is currently unsigned; its dependency and
runtime notices are bundled. No updater can guarantee automatic
recovery from every power-loss, disk failure, or external file-lock condition;
the retained backup supports manual recovery if rollback itself is blocked.

## Files and diagnostics

Staging, helper files, logs, and retained rollback backups are under:

`%LocalAppData%\Small-Demo-Manager\Updates\update-<unique-id>\`

`SDM_DATA_DIR` relocates this with the rest of the app's data during isolated
testing. Cancelled, discarded, or uninstalled temporary downloads are cleaned up.
Once installation is handed to the helper, the directory is retained:

* `backup/` — previous versions of files replaced by that transaction.
* `install.json` — approved installation plan and staged hashes.
* `installed.txt` — completed installation marker.
* `error.txt` — installation/startup failure details, when applicable.

Do not remove an update directory while its helper is running. Completed update
directories may be removed manually when you no longer need their backups. The
updater does not recursively prune arbitrary files or old user data.

## Validation

Unit tests cover release selection, legacy/prerelease exclusion, URL and checksum
validation, cancellation and download bounds, package traversal/duplicates/
symlinks, Windows archive paths, PE validation, staged tampering, backup creation,
user-data preservation, partial-install rollback, and failed-startup rollback.

To run the real generated ZIP through the same validator used after downloading:

```powershell
$env:SDM_UPDATE_PACKAGE = (Resolve-Path 'dist/SmallDemoManager-native-windows-x64.zip').Path
cargo test --locked --lib packaged_release_passes_the_actual_updater_validation -- --ignored
```

This read-only package check and isolated helper tests are not a published-release
upgrade test. Until a compatible app release is actually published, the app
reports that none is available instead of offering a legacy package.

On an **interactive Windows desktop with the required graphics driver**, exercise
the real helper, executable replacement, backup, restart acknowledgement, and
annotation/audio preservation against isolated copies:

```powershell
./scripts/test-updater.ps1
```

This developer test briefly opens its own app windows and retains artifacts under
the temporary `SmallDemoManager-tests` directory for inspection. It does not
modify the source ZIP or an existing installation. `-KeepReplacementOpen` leaves
the isolated replacement window open after a successful test. This PowerShell
test harness is not used or required by the app's updater.
