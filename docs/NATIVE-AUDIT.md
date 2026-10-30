# Native migration / feature audit

The root Cargo project is the application. The unused C# / React sources,
WebView2 bridge, duplicate .NET shipping directories and ZIP, obsolete UI images,
and duplicate icons have been removed from the working tree. Earlier versions
remain in Git history. Only artwork embedded by Rust is retained in `assets/`.

The former RTF patch notes were converted to `PATCHNOTES.md`, preserving their
dated release history and adding the 2.0.0 release notes. `README.md` now uses
the original feature-table / installation / credits style with a real native
screenshot. Local file paths in that screenshot are explicitly redacted.

## Problems found in the previous application

* Generated SteamID64 values when a real ID was unavailable.
* Seeded decorative waveforms, unrelated to decoded audio.
* A custom, unlabeled "rating" and duration-estimated ADR, not measured stats.
* Move/copy and backup switches were saved but ignored by the move operation.
* "Watched" folders were enumerated manually; there was no filesystem watcher.
* A path was described as valid just because the string was nonempty.
* Parse cache did not invalidate when file size/mtime changed.
* Moves removed the original before verification, accepted path traversal in
  filenames, and discarded the selected library row even when copying.
* Voice output folders collided for demos with the same filename; a new
  extraction deleted the old clips before decoding had succeeded.
* Playback progress ran on a UI timer rather than the audio device position.
* The high voice-index mask duplicated the low mask; slots over 20 were ignored.
* Parse/extraction cancellation tokens were not passed through consistently.
* Unknown errors were often swallowed and replaced with a successful-looking UI.

## Replacement policy

No sample matches, synthetic player IDs, random waveforms, guessed ratings,
invented round results, dead menu items, or nonfunctional format selectors.
Unknown data is displayed as unavailable. Only WAV export is offered: no hidden
FFmpeg/.NET/system-encoder dependency. Match-lobby discovery is omitted because
it is not reliably available from an arbitrary demo. The GitHub auto-updater uses
its own Rust helper, SHA-256 verification, a platform/version manifest, managed-file
allowlisting, backup/rollback, and replacement-startup acknowledgement. It does not
run the legacy updater script or install old .NET archives. Details and release
requirements are in `docs/UPDATING.md`.

The native UI follows the original colors (`#0b0d11`, `#12151c`, `#f59e0b`),
three-pane layout, map artwork, two-team match view and voice workspace.

## Licensing

New darkcr0w-owned native code uses the personal, non-commercial-use license in
`LICENSE`: copyright 2026 darkcr0w, all other rights reserved. The unchanged
original MIT notice is preserved in `LICENSES/Small-Demo-Manager-original-MIT.txt`.
This does not revoke previously granted MIT rights or relicense dependencies.

## Compatibility

Parser: `source2-demo` **0.5.10**, pinned and vendored with explicit CS2 fixes in
`vendor/README.md`. Upstream
includes the October 9, 2026 protobuf refresh and lossy game-event string fix:
https://github.com/Rupas1k/source2-demo/commit/48fbddc0939d8a7963c4dd9df00b96a6ea5c7d98

Compatibility must be tested against actual demos, not inferred from the crate
version. `SmallDemoManager inspect <file.dem>` and `extract <file.dem> <output>`
exercise the same parser and decoder used by the UI without changing the source
demo or the user's library. Record validation results here before release.

## Validation performed (2026-10-30)

* 40 unit tests passed: file safety/no-overwrite/backup, ZIP traversal, corrupt
  data protection, library locking, legacy annotation migration, cache
  invalidation, relocation, console masks, binary demo envelopes, cancellation,
  real Opus encode/decode/WAV output, packet padding, clip grouping, and measured
  waveforms, plus update release selection, URL/hash/package validation,
  cancellation, Windows ZIP paths, backup/rollback and user-data preservation.
  Strict `cargo clippy --locked --all-targets -- -D warnings` passed.
* A complete, locally available `de_cache` recording parsed to 10 real players,
  **13–2**, and **15 observed round transitions** (13 A wins / 2 B wins). These
  independent recorded score transitions agree with the final scoreboard.
* The same recording exported **331 WAV clips**, **458.7 seconds** of decoded
  audio across **10 recorded speakers**. The source SHA-256 was unchanged.
  Clip grouping uses actual packet timing/round boundaries, not the per-packet
  `section_number`, which would otherwise create thousands of tiny files.
* The local `aim_redline` recording is genuinely truncated: its final command at
  tick 12237 declares 415 payload bytes with only 406 remaining. It is rejected
  with that specific error; no data or match results are fabricated.
* Public demofile-net fixtures **14011**, **14150_ag2 (AnimGraph2)** and **14160**
  all pass the explicit integration suite without changing their hashes.
  Source pinned to commit `b476cff578f195716af565da5315a59ae9c2629b` in CI.
* The native OpenGL window was launched and captured at 1380×860 on this Windows
  host. It displayed the real match using the original dark/amber layout.
* The optimized **Windows x64 ZIP** was built, its SHA-256 sidecar verified, and
  its executable extracted to an isolated directory. With no compiler directory
  added to runtime `PATH`, the packaged binary reproduced the same 10-player,
  13–2, 15-round result and preserved the demo's SHA-256. The refreshed match,
  settings, and voice views were captured; the voice workspace displayed genuine
  saved WAV clips and sample-derived waveforms, not placeholders.
* Packaging includes the darkcr0w license, original MIT notice, font licenses,
  dependency notices, and Rust / MinGW / GCC runtime notices. Workspace license
  texts omitted by crates are supplied from recorded upstream revisions in
  `LICENSES/upstream/`; the Windows dependency index fails packaging if a notice
  is missing. The Windows binary is unsigned, as stated in the release notes.
* The generated ZIP passes the same manifest/hash/PE/path validation used by the
  updater. A read-only live GitHub check correctly reports no compatible release
  rather than offering the existing .NET archive. An isolated Windows helper test
  replaced real executable bytes after parent exit, verified the old executable
  backup, received the replacement's startup acknowledgement, and preserved notes,
  favorites, saved-audio bytes, and an unmanaged installation file. Paths containing
  spaces and `&` were included. The repeatable test is `scripts/test-updater.ps1`;
  this does not claim a published-release download/upgrade was performed.

### Remaining compatibility coverage

* A **new demo recorded on the latest CS2 patch**. A file's download date is not
  its game/protocol version; the available full match alone cannot certify the
  October 28 update. The app reports its recorded build/protocol in inspection.
* Fresh full-length matchmaking/FACEIT/overtime/POV recordings, and manual audio
  playback/listening on additional hardware; verify cumulative stats against CS2.
* Windows 10 and additional DPI/graphics/audio-device configurations.
* Code signing for the Windows binary.

Release 2.0.0 is prepared from the tested `main` commit. Packaging itself does not
push commits, create tags, or publish a release.
