<div align="center">

# 🎧 Small Demo Manager

**A modern, all-in-one demo manager for Counter-Strike 2.**
Browse your library, jump to matches, extract per-player voice chat, and copy in-game spectator commands — in one place.

[![Latest release](https://img.shields.io/github/v/release/thedarkcr0w/Small-Demo-Manager?label=release&color=f59e0b)](https://github.com/thedarkcr0w/Small-Demo-Manager/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/thedarkcr0w/Small-Demo-Manager/total?color=22c55e)](https://github.com/thedarkcr0w/Small-Demo-Manager/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?logo=windows)](#-runtime-requirements)
[![.NET](https://img.shields.io/badge/.NET-9.0-512BD4?logo=dotnet)](https://dotnet.microsoft.com/download/dotnet/9.0)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

<img width="1280" alt="Small Demo Manager — match overview" src="showcase.png" />

</div>

---

## ✨ Features

| | |
|---|---|
| 📁 **Watched folder library** | Point the app at your CS2 / FACEIT / ESEA folders, get a searchable table of every demo with map, score, players, tags, and notes — all persistent. |
| 🎯 **Auto-detect CS2 install** | One-click detection of your CS2 replays folder via the Steam registry + `libraryfolders.vdf`. |
| 🎙️ **Per-player voice extractor** | Pulls out each player's voice chat as `.wav` clips using the Opus decoder — saved under `%LocalAppData%\Small-Demo-Manager\Audio\`. |
| 📣 **Spectator voice command** | Build the `tv_listen_voice_indices` bitfield with a single click and copy it to your clipboard. |
| 🗂️ **Tags, notes & favorites** | Per-demo metadata that survives app updates and folder moves. |
| 🪟 **Move to CS2** | Move any demo into your CS2 replays folder with a hash-verified copy so you can use `playdemo <name>` in-game. |
| 🖱️ **Drag-and-drop import** | Drop a `.dem` from Explorer **or straight out of a 7-Zip / WinRAR archive** — the app extracts and imports automatically. |
| 🔄 **Built-in auto-updater** | New releases are detected and installed from GitHub via **Settings → App update**. |

---

## ⚙️ Runtime Requirements

Windows 10 or 11 with the **[.NET 9.0 Desktop Runtime (x64)](https://dotnet.microsoft.com/download/dotnet/9.0)** installed.

WebView2 (the embedded browser the UI runs in) ships with Windows 11 by default. On Windows 10, install the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) if it isn't already.

> [!IMPORTANT]
> Valve occasionally changes the demo file format with CS2 updates. When that happens the demo parser needs a new version of [`DemoFile`](https://www.nuget.org/packages/DemoFile.Game.Cs) — these updates ship through GitHub releases (and the in-app updater), not the Microsoft Store.

---

## 📥 Installation

1. Download the latest **`SmallDemoManager-shipping.zip`** from [Releases](https://github.com/thedarkcr0w/Small-Demo-Manager/releases/latest).
2. Extract the archive anywhere **except** `Program Files` (so the built-in updater can replace files without UAC prompts).
3. Run `SmallDemoManager.exe`.

That's it — no installer, no admin rights, no registry entries. Settings live under `%LocalAppData%\Small-Demo-Manager\`.

---

## 🙏 Credits & Special Thanks

This project was originally created by **[Pythaeus](https://github.com/pythaeusone)** as the [CS2-SourceTV-Demo-Voice-Calculator](https://github.com/pythaeusone/CS2-SourceTV-Demo-Voice-Calculator) and continues to be maintained in his absence.

Huge thanks to everyone who tests builds and reports issues:

- **[KEROVSKI](https://x.com/KEROVSKI_)** — for inspiring the project's direction
- **[@HaiX](https://x.com/HaiX)**, **[@LobaCS2](https://x.com/LobaCS2)**, **[@neokCS](https://x.com/neokCS)** — for raising awareness about the CS2 cheating problem
- **Throw** from the cswatch.in Discord
- Everyone in the Kerovski Discord helping with testing

---

## 🔗 Links

| | |
|---|---|
| **Repository** | https://github.com/thedarkcr0w/Small-Demo-Manager |
| **Kerovski's Discord** | https://discord.gg/n26tH9565K |
| **KEROVSKI's tool video** | https://www.youtube.com/watch?v=7vsrbD3xBwM |
| **darkcr0w on Steam** | https://steamcommunity.com/id/thedarkcr0w/ |
| **Pythaeus on Steam** | https://steamcommunity.com/id/pythaeus/ |
| **Support darkcr0w** | https://ko-fi.com/darkcr0w |
| **Support Pythaeus** | https://ko-fi.com/pythaeus |

---

## 📦 Built With

**Demo parsing** — [DemoFile](https://www.nuget.org/packages/DemoFile/) · [DemoFile.Game.Cs](https://www.nuget.org/packages/DemoFile.Game.Cs)
**Audio** — [Concentus](https://www.nuget.org/packages/Concentus) (Opus decoder) · [NAudio](https://www.nuget.org/packages/NAudio)
**UI** — [Microsoft.Web.WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) + React (precompiled with esbuild, vendored locally)
**Shell** — [WindowsAPICodePackCore](https://www.nuget.org/packages/WindowsAPICodePackCore) · [WindowsAPICodePackShell](https://www.nuget.org/packages/WindowsAPICodePackShell)

---

## 📅 Roadmap

- [x] Voice extractor
- [x] Voice clip player
- [x] Modernised UI
- [x] Match-results view with per-player stats
- [x] Custom rename function on move
- [x] Watched-folder library
- [x] Auto-detect CS2 install
- [x] Built-in auto-updater
- [x] Drag-and-drop import (incl. archive sources)
- [ ] Background pre-parse for full-library player search
- [ ] Find the match lobby using the demo
- [ ] Video walkthrough

---

<div align="center">

Released under the [MIT License](LICENSE). Commercial use is prohibited.

</div>
