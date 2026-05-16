using System.Text.Json;
using SmallDemoManager.UtilClass;

namespace SmallDemoManager.Bridge
{
    /// <summary>
    /// Persists watched folders + per-demo metadata (favorites, tags, notes, parsed cache)
    /// in a single library.json under %LocalAppData%\Small-Demo-Manager.
    /// </summary>
    public sealed class LibraryStore
    {
        private static readonly string FilePath = Path.Combine(
            LocalAppDataFolder.RootFolderPath, "Library.json");
        private static readonly JsonSerializerOptions JsonOpts = new()
        {
            WriteIndented = true,
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            PropertyNameCaseInsensitive = true,
        };
        private readonly object _gate = new();

        public sealed class Folder
        {
            public string Id { get; set; } = "";
            public string Label { get; set; } = "";
            public string Path { get; set; } = "";
            public string? Source { get; set; }
        }

        // Bump when the demo parser's output shape changes — older caches get re-parsed.
        public const int CurrentParseVersion = 4;

        public sealed class DemoMeta
        {
            public string Id { get; set; } = "";
            public string FullPath { get; set; } = "";
            public string FolderId { get; set; } = "";
            public long Size { get; set; }
            public string Date { get; set; } = "";
            public bool Fav { get; set; }
            public List<string> Tags { get; set; } = new();
            public string Note { get; set; } = "";
            public int ParseVersion { get; set; }
            public DemoDto? Cached { get; set; }
        }

        public sealed class Root
        {
            public List<Folder> Folders { get; set; } = new();
            public Dictionary<string, DemoMeta> Demos { get; set; } = new();
        }

        public Root Data { get; private set; } = new();

        public LibraryStore()
        {
            LocalAppDataFolder.EnsureRootDirectoryExists();
            Load();
        }

        public void Load()
        {
            lock (_gate)
            {
                if (!File.Exists(FilePath)) { Data = new Root(); return; }
                try
                {
                    var json = File.ReadAllText(FilePath);
                    var loaded = JsonSerializer.Deserialize<Root>(json, JsonOpts);
                    Data = loaded ?? new Root();
                }
                catch
                {
                    Data = new Root();
                }
            }
        }

        public void Save()
        {
            lock (_gate)
            {
                LocalAppDataFolder.EnsureRootDirectoryExists();
                var tmp = FilePath + ".tmp";
                File.WriteAllText(tmp, JsonSerializer.Serialize(Data, JsonOpts));
                File.Move(tmp, FilePath, overwrite: true);
            }
        }

        public Folder AddFolder(string path, string? label = null, string? source = null)
        {
            lock (_gate)
            {
                var existing = Data.Folders.FirstOrDefault(f =>
                    string.Equals(f.Path, path, StringComparison.OrdinalIgnoreCase));
                if (existing != null) return existing;

                var f = new Folder
                {
                    Id = "f-" + Guid.NewGuid().ToString("N")[..8],
                    Path = path,
                    Label = label ?? new DirectoryInfo(path).Name,
                    Source = source ?? GuessSourceFromPath(path),
                };
                Data.Folders.Add(f);
                Save();
                return f;
            }
        }

        public bool RemoveFolder(string folderId)
        {
            lock (_gate)
            {
                var idx = Data.Folders.FindIndex(f => f.Id == folderId);
                if (idx < 0) return false;
                Data.Folders.RemoveAt(idx);
                var toRemove = Data.Demos.Where(kv => kv.Value.FolderId == folderId).Select(kv => kv.Key).ToList();
                foreach (var k in toRemove) Data.Demos.Remove(k);
                Save();
                return true;
            }
        }

        public DemoMeta UpsertDemo(string folderId, FileInfo file)
        {
            var id = MakeDemoId(file.FullName);
            lock (_gate)
            {
                if (!Data.Demos.TryGetValue(id, out var meta))
                {
                    // The demo may have moved here from a different folder. Salvage any
                    // orphaned record whose filename + size matches so the user keeps
                    // their favorite/tags/notes across relocations.
                    var fileName = file.Name;
                    var size = file.Length;
                    var orphan = Data.Demos.FirstOrDefault(kv =>
                        kv.Key != id &&
                        kv.Value.Size == size &&
                        string.Equals(Path.GetFileName(kv.Value.FullPath), fileName, StringComparison.OrdinalIgnoreCase) &&
                        !File.Exists(kv.Value.FullPath));

                    if (orphan.Value != null)
                    {
                        meta = orphan.Value;
                        Data.Demos.Remove(orphan.Key);
                        meta.Id = id;
                    }
                    else
                    {
                        meta = new DemoMeta { Id = id };
                    }
                    Data.Demos[id] = meta;
                }
                meta.FullPath = file.FullName;
                meta.FolderId = folderId;
                meta.Size = file.Length;
                meta.Date = file.LastWriteTimeUtc.ToString("o");
                return meta;
            }
        }

        public DemoMeta? GetDemo(string id)
        {
            lock (_gate) return Data.Demos.TryGetValue(id, out var m) ? m : null;
        }

        public DemoMeta? GetDemoByPath(string fullPath)
        {
            var id = MakeDemoId(fullPath);
            return GetDemo(id);
        }

        public void SetFavorite(string id, bool fav)
        {
            lock (_gate)
            {
                if (!Data.Demos.TryGetValue(id, out var m)) return;
                m.Fav = fav;
                Save();
            }
        }

        public void SetTags(string id, List<string> tags)
        {
            lock (_gate)
            {
                if (!Data.Demos.TryGetValue(id, out var m)) return;
                m.Tags = tags.Where(t => !string.IsNullOrWhiteSpace(t)).Distinct().ToList();
                Save();
            }
        }

        public void SetNote(string id, string note)
        {
            lock (_gate)
            {
                if (!Data.Demos.TryGetValue(id, out var m)) return;
                m.Note = note ?? "";
                Save();
            }
        }

        public void CacheParsed(string id, DemoDto dto)
        {
            lock (_gate)
            {
                if (!Data.Demos.TryGetValue(id, out var m)) return;
                m.Cached = dto;
                m.ParseVersion = CurrentParseVersion;
                Save();
            }
        }

        public void RenameDemoId(string oldId, string newPath)
        {
            var newId = MakeDemoId(newPath);
            lock (_gate)
            {
                if (!Data.Demos.TryGetValue(oldId, out var m)) return;
                Data.Demos.Remove(oldId);
                m.Id = newId;
                m.FullPath = newPath;
                if (m.Cached != null)
                {
                    m.Cached.Id = newId;
                    m.Cached.FullPath = newPath;
                    m.Cached.File = Path.GetFileName(newPath);
                }
                Data.Demos[newId] = m;
                Save();
            }
        }

        public static string MakeDemoId(string fullPath)
        {
            // Stable id based on absolute path (lower-cased).
            var norm = Path.GetFullPath(fullPath).ToLowerInvariant();
            var bytes = System.Text.Encoding.UTF8.GetBytes(norm);
            using var sha = System.Security.Cryptography.SHA1.Create();
            var hash = sha.ComputeHash(bytes);
            return "d_" + Convert.ToHexString(hash, 0, 6).ToLowerInvariant();
        }

        public static string GuessSourceFromPath(string path)
        {
            var p = path.ToLowerInvariant();
            if (p.Contains("faceit")) return "Faceit";
            if (p.Contains("esea")) return "ESEA";
            if (p.Contains("scrim")) return "Scrim";
            if (p.Contains("tournament")) return "Tournament";
            if (p.Contains("pug")) return "Pug";
            if (p.Contains("server")) return "Server";
            if (p.Contains("replays") || p.Contains("matchmaking") || p.Contains("\\cs2\\")) return "MM";
            return "Other";
        }

        public static string GuessSourceFromFile(string fileName)
        {
            var n = fileName.ToLowerInvariant();
            if (n.StartsWith("match730_")) return "MM";
            if (n.StartsWith("1-")) return "Faceit";
            if (n.Contains("esea")) return "ESEA";
            if (n.Contains("tournament")) return "Tournament";
            if (n.Contains("scrim")) return "Scrim";
            if (n.Contains("pug")) return "Pug";
            if (n.Contains("server_record")) return "Server";
            return "Other";
        }
    }
}
