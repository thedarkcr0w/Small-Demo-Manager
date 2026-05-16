using System.Diagnostics;
using System.IO.Compression;
using System.Net.Http;
using System.Text;
using System.Text.Json;

namespace SmallDemoManager.Bridge
{
    /// <summary>
    /// Self-update flow: queries GitHub Releases for the latest version, downloads
    /// the shipping zip, writes a small PowerShell updater that waits for the
    /// running process to exit, replaces files, and relaunches.
    /// </summary>
    public static class UpdaterService
    {
        public const string GithubRepo = "thedarkcr0w/Small-Demo-Manager";
        public const string PreferredAssetName = "SmallDemoManager-shipping.zip";

        private static readonly HttpClient Http = CreateHttp();
        private static HttpClient CreateHttp()
        {
            var h = new HttpClient(new HttpClientHandler { AllowAutoRedirect = true });
            h.DefaultRequestHeaders.UserAgent.ParseAdd("SmallDemoManager-Updater/1.0");
            h.Timeout = TimeSpan.FromMinutes(5);
            return h;
        }

        public sealed class UpdateInfo
        {
            public string Current { get; set; } = "";
            public string Latest { get; set; } = "";
            public bool Available { get; set; }
            public string? DownloadUrl { get; set; }
            public string? AssetName { get; set; }
            public long AssetSize { get; set; }
            public string ReleaseUrl { get; set; } = "";
            public string ReleaseNotes { get; set; } = "";
            public string PublishedAt { get; set; } = "";
        }

        public static async Task<UpdateInfo> CheckAsync(CancellationToken ct = default)
        {
            var url = $"https://api.github.com/repos/{GithubRepo}/releases/latest";
            var json = await Http.GetStringAsync(url, ct);
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            var tag = root.TryGetProperty("tag_name", out var tn) ? tn.GetString() ?? "" : "";
            var releaseUrl = root.TryGetProperty("html_url", out var hu) ? hu.GetString() ?? "" : "";
            var notes = root.TryGetProperty("body", out var b) ? b.GetString() ?? "" : "";
            var publishedAt = root.TryGetProperty("published_at", out var pa) ? pa.GetString() ?? "" : "";

            string? downloadUrl = null;
            string? assetName = null;
            long assetSize = 0;
            if (root.TryGetProperty("assets", out var assets) && assets.ValueKind == JsonValueKind.Array)
            {
                JsonElement? preferred = null;
                JsonElement? anyZip = null;
                foreach (var asset in assets.EnumerateArray())
                {
                    var name = asset.GetProperty("name").GetString() ?? "";
                    if (string.Equals(name, PreferredAssetName, StringComparison.OrdinalIgnoreCase))
                    {
                        preferred = asset; break;
                    }
                    if (name.EndsWith(".zip", StringComparison.OrdinalIgnoreCase) && anyZip is null)
                    {
                        anyZip = asset;
                    }
                }
                var picked = preferred ?? anyZip;
                if (picked.HasValue)
                {
                    downloadUrl = picked.Value.GetProperty("browser_download_url").GetString();
                    assetName = picked.Value.GetProperty("name").GetString();
                    assetSize = picked.Value.TryGetProperty("size", out var sz) ? sz.GetInt64() : 0;
                }
            }

            var current = GlobalVersionInfo.GUI_VERSION;
            var available = downloadUrl != null && CompareVersions(tag, current) > 0;

            return new UpdateInfo
            {
                Current = current,
                Latest = tag,
                Available = available,
                DownloadUrl = downloadUrl,
                AssetName = assetName,
                AssetSize = assetSize,
                ReleaseUrl = releaseUrl,
                ReleaseNotes = notes,
                PublishedAt = publishedAt,
            };
        }

        /// <summary>
        /// Strips a leading "v." / "v" / "version " prefix, then parses with
        /// <see cref="Version.TryParse"/>. Returns -1/0/1 like
        /// <see cref="Version.CompareTo"/>; 0 if either side fails to parse.
        /// </summary>
        public static int CompareVersions(string? a, string? b)
        {
            var va = ParseVersion(a);
            var vb = ParseVersion(b);
            if (va == null || vb == null) return 0;
            return va.CompareTo(vb);
        }

        private static Version? ParseVersion(string? s)
        {
            if (string.IsNullOrWhiteSpace(s)) return null;
            var t = s.Trim();
            // Strip common prefixes like "v", "v.", "ver", "version".
            while (t.Length > 0 && !char.IsDigit(t[0])) t = t[1..];
            return Version.TryParse(t, out var v) ? v : null;
        }

        public sealed class DownloadResult
        {
            public string ZipPath { get; set; } = "";
            public string ExtractDir { get; set; } = "";
            public string TempRoot { get; set; } = "";
        }

        public static async Task<DownloadResult> DownloadAndExtractAsync(
            string downloadUrl,
            IProgress<float>? progress = null,
            CancellationToken ct = default)
        {
            var tempRoot = Path.Combine(Path.GetTempPath(),
                "sdm-update-" + Guid.NewGuid().ToString("N")[..8]);
            Directory.CreateDirectory(tempRoot);
            var zipPath = Path.Combine(tempRoot, "update.zip");

            using (var resp = await Http.GetAsync(downloadUrl, HttpCompletionOption.ResponseHeadersRead, ct))
            {
                resp.EnsureSuccessStatusCode();
                var total = resp.Content.Headers.ContentLength ?? -1L;
                await using var src = await resp.Content.ReadAsStreamAsync(ct);
                await using var dst = File.Create(zipPath);
                var buffer = new byte[81920];
                long read = 0;
                int n;
                while ((n = await src.ReadAsync(buffer, ct)) > 0)
                {
                    await dst.WriteAsync(buffer.AsMemory(0, n), ct);
                    read += n;
                    if (total > 0) progress?.Report(Math.Min(1f, (float)read / total));
                }
            }

            var extractDir = Path.Combine(tempRoot, "extracted");
            Directory.CreateDirectory(extractDir);
            ZipFile.ExtractToDirectory(zipPath, extractDir, overwriteFiles: true);

            // If the zip wraps its contents in a single top-level folder, unwrap.
            var entries = Directory.GetFileSystemEntries(extractDir);
            if (entries.Length == 1 && Directory.Exists(entries[0]))
                extractDir = entries[0];

            return new DownloadResult { ZipPath = zipPath, ExtractDir = extractDir, TempRoot = tempRoot };
        }

        /// <summary>
        /// Writes a self-contained PowerShell script that waits for the current
        /// process to exit, mirrors <paramref name="extractDir"/> over the install
        /// dir, then relaunches the app. Spawns it detached and returns the script
        /// path; the caller is expected to call <c>Form.Close()</c> shortly after.
        /// </summary>
        public static string SpawnUpdaterAndDetach(string extractDir, string tempRoot)
        {
            var installDir = AppContext.BaseDirectory.TrimEnd('\\', '/');
            var exePath = Path.Combine(installDir, "SmallDemoManager.exe");
            var currentPid = Environment.ProcessId;
            var logPath = Path.Combine(tempRoot, "update.log");
            var scriptPath = Path.Combine(tempRoot, "apply.ps1");

            // PowerShell script: wait for caller to exit, copy files, relaunch.
            // Single-quoted strings so embedded $ / ` don't get expanded.
            var sb = new StringBuilder();
            sb.AppendLine("$ErrorActionPreference = 'Continue'");
            sb.AppendLine($"$log = '{logPath.Replace("'", "''")}'");
            sb.AppendLine("function Log($m) { Add-Content -Path $log -Value (('[' + (Get-Date -Format HH:mm:ss.fff) + '] ' + $m)) }");
            sb.AppendLine("Log 'updater started'");
            sb.AppendLine($"$pidToWait = {currentPid}");
            sb.AppendLine("for ($i = 0; $i -lt 60; $i++) {");
            sb.AppendLine("  $p = Get-Process -Id $pidToWait -ErrorAction SilentlyContinue");
            sb.AppendLine("  if (-not $p) { break }");
            sb.AppendLine("  Start-Sleep -Milliseconds 500");
            sb.AppendLine("}");
            sb.AppendLine("Start-Sleep -Milliseconds 800");
            sb.AppendLine($"$src = '{extractDir.Replace("'", "''")}'");
            sb.AppendLine($"$dst = '{installDir.Replace("'", "''")}'");
            sb.AppendLine("Log ('copying ' + $src + ' -> ' + $dst)");
            sb.AppendLine("try {");
            sb.AppendLine("  Copy-Item -Path (Join-Path $src '*') -Destination $dst -Recurse -Force -ErrorAction Stop");
            sb.AppendLine("  Log 'copy ok'");
            sb.AppendLine("} catch {");
            sb.AppendLine("  Log ('copy failed: ' + $_.Exception.Message)");
            sb.AppendLine("  exit 1");
            sb.AppendLine("}");
            sb.AppendLine($"$exe = '{exePath.Replace("'", "''")}'");
            sb.AppendLine("Log ('launching ' + $exe)");
            sb.AppendLine("Start-Process -FilePath $exe");
            sb.AppendLine($"$tempRoot = '{tempRoot.Replace("'", "''")}'");
            sb.AppendLine("Start-Sleep -Milliseconds 1500");
            sb.AppendLine("try { Remove-Item -Path $tempRoot -Recurse -Force -ErrorAction SilentlyContinue } catch {}");
            sb.AppendLine("Log 'done'");
            File.WriteAllText(scriptPath, sb.ToString());

            Process.Start(new ProcessStartInfo
            {
                FileName = "powershell.exe",
                Arguments = $"-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File \"{scriptPath}\"",
                CreateNoWindow = true,
                UseShellExecute = false,
                WindowStyle = ProcessWindowStyle.Hidden,
            });

            return scriptPath;
        }
    }
}
