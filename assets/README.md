# Embedded artwork

`iconApp.ico` and the 13 PNGs in `maps/` are the original repository artwork still
used by the native app. Their original attribution is retained in
`LICENSES/Small-Demo-Manager-original-MIT.txt` and `THIRD-PARTY-NOTICES.md`; the
darkcr0w personal-use terms do not replace pre-existing third-party rights.

The icon is embedded by `build.rs` and `src/main.rs`. Map images are registered in
`src/ui/theme.rs` and embedded at compile time with `include_bytes!`; they are not
loaded from a web frontend or downloaded at runtime. Adding another map requires
an image and a matching entry in `map_art`. Maps without artwork show their recorded
name rather than an unrelated image. Despite the `.png` names, some of the original
files hold JPEG or WebP data; the `image` crate is built with PNG, JPEG and WebP
decoders so every one of them loads.

`fonts/` holds the interface fonts, embedded unmodified by `src/ui/theme.rs`:
Onest (Regular/Medium/Bold), Unbounded (ExtraBold) and JetBrains Mono (Medium),
all under the SIL Open Font License 1.1 — see `LICENSES/fonts/OFL.txt`.
