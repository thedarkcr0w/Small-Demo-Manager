# Patch Notes

## 31.10.2026 — 2.0.1

### [ GENERAL ]
- Fixed long startup stalls when loading and rescanning large demo libraries.
- Reduced time spent reading and saving library data and voice manifests.
- Library layouts now draw only the visible rows and cards.

### [ LIBRARY ]
- Stopped importing the old app's library and settings. Old data files are left untouched.
- Folders previously imported from the old library are no longer watched. Add the folders you want to use again.
- Folder scans only list demos. Parsing starts when a demo is clicked or dropped into the app.
- Fixed drops being rejected in an empty library or while another demo was parsing.
- Added cancellation during demo validation and sign-on. These stages no longer show a misleading 0%.

### [ VOICE ]
- Extract all now creates one everyone track and separate player tracks.
- Click a player to hear their voice in the second playback row. Original clips are kept.
- Combined tracks keep demo order, mix overlapping voices and skip silent gaps between clips.

### [ MISC ]
- Removed documentation, readmes and screenshots from the Windows ZIP. Required license notices are bundled together.

## 30.10.2026 — 2.0.0

### [ GENERAL ]
- Rebuilt Small Demo Manager in Rust. The dark/amber layout is unchanged.
- Removed the .NET and WebView2 runtime requirements.
- Added table, card and hybrid library layouts.

### [ DEMOS ]
- Fixed parsing of CS2 string tables and game-mode data.
- Added support for AnimGraph2 demo fixtures and recordings without final metadata.
- Fixed missing round results in demos without round-start or round-end events.
- Added a specific error for incomplete downloads and truncated recordings.
- Missing player IDs and stats are no longer filled with generated values.

### [ VOICE ]
- Fixed Opus packet offsets and voice clips being split into thousands of small files.
- Waveform previews are now generated from the recorded audio.
- Playback progress now follows the audio device.
- Fixed spectator voice commands for player slots above 32.

### [ LIBRARY ]
- Added folder watching and optional background parsing for player search.
- Fixed cached match data not refreshing after a demo changed.
- Notes, tags and favorites are kept when renaming or moving a demo in the app.
- Added verified backups for moves. Existing destination files are not overwritten.
- Added ZIP import checks and Recycle Bin deletion.

### [ UPDATES ]
- Added automatic GitHub release checks and background downloads.
- Update packages are checked against their SHA-256 before installation.
- Added restart-to-install confirmation and rollback if installation or startup fails.
- Previous .NET packages are not offered as updates to this build.

### [ MISC ]
- Removed the old C#/React application, duplicate builds and unused assets.
- Existing settings and annotations are imported without modifying the old data files.
- New code is licensed to darkcr0w. Personal, non-commercial use is allowed; all other rights reserved. Existing third-party licenses remain unchanged.

### [ KNOWN ISSUES ]
- Voice export is WAV only. MP3 and FLAC are not included in this build.
- Imports support `.dem` and ZIP files. Extract RAR/7z archives before importing.
- Existing audio from the old app is kept on disk but is not added to the new clip browser.
- Demos recorded on the newest CS2 patch have not been fully tested.
- The Windows executable is unsigned.

---

## Previous .NET Application

These dated entries are preserved from the former `PatchNotes.rtf`. They describe
the old application, not additional features promised by the native preview.

### 23.12.2025

* Update Parser to read older demos.

### 20.11.2025

* The new demo versions have caused a loop.

### 18.10.2025

* The last selection when moving the demo file is now saved.

### 07.10.2025

* Fix a small Bitfield Calc. bug

### 23.09.2025

* Renaming demo when moving improved

### 22.09.2025

* Parser adapted to new demo version dated 17 September 2025

### 08.09.2025

* Fixed an error when loading the patch notes.
* The option for the Shell Context Menu has been removed from the settings.

### 03.09.2025

* Revise tab blocking during import, etc.
* After selecting a CS2 path, this is now also displayed correctly in the settings.
* Bots, spectators, or admins are skipped when reading the demo.

### 02.09.2025

* The Home page has been completed.
* The About page has been completed.
* MessageBox and other dialogue boxes revised

### 01.09.2025

* The Settings page has been completed.

### 31.08.2025

* Voice audio extractor logic implemented.
* Improve Audioplayer, start/stop.

### 21.08.2025

* Dark and Light mode added.
* Design created for Voice Audio Extractor.

### 20.08.2025

* Tab page for match details created.
* Evaluate game data and integrate it into match details.
* Performance improved for Material Design.

### 19.08.2025

* Improved demo loading.
* Player data expanded.
* Bitfield calculator integrated into the new GUI.
* Tab page lock class created.

### 18.08.2025

* Project created for version 2.0.
* The new GUI design has been created.
* All controls and events have been bound.
