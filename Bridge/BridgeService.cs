using System.Diagnostics;
using System.Text.Json;
using SmallDemoManager.AudioExtract;
using SmallDemoManager.UtilClass;

namespace SmallDemoManager.Bridge
{
    /// <summary>
    /// JSON message dispatcher between the React UI (WebView2) and the .NET backend.
    /// Request format: { id, type, payload }
    /// Response format: { id, ok, result | error }
    /// Event format:    { event: 'name', payload }
    /// </summary>
    public sealed class BridgeService : IDisposable
    {
        private static readonly JsonSerializerOptions JsonOpts = new()
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            PropertyNameCaseInsensitive = true,
            DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull,
        };

        private readonly Form _owner;
        private readonly LibraryStore _store = new();
        private readonly DemoIndex _index;
        private Action<string>? _post;
        private CancellationTokenSource? _voiceCts;
        private CancellationTokenSource? _scanCts;

        // Active streaming-import sessions: sessionId → (open file stream, dest path).
        private sealed class ImportSession
        {
            public FileStream Stream { get; init; } = null!;
            public string Path { get; init; } = "";
        }
        private readonly Dictionary<string, ImportSession> _importSessions = new();
        private readonly object _importLock = new();

        public string? StartupDemo { get; set; }
        public Action? WindowDragRequested { get; set; }

        public BridgeService(Form owner)
        {
            _owner = owner;
            _index = new DemoIndex(_store);
            AudioReadHelper.PlaybackEnded += () => Emit("playback-ended", new { });
        }

        public void SetMessagePoster(Action<string> post) => _post = post;

        private static readonly string LogPath = Path.Combine(
            LocalAppDataFolder.RootFolderPath, "bridge.log");
        public static void DebugLog(string msg) => Log(msg);

        private void AbortImportSession(string sessionId)
        {
            ImportSession? sess;
            lock (_importLock)
            {
                if (!_importSessions.TryGetValue(sessionId, out sess)) return;
                _importSessions.Remove(sessionId);
            }
            try { sess.Stream.Dispose(); } catch { }
            try { File.Delete(sess.Path); } catch { }
        }

        private static string MakeUniqueStagingPath(string dir, string fileName)
        {
            var dest = Path.Combine(dir, fileName);
            if (!File.Exists(dest)) return dest;
            var stem = Path.GetFileNameWithoutExtension(fileName);
            var ext = Path.GetExtension(fileName);
            for (int i = 1; i < 1000; i++)
            {
                dest = Path.Combine(dir, $"{stem} ({i}){ext}");
                if (!File.Exists(dest)) return dest;
            }
            return Path.Combine(dir, $"{stem}-{Guid.NewGuid():N}{ext}");
        }

        private static void Log(string msg)
        {
            try
            {
                File.AppendAllText(LogPath,
                    $"[{DateTime.Now:HH:mm:ss.fff}] {msg}{Environment.NewLine}");
            }
            catch { }
        }

        public void HandleMessage(string? json)
        {
            Log($"recv json={(json == null ? "<null>" : json.Length > 200 ? json[..200] + "…" : json)}");
            if (string.IsNullOrWhiteSpace(json)) return;
            JsRequest? req;
            try { req = JsonSerializer.Deserialize<JsRequest>(json, JsonOpts); }
            catch (Exception ex) { Log("deserialize failed: " + ex.Message); return; }
            if (req == null || string.IsNullOrEmpty(req.Type)) { Log("empty req"); return; }
            Log($"dispatching id={req.Id} type={req.Type}");

            _ = Task.Run(async () =>
            {
                try
                {
                    var result = await Dispatch(req);
                    Log($"dispatched id={req.Id} ok");
                    Reply(req.Id, result);
                }
                catch (Exception ex)
                {
                    Log($"dispatch threw id={req.Id}: {ex}");
                    Fail(req.Id, ex.Message);
                }
            });
        }

        private async Task<object?> Dispatch(JsRequest req)
        {
            switch (req.Type)
            {
                case "getInitialState":
                    return BuildInitialState();

                case "addFolder":
                    return await UiInvoke(() =>
                    {
                        var picked = PathPicker.PickFolder("Add a demo folder to watch");
                        if (string.IsNullOrEmpty(picked)) return (object?)null;
                        var folder = _store.AddFolder(picked);
                        var demos = _index.ListDemosInFolder(folder).ToList();
                        return new { folder = ToFolderDto(folder, demos.Count), demos };
                    });

                case "removeFolder":
                {
                    var folderId = req.Payload.GetProperty("folderId").GetString();
                    return _store.RemoveFolder(folderId ?? "");
                }

                case "scanAll":
                    _scanCts?.Cancel();
                    _scanCts = new CancellationTokenSource();
                    return await ScanAllAsync(_scanCts.Token);

                case "scanFolder":
                {
                    var folderId = req.Payload.GetProperty("folderId").GetString() ?? "";
                    var folder = _store.Data.Folders.FirstOrDefault(f => f.Id == folderId);
                    if (folder == null) return new { demos = Array.Empty<DemoDto>() };
                    var demos = _index.ListDemosInFolder(folder).ToList();
                    return new { demos };
                }

                case "parseDemo":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    var progress = new Progress<float>(p =>
                        Emit("parse-progress", new { demoId, p }));
                    var dto = await _index.ParseAsync(demoId, progress);
                    if (dto != null) Emit("parse-progress", new { demoId, p = 1.0f });
                    return dto;
                }

                case "toggleFavorite":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    var fav = req.Payload.GetProperty("fav").GetBoolean();
                    _store.SetFavorite(demoId, fav);
                    return true;
                }

                case "setTags":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    var tagsEl = req.Payload.GetProperty("tags");
                    var tags = tagsEl.EnumerateArray().Select(e => e.GetString() ?? "").ToList();
                    _store.SetTags(demoId, tags);
                    return true;
                }

                case "setNote":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    var note = req.Payload.GetProperty("note").GetString() ?? "";
                    _store.SetNote(demoId, note);
                    return true;
                }

                case "extractVoice":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    return await ExtractVoiceAsync(demoId);
                }

                case "listVoiceClips":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    return ListVoiceClips(demoId);
                }

                case "playClip":
                {
                    var path = req.Payload.GetProperty("path").GetString() ?? "";
                    return AudioReadHelper.Play(path);
                }

                case "stopClip":
                    AudioReadHelper.Stop();
                    return true;

                case "revealInFolder":
                {
                    var path = req.Payload.GetProperty("path").GetString() ?? "";
                    return RevealInExplorer(path);
                }

                case "openExternal":
                {
                    var url = req.Payload.GetProperty("url").GetString() ?? "";
                    return OpenExternal(url);
                }

                case "moveToCs2":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    string? newName = null;
                    if (req.Payload.TryGetProperty("newName", out var nn) && nn.ValueKind == JsonValueKind.String)
                        newName = nn.GetString();
                    return await MoveToCs2Async(demoId, newName);
                }

                case "dropDebug":
                {
                    // Surfaces what was in dataTransfer for drops we couldn't parse —
                    // helps diagnose archive viewers / non-standard drag sources.
                    Log("dropDebug: " + req.Payload.GetRawText());
                    return true;
                }

                case "importChunkBegin":
                {
                    var sessionId = req.Payload.GetProperty("sessionId").GetString() ?? "";
                    var rawName = req.Payload.GetProperty("name").GetString() ?? "";
                    if (string.IsNullOrEmpty(sessionId) || string.IsNullOrEmpty(rawName))
                        return new { ok = false, error = "missing session/name" };

                    var staging = LocalAppDataFolder.EnsureSubDirectoryExists("DropStaging");
                    var safe = string.Concat(rawName.Where(c => !Path.GetInvalidFileNameChars().Contains(c)));
                    if (!safe.EndsWith(".dem", StringComparison.OrdinalIgnoreCase)) safe += ".dem";
                    var dest = MakeUniqueStagingPath(staging, safe);
                    try
                    {
                        var fs = new FileStream(dest, FileMode.CreateNew, FileAccess.Write, FileShare.None, 1 << 16);
                        lock (_importLock) _importSessions[sessionId] = new ImportSession { Stream = fs, Path = dest };
                        Log($"importChunkBegin {sessionId} -> {dest}");
                        return new { ok = true };
                    }
                    catch (Exception ex)
                    {
                        Log("importChunkBegin failed: " + ex.Message);
                        return new { ok = false, error = ex.Message };
                    }
                }

                case "importChunkData":
                {
                    var sessionId = req.Payload.GetProperty("sessionId").GetString() ?? "";
                    var b64 = req.Payload.GetProperty("dataBase64").GetString() ?? "";
                    ImportSession? sess;
                    lock (_importLock) _importSessions.TryGetValue(sessionId, out sess);
                    if (sess == null) return new { ok = false, error = "unknown session" };
                    try
                    {
                        var bytes = Convert.FromBase64String(b64);
                        await sess.Stream.WriteAsync(bytes);
                        return new { ok = true };
                    }
                    catch (Exception ex)
                    {
                        Log("importChunkData failed: " + ex.Message);
                        AbortImportSession(sessionId);
                        return new { ok = false, error = ex.Message };
                    }
                }

                case "importChunkEnd":
                {
                    var sessionId = req.Payload.GetProperty("sessionId").GetString() ?? "";
                    ImportSession? sess;
                    lock (_importLock) {
                        _importSessions.TryGetValue(sessionId, out sess);
                        if (sess != null) _importSessions.Remove(sessionId);
                    }
                    if (sess == null) return new { ok = false, error = "unknown session" };
                    try { await sess.Stream.FlushAsync(); sess.Stream.Dispose(); }
                    catch (Exception ex) { Log("importChunkEnd flush failed: " + ex.Message); }
                    Log($"importChunkEnd {sessionId} -> {sess.Path}");
                    return new { ok = true, path = sess.Path };
                }

                case "importChunkAbort":
                {
                    // Find the in-flight session for this file name (best-effort) and clean up.
                    var name = req.Payload.TryGetProperty("name", out var n) ? n.GetString() ?? "" : "";
                    string? toAbort = null;
                    lock (_importLock)
                    {
                        foreach (var kv in _importSessions)
                        {
                            if (kv.Value.Path.EndsWith(name, StringComparison.OrdinalIgnoreCase))
                            { toAbort = kv.Key; break; }
                        }
                    }
                    if (toAbort != null) AbortImportSession(toAbort);
                    return new { ok = true };
                }

                case "importChunkFinalize":
                {
                    // JS finished all chunks; emit files-dropped so React picks them up.
                    var pathsEl = req.Payload.GetProperty("paths");
                    var paths = pathsEl.EnumerateArray().Select(e => e.GetString() ?? "").Where(p => !string.IsNullOrWhiteSpace(p)).ToArray();
                    if (paths.Length > 0) Emit("files-dropped", new { paths, staged = true });
                    return new { ok = true };
                }

                case "dragActive":
                {
                    var active = req.Payload.TryGetProperty("active", out var a) && a.ValueKind == JsonValueKind.True;
                    Emit("drag-active", new { active });
                    return true;
                }

                case "importDemoFiles":
                {
                    var folderId = req.Payload.GetProperty("folderId").GetString() ?? "";
                    var pathsEl = req.Payload.GetProperty("paths");
                    var paths = pathsEl.EnumerateArray()
                        .Select(e => e.GetString() ?? "")
                        .Where(p => !string.IsNullOrWhiteSpace(p))
                        .ToList();
                    bool cleanupSource = req.Payload.TryGetProperty("cleanupSource", out var cs)
                        && cs.ValueKind == JsonValueKind.True;
                    return ImportDemoFiles(folderId, paths, cleanupSource);
                }

                case "deleteDemo":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    return DeleteDemo(demoId);
                }

                case "renameDemo":
                {
                    var demoId = req.Payload.GetProperty("demoId").GetString() ?? "";
                    var newName = req.Payload.GetProperty("newName").GetString() ?? "";
                    return RenameDemo(demoId, newName);
                }

                case "copyToClipboard":
                {
                    var text = req.Payload.GetProperty("text").GetString() ?? "";
                    return await UiInvoke(() => { Clipboard.SetText(text); return true; });
                }

                case "windowMinimize":
                    return await UiInvoke(() => { _owner.WindowState = FormWindowState.Minimized; return true; });

                case "windowMaximize":
                    return await UiInvoke(() =>
                    {
                        _owner.WindowState = _owner.WindowState == FormWindowState.Maximized
                            ? FormWindowState.Normal : FormWindowState.Maximized;
                        return true;
                    });

                case "windowClose":
                    return await UiInvoke(() => { _owner.Close(); return true; });

                case "windowDrag":
                    WindowDragRequested?.Invoke();
                    return true;

                case "checkForUpdate":
                    return await UpdaterService.CheckAsync();

                case "applyUpdate":
                {
                    var dlUrl = req.Payload.GetProperty("downloadUrl").GetString() ?? "";
                    if (string.IsNullOrEmpty(dlUrl))
                        return new { ok = false, error = "Missing downloadUrl" };
                    return await ApplyUpdateAsync(dlUrl);
                }

                case "detectCs2Path":
                    return new { path = SteamLocator.FindCs2ReplaysFolder() ?? "" };

                case "pickCs2Folder":
                    return await UiInvoke(() =>
                        new { path = PathPicker.PickFolder("Select your CS2 replays folder") ?? "" });

                case "saveSettings":
                {
                    if (req.Payload.TryGetProperty("cs2Path", out var p) && p.ValueKind == JsonValueKind.String)
                        JsonClass.WriteJson("CS2DemoPath", p.GetString() ?? "");
                    if (req.Payload.TryGetProperty("moveOnImport", out var mi) && mi.ValueKind != JsonValueKind.Undefined)
                        JsonClass.WriteJson("MoveOnImport", mi.GetBoolean());
                    if (req.Payload.TryGetProperty("autoBackup", out var ab) && ab.ValueKind != JsonValueKind.Undefined)
                        JsonClass.WriteJson("AutoBackup", ab.GetBoolean());
                    return true;
                }

                default:
                    throw new InvalidOperationException("Unknown request type: " + req.Type);
            }
        }

        private InitialStateDto BuildInitialState()
        {
            var demos = _index.ListDemos();
            var byFolder = demos.GroupBy(d => d.FolderId).ToDictionary(g => g.Key, g => g.Count());
            var folders = _store.Data.Folders.Select(f => ToFolderDto(f, byFolder.GetValueOrDefault(f.Id, 0))).ToList();
            return new InitialStateDto
            {
                Folders = folders,
                Demos = demos,
                StartupDemo = StartupDemo,
                Cs2Path = JsonClass.ReadJson<string>("CS2DemoPath") ?? "",
                MoveOnImport = JsonClass.KeyExists("MoveOnImport")
                    ? JsonClass.ReadJson<bool>("MoveOnImport") : true,
                AutoBackup = JsonClass.KeyExists("AutoBackup")
                    && JsonClass.ReadJson<bool>("AutoBackup"),
                AppVersion = GlobalVersionInfo.GUI_VERSION,
            };
        }

        private static WatchedFolderDto ToFolderDto(LibraryStore.Folder f, int count) => new()
        {
            Id = f.Id,
            Label = f.Label,
            Path = f.Path,
            Source = f.Source,
            Count = count,
        };

        private async Task<object> ScanAllAsync(CancellationToken ct)
        {
            int total = _store.Data.Folders.Count;
            int done = 0;
            var allDemos = new List<DemoDto>();
            foreach (var folder in _store.Data.Folders.ToList())
            {
                if (ct.IsCancellationRequested) break;
                Emit("scan-progress", new {
                    progress = total == 0 ? 100 : (done * 100 / Math.Max(1, total)),
                    path = folder.Path,
                    found = allDemos.Count,
                });
                var demos = _index.ListDemosInFolder(folder).ToList();
                allDemos.AddRange(demos);
                done++;
            }
            Emit("scan-progress", new { progress = 100, path = "", found = allDemos.Count });
            return new { demos = allDemos };
        }

        private async Task<object?> ExtractVoiceAsync(string demoId)
        {
            var meta = _store.GetDemo(demoId);
            if (meta == null || !File.Exists(meta.FullPath)) return null;

            _voiceCts?.Cancel();
            _voiceCts = new CancellationTokenSource();
            var progress = new Progress<float>(p =>
                Emit("extract-progress", new { demoId, p }));

            var ok = await AudioExtractor.ExtractAsync(meta.FullPath, progress);
            if (!ok) return new { ok = false, clips = Array.Empty<VoiceClipDto>() };

            return new { ok = true, clips = ListVoiceClips(demoId) };
        }

        private List<VoiceClipDto> ListVoiceClips(string demoId)
        {
            var meta = _store.GetDemo(demoId);
            if (meta == null) return new();
            var demoName = Path.GetFileNameWithoutExtension(meta.FullPath);
            var baseDir = Path.Combine(LocalAppDataFolder.RootFolderPath, "Audio", demoName);
            if (!Directory.Exists(baseDir)) return new();

            // AudioExtractor writes folders named after the player when their controller
            // is still present at end-of-stream, and falls back to the SteamID64 otherwise.
            // Use the cached demo parse to build a SteamID -> player-name lookup so the
            // React UI can match clips back to its player list.
            var idToName = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            if (meta.Cached != null)
            {
                foreach (var p in meta.Cached.Players1.Concat(meta.Cached.Players2))
                {
                    if (!string.IsNullOrEmpty(p.SteamId) && !idToName.ContainsKey(p.SteamId))
                        idToName[p.SteamId] = p.Name;
                }
            }

            var clips = new List<VoiceClipDto>();
            foreach (var playerDir in Directory.GetDirectories(baseDir))
            {
                var folderName = Path.GetFileName(playerDir);
                // If the folder name is a 17-digit steamid, try to resolve to the player's real name.
                var displayName = folderName;
                var steamId = "";
                if (folderName.Length == 17 && folderName.All(char.IsDigit))
                {
                    steamId = folderName;
                    if (idToName.TryGetValue(folderName, out var realName))
                        displayName = realName;
                }
                else
                {
                    // It's already a player name; reverse-look up SteamID for completeness.
                    steamId = idToName.FirstOrDefault(kv => string.Equals(kv.Value, folderName, StringComparison.OrdinalIgnoreCase)).Key ?? "";
                }

                var entries = AudioReadHelper.GetAudioEntries(folderName, baseDir);
                foreach (var entry in entries)
                {
                    if (entry.FilePath == null) continue;
                    clips.Add(new VoiceClipDto
                    {
                        Id = LibraryStore.MakeDemoId(entry.FilePath),
                        Player = displayName,
                        SteamId = steamId,
                        Round = entry.Round,
                        DemoSec = (int)entry.Time.TotalSeconds,
                        Dur = Math.Round(entry.DurationSeconds, 2),
                        Format = "wav",
                        Path = entry.FilePath,
                    });
                }
            }
            return clips.OrderBy(c => c.Player).ThenBy(c => c.Round).ToList();
        }

        private async Task<object?> MoveToCs2Async(string demoId, string? newName)
        {
            Log($"moveToCs2 start demoId={demoId} newName={newName ?? "<null>"}");
            var meta = _store.GetDemo(demoId);
            if (meta == null)
            {
                Log("moveToCs2: meta is null");
                return new { ok = false, error = "Demo not in library" };
            }
            if (!File.Exists(meta.FullPath))
            {
                Log($"moveToCs2: source file not found at {meta.FullPath}");
                return new { ok = false, error = $"Source file not found: {meta.FullPath}" };
            }

            var cs2Path = PathPicker.ReadPath("CS2DemoPath");
            Log($"moveToCs2: read CS2DemoPath={cs2Path ?? "<null>"}");
            if (string.IsNullOrEmpty(cs2Path))
            {
                Log("moveToCs2: prompting user for CS2 folder");
                cs2Path = await UiInvoke(() => PathPicker.GetPath("Select the CS2 Demo folder", "CS2DemoPath"));
                Log($"moveToCs2: picker returned {cs2Path ?? "<cancelled>"}");
                if (string.IsNullOrEmpty(cs2Path))
                    return new { ok = false, error = "No CS2 demo folder selected" };
            }
            // The replays folder is created lazily by CS2 (and by us) on first use, so
            // accept any path whose parent directory exists and create it on demand.
            if (!Directory.Exists(cs2Path))
            {
                try
                {
                    var parent = Path.GetDirectoryName(cs2Path);
                    if (parent == null || !Directory.Exists(parent))
                        return new { ok = false, error = $"CS2 path's parent doesn't exist: {cs2Path}" };
                    Directory.CreateDirectory(cs2Path);
                    Log($"moveToCs2: created {cs2Path}");
                }
                catch (Exception ex)
                {
                    return new { ok = false, error = "Could not create CS2 demo folder: " + ex.Message };
                }
            }

            try
            {
                var fileName = !string.IsNullOrWhiteSpace(newName)
                    ? Path.ChangeExtension(newName!.Trim(), ".dem")
                    : Path.GetFileName(meta.FullPath);
                var target = Path.Combine(cs2Path, fileName);
                Log($"moveToCs2: source={meta.FullPath} target={target}");
                if (File.Exists(target))
                {
                    Log("moveToCs2: target already exists");
                    return new { ok = false, error = "A file with that name already exists in the CS2 folder" };
                }

                var sourceHash = meta.FullPath.ComputeFileHash();
                File.Move(meta.FullPath, target);
                var destHash = target.ComputeFileHash();
                if (!sourceHash.HashesAreEqual(destHash))
                {
                    Log("moveToCs2: hash mismatch");
                    return new { ok = false, error = "Hash mismatch after move" };
                }

                _store.RenameDemoId(meta.Id, target);
                Log($"moveToCs2: success → {target}");
                return new { ok = true, newPath = target };
            }
            catch (Exception ex)
            {
                Log($"moveToCs2 threw: {ex}");
                return new { ok = false, error = ex.Message };
            }
        }

        private object ImportDemoFiles(string folderId, List<string> sourcePaths, bool cleanupSource)
        {
            Log($"importDemoFiles folderId={folderId} count={sourcePaths.Count} cleanup={cleanupSource}");
            var folder = _store.Data.Folders.FirstOrDefault(f => f.Id == folderId);
            if (folder == null) return new { ok = false, error = "Watched folder not found" };
            if (!Directory.Exists(folder.Path))
            {
                try { Directory.CreateDirectory(folder.Path); }
                catch (Exception ex)
                {
                    return new { ok = false, error = "Folder unavailable: " + ex.Message };
                }
            }

            var imported = new List<DemoDto>();
            var errors = new List<string>();
            foreach (var src in sourcePaths)
            {
                var name = Path.GetFileName(src);
                if (!File.Exists(src)) { errors.Add($"{name}: not found"); continue; }
                if (!src.EndsWith(".dem", StringComparison.OrdinalIgnoreCase))
                { errors.Add($"{name}: not a .dem file"); continue; }

                try
                {
                    // If the source is already inside this folder, just index it in place
                    // rather than copying onto itself.
                    var dest = string.Equals(Path.GetDirectoryName(src), folder.Path,
                        StringComparison.OrdinalIgnoreCase)
                        ? src
                        : ResolveUniquePath(folder.Path, name);

                    if (dest != src) File.Copy(src, dest, overwrite: false);

                    var fi = new FileInfo(dest);
                    var meta = _store.UpsertDemo(folder.Id, fi);
                    imported.Add(DemoIndex.BuildDto(meta, folder, fi));

                    // Only delete the source if it was a staged copy (e.g., extracted from
                    // an archive into our drop-staging dir) and we actually copied it elsewhere.
                    if (cleanupSource && dest != src)
                    {
                        try { File.Delete(src); }
                        catch (Exception ex) { Log($"importDemoFiles cleanup failed for {src}: {ex.Message}"); }
                    }
                }
                catch (Exception ex)
                {
                    Log($"importDemoFiles failed for {src}: {ex.Message}");
                    errors.Add($"{name}: {ex.Message}");
                }
            }

            _store.Save();
            return new { ok = imported.Count > 0, demos = imported, errors };
        }

        private static string ResolveUniquePath(string folder, string fileName)
        {
            var dest = Path.Combine(folder, fileName);
            if (!File.Exists(dest)) return dest;
            var stem = Path.GetFileNameWithoutExtension(fileName);
            var ext = Path.GetExtension(fileName);
            for (int i = 1; i < 1000; i++)
            {
                var candidate = Path.Combine(folder, $"{stem} ({i}){ext}");
                if (!File.Exists(candidate)) return candidate;
            }
            throw new IOException("No unique filename available");
        }

        private object DeleteDemo(string demoId)
        {
            var meta = _store.GetDemo(demoId);
            if (meta == null) return new { ok = false };
            try
            {
                if (File.Exists(meta.FullPath)) File.Delete(meta.FullPath);
                _store.Data.Demos.Remove(demoId);
                _store.Save();
                return new { ok = true };
            }
            catch (Exception ex)
            {
                return new { ok = false, error = ex.Message };
            }
        }

        private object RenameDemo(string demoId, string newName)
        {
            var meta = _store.GetDemo(demoId);
            if (meta == null || !File.Exists(meta.FullPath))
                return new { ok = false, error = "File not found" };
            try
            {
                var safe = Path.ChangeExtension(newName.Trim(), ".dem");
                var target = Path.Combine(Path.GetDirectoryName(meta.FullPath)!, safe);
                if (File.Exists(target)) return new { ok = false, error = "Name already taken" };
                File.Move(meta.FullPath, target);
                _store.RenameDemoId(demoId, target);
                return new { ok = true, newPath = target, newId = LibraryStore.MakeDemoId(target) };
            }
            catch (Exception ex)
            {
                return new { ok = false, error = ex.Message };
            }
        }

        private async Task<object> ApplyUpdateAsync(string downloadUrl)
        {
            try
            {
                Log("applyUpdate: downloading " + downloadUrl);
                var progress = new Progress<float>(p =>
                    Emit("update-progress", new { p }));
                var result = await UpdaterService.DownloadAndExtractAsync(downloadUrl, progress);
                Log($"applyUpdate: extracted to {result.ExtractDir}");

                UpdaterService.SpawnUpdaterAndDetach(result.ExtractDir, result.TempRoot);
                Log("applyUpdate: updater spawned, closing app");

                // Give the JS bridge a moment to ack before we close.
                _ = Task.Run(async () =>
                {
                    await Task.Delay(400);
                    await UiInvoke(() => { _owner.Close(); return true; });
                });
                return new { ok = true };
            }
            catch (Exception ex)
            {
                Log("applyUpdate failed: " + ex);
                return new { ok = false, error = ex.Message };
            }
        }

        private static bool RevealInExplorer(string path)
        {
            try
            {
                if (!File.Exists(path) && !Directory.Exists(path)) return false;
                Process.Start(new ProcessStartInfo
                {
                    FileName = "explorer.exe",
                    Arguments = File.Exists(path) ? $"/select,\"{path}\"" : $"\"{path}\"",
                    UseShellExecute = true,
                });
                return true;
            }
            catch { return false; }
        }

        private static bool OpenExternal(string url)
        {
            try
            {
                if (!Uri.TryCreate(url, UriKind.Absolute, out var uri)) return false;
                if (uri.Scheme != Uri.UriSchemeHttps && uri.Scheme != Uri.UriSchemeHttp) return false;
                Process.Start(new ProcessStartInfo { FileName = url, UseShellExecute = true });
                return true;
            }
            catch { return false; }
        }

        private void Reply(int id, object? result)
        {
            if (id == 0) return;
            string json;
            try { json = JsonSerializer.Serialize(new { id, ok = true, result }, JsonOpts); }
            catch (Exception ex) { Log($"reply serialize failed id={id}: {ex}"); Fail(id, "Serialize failed: " + ex.Message); return; }
            Log($"reply id={id} len={json.Length}");
            _post?.Invoke(json);
        }

        private void Fail(int id, string error)
        {
            if (id == 0) return;
            var json = JsonSerializer.Serialize(new { id, ok = false, error }, JsonOpts);
            Log($"fail id={id} error={error}");
            _post?.Invoke(json);
        }

        public void Emit(string evt, object payload)
        {
            var json = JsonSerializer.Serialize(new { @event = evt, payload }, JsonOpts);
            _post?.Invoke(json);
        }

        private Task<T> UiInvoke<T>(Func<T> action)
        {
            var tcs = new TaskCompletionSource<T>();
            if (_owner.InvokeRequired)
                _owner.BeginInvoke(() =>
                {
                    try { tcs.SetResult(action()); }
                    catch (Exception ex) { tcs.SetException(ex); }
                });
            else
            {
                try { tcs.SetResult(action()); }
                catch (Exception ex) { tcs.SetException(ex); }
            }
            return tcs.Task;
        }

        public void Dispose()
        {
            _voiceCts?.Cancel();
            _scanCts?.Cancel();
            AudioReadHelper.Stop();
        }
    }
}
