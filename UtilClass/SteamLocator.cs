using System.Text.RegularExpressions;
using Microsoft.Win32;

namespace SmallDemoManager.UtilClass
{
    /// <summary>
    /// Locates the CS2 replays directory by inspecting Steam's registry entries and
    /// scanning every configured library folder. CS2 = Steam app id 730 (the listing
    /// is still labelled "Counter-Strike Global Offensive" because the install folder
    /// name carried over from CSGO).
    /// </summary>
    public static class SteamLocator
    {
        // App-id 730 is CS2 (formerly CSGO). The install folder name carried over
        // from CSGO. Demos placed in <install>\game\csgo\ are playable in-game
        // via the `playdemo <name>` console command (which resolves relative to
        // the game's working directory, not the replays subfolder).
        private const string Cs2AppId = "730";
        private const string Cs2InstallDir = "Counter-Strike Global Offensive";
        private const string Cs2DemoSubPath = @"game\csgo";
        private const string Cs2ExeSubPath = @"game\bin\win64\cs2.exe";

        public static string? FindCs2ReplaysFolder()
        {
            var info = FindCs2Install();
            if (info == null) return null;
            // Return <install>\game\csgo so a demo dropped here is callable
            // with `playdemo <filename>` from the in-game console.
            return Path.Combine(info.InstallDir, Cs2DemoSubPath);
        }

        public sealed class Cs2Install
        {
            public string LibraryRoot { get; init; } = "";
            public string InstallDir { get; init; } = "";
        }

        public static Cs2Install? FindCs2Install()
        {
            foreach (var lib in EnumerateLibraries())
            {
                // Strongest signal: Steam tracks the install via this manifest. Its mere
                // presence (alongside an installdir entry pointing at our install folder)
                // confirms the game is installed in this library.
                var manifest = Path.Combine(lib, "steamapps", $"appmanifest_{Cs2AppId}.acf");
                if (!File.Exists(manifest)) continue;

                var installDirName = ReadManifestInstallDir(manifest) ?? Cs2InstallDir;
                var installDir = Path.Combine(lib, "steamapps", "common", installDirName);

                // Sanity check: at least one of the canonical sub-paths should exist.
                // Either the cs2 executable (definitive proof of a CS2 install) or the
                // csgo folder (works even on partial installs).
                if (Directory.Exists(installDir) &&
                    (File.Exists(Path.Combine(installDir, Cs2ExeSubPath)) ||
                     Directory.Exists(Path.Combine(installDir, "game", "csgo"))))
                {
                    return new Cs2Install { LibraryRoot = lib, InstallDir = installDir };
                }
            }

            // Fallback: some users move their CS2 install around without updating Steam's
            // manifest cleanly. As a last resort, accept any library that contains a
            // CS2 folder.
            foreach (var lib in EnumerateLibraries())
            {
                var installDir = Path.Combine(lib, "steamapps", "common", Cs2InstallDir);
                if (File.Exists(Path.Combine(installDir, Cs2ExeSubPath)))
                    return new Cs2Install { LibraryRoot = lib, InstallDir = installDir };
            }
            return null;
        }

        public static IEnumerable<string> EnumerateLibraries()
        {
            var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            var steamRoot = FindSteamRoot();
            if (!string.IsNullOrEmpty(steamRoot) && seen.Add(steamRoot))
                yield return steamRoot;

            if (string.IsNullOrEmpty(steamRoot)) yield break;

            // libraryfolders.vdf lists every library Steam knows about, including those
            // on drives other than the one Steam itself was installed on.
            var libFile = Path.Combine(steamRoot, "steamapps", "libraryfolders.vdf");
            if (!File.Exists(libFile)) yield break;

            string content;
            try { content = File.ReadAllText(libFile); }
            catch { yield break; }

            foreach (Match m in Regex.Matches(content, "\"path\"\\s*\"([^\"]+)\""))
            {
                var p = m.Groups[1].Value.Replace("\\\\", "\\");
                if (Directory.Exists(p) && seen.Add(p))
                    yield return p;
            }
        }

        private static string? ReadManifestInstallDir(string manifestPath)
        {
            try
            {
                var text = File.ReadAllText(manifestPath);
                var m = Regex.Match(text, "\"installdir\"\\s*\"([^\"]+)\"");
                if (m.Success) return m.Groups[1].Value.Replace("\\\\", "\\");
            }
            catch { }
            return null;
        }

        private static string? FindSteamRoot()
        {
            // HKCU first — most reliable when Steam is installed per-user.
            try
            {
                using var key = Registry.CurrentUser.OpenSubKey(@"Software\Valve\Steam");
                if (key?.GetValue("SteamPath") is string p && Directory.Exists(p))
                    return p.Replace('/', '\\');
            }
            catch { }

            // Fall back to the system-wide registration.
            foreach (var hive in new[] {
                @"SOFTWARE\WOW6432Node\Valve\Steam",
                @"SOFTWARE\Valve\Steam",
            })
            {
                try
                {
                    using var key = Registry.LocalMachine.OpenSubKey(hive);
                    if (key?.GetValue("InstallPath") is string p && Directory.Exists(p))
                        return p;
                }
                catch { }
            }

            // Last-ditch: well-known default locations.
            foreach (var candidate in new[] {
                @"C:\Program Files (x86)\Steam",
                @"C:\Program Files\Steam",
            })
            {
                if (Directory.Exists(candidate)) return candidate;
            }
            return null;
        }
    }
}
