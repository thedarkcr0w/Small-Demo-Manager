# Third-party notices

New native application: Copyright (c) 2026 darkcr0w. All rights reserved;
personal, non-commercial use permitted under `LICENSE`.

Original Small Demo Manager: Copyright (c) 2025 Mustafa J., MIT license. The
original license and attribution are preserved in
`LICENSES/Small-Demo-Manager-original-MIT.txt`. The original icon and map artwork
are retained in `assets/`. Those materials and pre-existing source are
not relicensed by the native application's personal-use terms.

The native application uses these open-source projects (including transitive
dependencies resolved in `Cargo.lock`):

* egui / eframe / egui_extras — MIT OR Apache-2.0
  https://github.com/emilk/egui
* source2-demo / protobufs — MIT OR Apache-2.0
  https://github.com/Rupas1k/source2-demo
* opus Rust bindings — MIT OR Apache-2.0
  https://github.com/SpaceManiac/opus-rs
* libopus (statically built by opusic-sys) — BSD-3-Clause
  https://opus-codec.org/license/
* rodio — MIT OR Apache-2.0
  https://github.com/RustAudio/rodio
* Symphonia (WAV/PCM decoding) — MPL-2.0
  https://github.com/pdeljanov/Symphonia
* hound — Apache-2.0
  https://github.com/ruuda/hound
* notify — CC0-1.0
  https://github.com/notify-rs/notify
* rfd — MIT
  https://github.com/PolyMeilex/rfd
* serde / serde_json — MIT OR Apache-2.0
* chrono — MIT OR Apache-2.0
* reqwest / rustls — MIT OR Apache-2.0 / ISC as applicable
* sha2 — MIT OR Apache-2.0
* tempfile — MIT OR Apache-2.0
* zip — MIT
* trash — MIT
* Rust standard library and compiler-builtins — compiler/runtime notices below in the release bundle
* GCC runtime — GPL-3.0 with the GCC Runtime Library Exception
* MinGW-w64 runtime and winpthreads — their bundled notices apply

This is an attribution index, not a replacement for upstream license texts.
`scripts/package.ps1` appends full license/notice texts from the locked Windows
dependency graph to this file in the release ZIP. Its component/source index is
`LICENSES/dependencies/INDEX.md`.
Workspace license texts omitted by published crates are supplied from
the repository's `LICENSES/upstream/`, with their upstream URLs, revisions, and
file hashes. Font licenses and compiler/runtime notices are consolidated here
in the release bundle as well. The original MIT notice is also retained at its
existing path for compatibility with the 2.0.0 updater. Packaging fails on missing dependency
notices instead of producing a warning-only bundle.
Unmodified sources are available from each exact crates.io version linked in
that index and via `cargo fetch --locked`; the modified parser is in `vendor/`.
The build script does not publish GitHub releases.

Segoe UI / Consolas are loaded from the operating system when available and are
not bundled or redistributed. egui's fallback fonts are provided by
epaint_default_fonts under its upstream licenses.
