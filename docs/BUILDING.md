# Building the Native App

## Windows prerequisites

* Rust **1.95 or newer** (`cargo`, `rustfmt`, and `clippy`).
* CMake, used to build the bundled libopus decoder.
* Recommended: the Rust MSVC toolchain plus Visual Studio Build Tools with
  **Desktop development with C++** and the Windows SDK.
* Alternatively, Rust GNU plus a matching **GCC-based MinGW-w64** toolchain on
  `PATH`. Do not mix LLVM-MinGW objects/libraries with GCC MinGW output, or add
  `-C link-self-contained=yes` to this GNU setup; that combination failed startup
  validation on the test host.

## Build, test, and package

```powershell
cargo run --locked
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
./scripts/package.ps1
```

The package script builds `SmallDemoManager.exe`, bundles the README, patch notes,
screenshot, docs, licenses, and available dependency notices, then writes:

* `dist/SmallDemoManager-native-windows-x64.zip`
* `dist/SmallDemoManager-native-windows-x64.zip.sha256`

It does not push, tag, or publish anything. `CARGO_TARGET_DIR` is respected.
Packaging fails if a Windows dependency is missing its license texts. Workspace
notices omitted by crates are supplied from the reviewed files in `LICENSES/upstream/`.
The ZIP includes `update-manifest.json` with the version, platform, and executable
hash. Publishing a compatible update also requires its SHA-256 sidecar and a
matching stable SemVer tag; see [UPDATING.md](UPDATING.md).

### GNU toolchain example

Replace the compiler path with your MinGW-w64 installation. A separate target
directory avoids reusing objects from another native toolchain.

```powershell
$bin = 'C:\Toolchains\mingw64\bin'
$env:PATH = "$bin;$env:PATH"
$env:CC = "$bin\gcc.exe"
$env:CXX = "$bin\g++.exe"
$env:CMAKE_GENERATOR = 'MinGW Makefiles'
$env:CARGO_TARGET_DIR = 'target\gnu'
cargo run --locked
```

## Read-only CLI diagnostics

Use the console debug build; the optimized release executable uses the Windows
GUI subsystem. These commands use the same parser and decoder as the UI without
modifying the source demo or the user's library:

```powershell
cargo run --locked -- inspect 'C:\Demos\match.dem'
cargo run --locked -- extract 'C:\Demos\match.dem' 'C:\Demos\voice-test'
```

Choose a fresh extraction output directory; existing manifests are not replaced.
Set `SDM_DATA_DIR` to a separate folder before starting the GUI for isolated tests.

## Public protocol regression fixtures

The integration suite is ignored by default because demo binaries are not stored
in the repository. Download the same pinned public fixtures used by CI:

```powershell
$env:SDM_DEMO_FIXTURES = Join-Path $env:TEMP 'sdm-protocol-fixtures'
New-Item -ItemType Directory -Path $env:SDM_DEMO_FIXTURES -Force | Out-Null
$commit = 'b476cff578f195716af565da5315a59ae9c2629b'
foreach ($name in @('14011.dem', '14150_ag2.dem', '14160.dem')) {
    Invoke-WebRequest "https://raw.githubusercontent.com/saul/demofile-net/$commit/demos/$name" -OutFile (Join-Path $env:SDM_DEMO_FIXTURES $name)
}
cargo test --locked --test protocol_fixtures -- --ignored
```

Passing these fixtures does not certify an untested new CS2 patch. See
[NATIVE-AUDIT.md](NATIVE-AUDIT.md) for the evidence and remaining release checks.

## Source layout

| Path | Purpose |
|---|---|
| `src/` | Native UI, demo parsing, audio, persistence, and desktop integration. |
| `assets/` | The icon and map artwork embedded by Rust at build time. |
| `tests/` | Public-protocol regression suite. Unit tests are next to their code. |
| `vendor/source2-demo/` | Pinned parser source, compatibility fixes, and upstream licenses. |
| `scripts/` | Native packaging and dependency-notice collection. |
| `LICENSES/` | Preserved original license; generated packages also contain dependency notices. |
| `docs/` | Build instructions and migration / validation audit. |

The obsolete .NET / React sources and shipping artifacts were removed from the
working tree; earlier versions remain in Git history. Do not trim the vendored
parser's upstream source or license files merely because a module is not called
directly by this app.
