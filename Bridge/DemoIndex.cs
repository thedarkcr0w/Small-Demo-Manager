using DemoFile;
using DemoFile.Game.Cs;
using SmallDemoManager.HelperClass;

namespace SmallDemoManager.Bridge
{
    /// <summary>
    /// File-level scanning of watched folders + full demo parsing (cached).
    /// </summary>
    public sealed class DemoIndex
    {
        private readonly LibraryStore _store;

        public DemoIndex(LibraryStore store)
        {
            _store = store;
        }

        public List<DemoDto> ListDemos()
        {
            var result = new List<DemoDto>();
            foreach (var folder in _store.Data.Folders)
            {
                foreach (var demo in ListDemosInFolder(folder))
                    result.Add(demo);
            }
            return result;
        }

        public IEnumerable<DemoDto> ListDemosInFolder(LibraryStore.Folder folder)
        {
            if (!Directory.Exists(folder.Path)) yield break;
            string[] files;
            try
            {
                files = Directory.GetFiles(folder.Path, "*.dem", SearchOption.TopDirectoryOnly);
            }
            catch
            {
                yield break;
            }
            foreach (var path in files)
            {
                FileInfo fi;
                try { fi = new FileInfo(path); }
                catch { continue; }
                var meta = _store.UpsertDemo(folder.Id, fi);
                yield return BuildDto(meta, folder, fi);
            }
        }

        public DemoDto? GetDemoDto(string demoId)
        {
            var meta = _store.GetDemo(demoId);
            if (meta == null) return null;
            var folder = _store.Data.Folders.FirstOrDefault(f => f.Id == meta.FolderId);
            if (folder == null) return null;
            if (!File.Exists(meta.FullPath)) return null;
            return BuildDto(meta, folder, new FileInfo(meta.FullPath));
        }

        public static DemoDto BuildDto(LibraryStore.DemoMeta meta, LibraryStore.Folder folder, FileInfo fi)
        {
            DemoDto dto;
            // Only trust the cache if it was produced by the current parser version.
            // Older caches will be lazily re-parsed when the user selects them.
            if (meta.Cached != null && meta.ParseVersion == LibraryStore.CurrentParseVersion)
            {
                dto = meta.Cached;
                dto.FullPath = meta.FullPath;
                dto.File = fi.Name;
                dto.FolderId = folder.Id;
                dto.FolderPath = folder.Path;
                dto.Size = Math.Round(fi.Length / (1024.0 * 1024.0), 1);
                dto.Date = fi.LastWriteTime.ToString("yyyy-MM-ddTHH:mm:ss");
                dto.Parsed = true;
            }
            else
            {
                dto = new DemoDto
                {
                    Id = meta.Id,
                    File = fi.Name,
                    FullPath = fi.FullName,
                    FolderId = folder.Id,
                    FolderPath = folder.Path,
                    Map = "de_unknown",
                    Date = fi.LastWriteTime.ToString("yyyy-MM-ddTHH:mm:ss"),
                    Size = Math.Round(fi.Length / (1024.0 * 1024.0), 1),
                    Tick = 64,
                    Source = LibraryStore.GuessSourceFromFile(fi.Name) is var src && src != "Other"
                        ? src : (folder.Source ?? "Other"),
                    Server = "",
                    T1 = "Team A",
                    T2 = "Team B",
                    Parsed = false,
                };
            }
            dto.Id = meta.Id;
            dto.Fav = meta.Fav;
            dto.Tags = meta.Tags.ToList();
            dto.Note = meta.Note;
            return dto;
        }

        /// <summary>
        /// Parse a demo file → fill players, scores, map, duration, rounds; cache result.
        /// </summary>
        public async Task<DemoDto?> ParseAsync(string demoId, IProgress<float>? progress = null,
            CancellationToken ct = default)
        {
            var meta = _store.GetDemo(demoId);
            if (meta == null) return null;
            var folder = _store.Data.Folders.FirstOrDefault(f => f.Id == meta.FolderId);
            if (folder == null) return null;
            if (!File.Exists(meta.FullPath)) return null;

            var fi = new FileInfo(meta.FullPath);
            var dto = BuildDto(meta, folder, fi);
            try
            {
                await ParseInto(dto, ct, progress);
                dto.Parsed = true;
                meta.Cached = dto;
                _store.CacheParsed(demoId, dto);
            }
            catch (Exception ex)
            {
                dto.ParseError = ex.Message;
            }
            return dto;
        }

        private static async Task ParseInto(DemoDto dto, CancellationToken ct, IProgress<float>? progress)
        {
            var demo = new CsDemoParser();
            var snapshots = new List<PlayerSnapshot>();
            var rounds = new List<string>();
            int totalTicks = 0;
            int tickRate = 64;
            int? ctTeamScore = null, tTeamScore = null;
            string? mapName = null;
            string? hostName = null;
            string? team1Name = null;
            string? team2Name = null;

            demo.Source1GameEvents.RoundEnd += e =>
            {
                snapshots.Clear();
                foreach (var team in new[] { demo.TeamCounterTerrorist, demo.TeamTerrorist })
                {
                    foreach (var p in team.CSPlayerControllers)
                    {
                        if (string.IsNullOrWhiteSpace(p.PlayerName)) continue;
                        var matchStats = p.ActionTrackingServices?.MatchStats ?? new CSMatchStats();
                        if (p.PlayerInfo?.Fakeplayer == true) continue;
                        if (matchStats.LiveTime == 0 ||
                            (matchStats.Kills + matchStats.Deaths + matchStats.Assists + matchStats.Damage) == 0)
                            continue;

                        double kd = matchStats.Deaths > 0 ? (double)matchStats.Kills / matchStats.Deaths : matchStats.Kills;
                        double hsPercent = matchStats.Kills > 0
                            ? (double)matchStats.HeadShotKills * 100 / matchStats.Kills : 0;

                        snapshots.Add(new PlayerSnapshot
                        {
                            UserId = (int)p.EntityIndex.Value,
                            PlayerName = p.PlayerName!,
                            TeamNumber = (int)p.CSTeamNum,
                            TeamName = string.IsNullOrWhiteSpace(p.Team.ClanTeamname)
                                ? null : p.Team.ClanTeamname,
                            PlayerSteamID = p.PlayerInfo?.Steamid ?? p.SteamID,
                            Kills = matchStats.Kills,
                            Death = matchStats.Deaths,
                            Assists = matchStats.Assists,
                            HeadShotKill = matchStats.HeadShotKills,
                            HeadShotPerecent = hsPercent,
                            Kd = kd,
                            Damage = matchStats.Damage,
                            Score = p.Score,
                            UtilityDamage = matchStats.UtilityDamage,
                            EnemiesFlashed = matchStats.EnemiesFlashed,
                            MVP = p.MVPs,
                            ThreeK = matchStats.Enemy3Ks,
                            FourK = matchStats.Enemy4Ks,
                            FiveK = matchStats.Enemy5Ks,
                            EndScore = p.Team.Score,
                        });
                    }
                }

                // Determine the winning side from score deltas, then tag the round with
                // the side ("CT"/"T") plus a snapshot of which clan is on each side right
                // now. Final A/B mapping is resolved after parsing (we need the end-of-game
                // clan-to-side assignment).
                int newCt = demo.TeamCounterTerrorist.Score;
                int newT = demo.TeamTerrorist.Score;
                string? winSide = null;
                if (ctTeamScore is int prevCt && tTeamScore is int prevT)
                {
                    // Both totals reset to 0 → mp_restartgame fired (end of warmup, knife
                    // round restart, etc.). Discard anything we counted up to here.
                    bool bothReset = newCt == 0 && newT == 0 && (prevCt > 0 || prevT > 0);
                    // Half-time can flip the CT/T team objects (clans swap sides) so the
                    // recorded scores literally swap. Same total but ct↔t — not a new round.
                    bool halfSwap = prevCt != prevT && newCt == prevT && newT == prevCt;

                    if (bothReset) rounds.Clear();
                    else if (halfSwap) { /* not a real round */ }
                    else if (newCt > prevCt) winSide = "CT";
                    else if (newT > prevT) winSide = "T";
                }
                else if (newCt + newT == 1)
                {
                    winSide = newCt == 1 ? "CT" : "T";
                }
                if (winSide != null)
                {
                    var ctClan = demo.TeamCounterTerrorist.ClanTeamname ?? "";
                    var tClan = demo.TeamTerrorist.ClanTeamname ?? "";
                    rounds.Add($"{winSide}|{ctClan}|{tClan}");
                }

                ctTeamScore = newCt;
                tTeamScore = newT;

                mapName = demo.ServerInfo?.MapName ?? mapName;
                hostName = demo.ServerInfo?.HostName ?? hostName;
                team1Name = demo.TeamCounterTerrorist.ClanTeamname;
                team2Name = demo.TeamTerrorist.ClanTeamname;
            };

            using var stream = new FileStream(dto.FullPath, FileMode.Open, FileAccess.Read,
                FileShare.Read, 4096 * 1024);
            var reader = DemoFileReader.Create(demo, stream);

            var progressTask = Task.Run(async () =>
            {
                while (!ct.IsCancellationRequested)
                {
                    try
                    {
                        var ratio = (double)stream.Position / Math.Max(1, stream.Length);
                        progress?.Report((float)Math.Clamp(ratio, 0, 1));
                    }
                    catch { }
                    await Task.Delay(200, ct).ContinueWith(_ => { });
                    if (stream.Position >= stream.Length) break;
                }
            }, ct);

            await reader.ReadAllAsync(ct);

            totalTicks = demo.TickCount.Value;
            tickRate = (int)CsDemoParser.TickRate;
            mapName ??= demo.ServerInfo?.MapName;
            hostName ??= demo.ServerInfo?.HostName;

            var ctPlayers = snapshots.Where(p => p.TeamNumber == 3).ToList();
            var tPlayers = snapshots.Where(p => p.TeamNumber == 2).ToList();

            dto.Map = NormalizeMap(mapName);
            dto.Server = hostName ?? "";
            dto.Tick = tickRate;
            dto.Dur = totalTicks > 0 ? totalTicks / Math.Max(1, tickRate) : 0;
            dto.S1 = ctPlayers.FirstOrDefault()?.EndScore ?? 0;
            dto.S2 = tPlayers.FirstOrDefault()?.EndScore ?? 0;
            dto.T1 = string.IsNullOrWhiteSpace(ctPlayers.FirstOrDefault()?.TeamName)
                ? "CT side" : ctPlayers.First().TeamName!;
            dto.T2 = string.IsNullOrWhiteSpace(tPlayers.FirstOrDefault()?.TeamName)
                ? "T side" : tPlayers.First().TeamName!;
            dto.Players1 = ctPlayers.Select(p => ToPlayerDto(p, "A", dto.Dur)).ToList();
            dto.Players2 = tPlayers.Select(p => ToPlayerDto(p, "B", dto.Dur)).ToList();

            // Resolve per-round A/B labels.
            // "Team A" is the clan currently on CT at end of demo (dto.T1).
            // For each round entry "winSide|ctClan|tClan":
            //   if the round's winning side's clan == dto.T1's clan → "A", else "B".
            var t1Clan = ctPlayers.FirstOrDefault()?.TeamName ?? "";
            var t2Clan = tPlayers.FirstOrDefault()?.TeamName ?? "";
            var hasClans = !string.IsNullOrWhiteSpace(t1Clan) && t1Clan != t2Clan;

            int finalCtScore = dto.S1, finalTScore = dto.S2;
            // For clan-less demos, fall back to side+half-swap heuristic.
            int halfSwapAt = (finalCtScore + finalTScore) >= 24 ? 12
                            : (finalCtScore + finalTScore) >= 30 ? 15 : int.MaxValue;

            int idx = 0;
            dto.Rounds = rounds.Select(entry =>
            {
                var parts = entry.Split('|');
                var winSide = parts[0];
                var ctClanRound = parts.Length > 1 ? parts[1] : "";
                var tClanRound = parts.Length > 2 ? parts[2] : "";
                bool roundIsA;
                if (hasClans)
                {
                    var winClan = winSide == "CT" ? ctClanRound : tClanRound;
                    roundIsA = winClan == t1Clan;
                }
                else
                {
                    bool swapped = idx >= halfSwapAt;
                    bool ctSideIsTeamA = !swapped; // assume "Team A" starts on CT
                    roundIsA = (winSide == "CT") == ctSideIsTeamA;
                }
                idx++;
                return roundIsA ? "A" : "B";
            }).ToList();
        }

        private static PlayerDto ToPlayerDto(PlayerSnapshot p, string team, int durSeconds)
        {
            int adr = durSeconds > 0 ? (int)Math.Round(p.Damage / Math.Max(1, durSeconds / 105.0)) : 0;
            double rating = 0.45 * Math.Min(2.0, p.Kd) + 0.5 * Math.Min(1.5, adr / 80.0)
                          + 0.05 * Math.Min(1.0, p.MVP / 5.0);
            return new PlayerDto
            {
                Name = p.PlayerName ?? "Unknown",
                Team = team,
                SteamId = p.PlayerSteamID?.ToString() ?? "",
                K = p.Kills,
                D = p.Death,
                A = p.Assists,
                Hs = (int)Math.Round(p.HeadShotPerecent),
                Adr = adr,
                Rating = rating.ToString("0.00", System.Globalization.CultureInfo.InvariantCulture),
                Flagged = false,
                Favorite = false,
                UserId = p.UserId,
            };
        }

        private static readonly HashSet<string> KnownMaps = new(StringComparer.OrdinalIgnoreCase)
        {
            "de_mirage","de_inferno","de_dust2","de_nuke","de_overpass",
            "de_anubis","de_ancient","de_vertigo","de_train",
            "de_basalt","de_brewery","de_dogtown","de_jura",
        };

        private static string NormalizeMap(string? raw)
        {
            if (string.IsNullOrWhiteSpace(raw)) return "de_unknown";
            var m = raw.Trim().ToLowerInvariant();
            if (KnownMaps.Contains(m)) return m;
            // Sometimes parsed as "de_mirage_se" or with full paths; try last segment.
            var seg = m.Split('/', '\\').Last();
            if (KnownMaps.Contains(seg)) return seg;
            return m;
        }
    }
}
