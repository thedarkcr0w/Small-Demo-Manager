use anyhow::{Context, Result, ensure};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let pattern = format!(r#""{}"\s*"((?:\\.|[^"\\])*)""#, regex::escape(key));
    let Ok(re) = Regex::new(&pattern) else {
        return vec![];
    };
    re.captures_iter(text)
        .map(|c| c[1].replace("\\\\", "\\").replace("\\\"", "\""))
        .collect()
}

pub fn detect() -> Result<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::*};
        for (hive, subkey, value) in [
            (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
            (
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\WOW6432Node\Valve\Steam",
                "InstallPath",
            ),
        ] {
            if let Ok(key) = RegKey::predef(hive).open_subkey(subkey)
                && let Ok(path) = key.get_value::<String, _>(value)
            {
                roots.push(PathBuf::from(path));
            }
        }
        roots.extend([
            PathBuf::from(r"C:\Program Files (x86)\Steam"),
            PathBuf::from(r"C:\Program Files\Steam"),
        ]);
    }
    #[cfg(not(windows))]
    if let Some(home) = directories::BaseDirs::new() {
        roots.extend([
            home.home_dir().join(".steam/steam"),
            home.home_dir().join(".local/share/Steam"),
        ]);
    }
    for root in roots {
        let mut libraries = vec![root.clone()];
        if let Ok(text) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            libraries.extend(vdf_values(&text, "path").into_iter().map(PathBuf::from));
        }
        for library in libraries {
            let manifest = library.join("steamapps/appmanifest_730.acf");
            let Ok(text) = fs::read_to_string(manifest) else {
                continue;
            };
            let Some(install) = vdf_values(&text, "installdir").into_iter().next() else {
                continue;
            };
            // Do not allow a malformed manifest to redirect writes outside common/.
            if Path::new(&install).components().count() != 1 {
                continue;
            }
            let csgo = library
                .join("steamapps/common")
                .join(install)
                .join("game/csgo");
            if validate_destination(&csgo).is_ok() {
                return Ok(csgo);
            }
        }
    }
    anyhow::bail!(
        "CS2 was not found in Steam's libraries. Browse to its game/csgo directory in Settings"
    )
}

pub fn validate_destination(path: &Path) -> Result<()> {
    ensure!(
        !path.as_os_str().is_empty(),
        "Set the CS2 game/csgo directory first"
    );
    ensure!(
        path.is_dir(),
        "Directory does not exist: {}",
        path.display()
    );
    let game = if path
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case("replays"))
    {
        path.parent().context("No parent directory")?
    } else {
        path
    };
    ensure!(
        game.join("pak01_dir.vpk").is_file() || game.join("gameinfo.gi").is_file(),
        "Not a CS2 game/csgo directory (gameinfo.gi / pak01_dir.vpk missing)"
    );
    Ok(())
}

pub fn play_command(path: &Path, cs2_path: &Path) -> String {
    let base = if cs2_path
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case("replays"))
    {
        cs2_path.parent().unwrap_or(cs2_path)
    } else {
        cs2_path
    };
    let normalized = crate::files::display_path(path).replace('\\', "/");
    let root = crate::files::display_path(base)
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned();
    let relative = if !root.is_empty()
        && normalized
            .to_lowercase()
            .starts_with(&(root.to_lowercase() + "/"))
    {
        &normalized[root.len() + 1..]
    } else {
        &normalized
    };
    let safe = relative.replace(['"', ';', '\r', '\n'], "");
    format!("playdemo \"{safe}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_escaped_library_paths() {
        assert_eq!(
            vdf_values(
                r#""libraryfolders" { "1" { "path" "D:\\SteamLibrary" } }"#,
                "path"
            ),
            [r"D:\SteamLibrary"]
        );
    }
    #[test]
    fn empty_or_non_game_paths_are_not_valid() {
        assert!(validate_destination(Path::new("")).is_err());
        let dir = tempfile::tempdir().expect("dir");
        assert!(validate_destination(dir.path()).is_err());
    }

    #[test]
    fn commands_preserve_the_replays_subdirectory_and_remove_extended_prefixes() {
        assert_eq!(
            play_command(
                Path::new(r"\\?\C:\CS2\game\csgo\replays\match.dem"),
                Path::new(r"C:\CS2\game\csgo\replays")
            ),
            "playdemo \"replays/match.dem\""
        );
        assert_eq!(
            play_command(Path::new(r"C:\Demos\match.dem"), Path::new("")),
            "playdemo \"C:/Demos/match.dem\""
        );
    }
}
