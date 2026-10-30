# Startup hang investigation

The reported symptom is a window that opens and then becomes unresponsive, not
an error dialog or a process exit. The user confirmed that the startup test build
fixed this symptom on the affected tester's machine. A subsequent report of a
demo remaining at parsing 0% is not treated as proof of a decoder crash.

## Reproduction

An isolated synthetic library with 1,000 entries and 5,165,435 bytes of saved JSON
reproduces the symptom in the published 2.0.0 executable. No actual user library or
demo is modified by this test.

| Operation | 2.0.0 code | Patched code |
|---|---:|---:|
| Load saved library | 9,194 ms | 141 ms |
| Reconcile scan results | 23,897 ms | 93 ms |
| Save library | 959 ms | 78 ms |

These filesystem timings were measured in the debug integration test on the same
Windows host. They are diagnostic measurements, not performance guarantees.

A separate test of the actual optimized Windows executables sent window-message
heartbeats for 12 seconds. The published build became unresponsive (10 of 11
heartbeats timed out). The patched build stayed responsive (0 of 107 timed out;
maximum response time 10.2 ms). The title-bearing window was available after
9,467 ms versus 224 ms; this is not a first-paint measurement.

## Changes

* Buffer native library and voice-manifest JSON reads instead
  of issuing a filesystem read for each byte consumed by the JSON decoder.
* Buffer JSON writes and explicitly flush before syncing and atomically replacing
  the saved file. The existing no-partial-overwrite behavior remains.
* Index saved and scanned paths once. Previously each scanned demo canonicalized
  saved paths repeatedly, producing quadratic filesystem work on the UI thread.
* Draw only visible library rows/cards and count sidebar entries without cloning
  every cached match and annotation on every frame.
* Ignore `Library.json` and `Config.json`. Folders with the original .NET ID format
  are no longer watched, including folders imported into native data by 2.0.0.
  Their inactive annotations and all original files are retained.
* Make background parsing explicitly opt-in, including for existing 2.0.0 data.
  Cancel stops the background queue and is checked during validation and sign-on.
  Those stages show indeterminate progress until a tick percentage is available.

Regression tests cover cache/annotation preservation, large Unicode notes,
external renames, ambiguous fingerprints, and avoiding reuse of a moved record.
See [BUILDING.md](BUILDING.md#startup-regression-benchmark) for the repeatable
benchmark and interactive Windows heartbeat test.

The initial 2.0.1-dev test package was confirmed before preparing the stable
2.0.1 release. The original 2.0.0 GitHub release is not overwritten.
