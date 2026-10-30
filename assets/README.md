# Embedded artwork

`iconApp.ico` and the 13 PNGs in `maps/` are the original repository artwork still
used by the native app. Their original attribution is retained in
`LICENSES/Small-Demo-Manager-original-MIT.txt` and `THIRD-PARTY-NOTICES.md`; the
darkcr0w personal-use terms do not replace pre-existing third-party rights.

The icon is embedded by `build.rs` and `src/main.rs`. Map images are registered in
`src/ui/theme.rs` and embedded at compile time with `include_bytes!`; they are not
loaded from a web frontend or downloaded at runtime. Adding another map requires
a PNG and a matching entry in `map_art`. Maps without artwork show their recorded
name rather than an unrelated image.
