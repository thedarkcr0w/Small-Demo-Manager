use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

pub const PARSER_VERSION: &str = "source2-demo-0.5.10/sdm-4";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Folder {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub cs2_path: PathBuf,
    pub move_files: bool,
    pub backup_before_move: bool,
    pub background_parse: bool,
    pub check_for_updates: bool,
    pub layout: Layout,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cs2_path: PathBuf::new(),
            move_files: false,
            backup_before_move: true,
            background_parse: false,
            check_for_updates: true,
            layout: Layout::Hybrid,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Layout {
    Table,
    Cards,
    #[default]
    Hybrid,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileStamp {
    pub size: u64,
    pub modified_ns: u128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Demo {
    pub id: String,
    pub path: PathBuf,
    pub folder_id: String,
    pub stamp: FileStamp,
    pub fingerprint: String,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub note: String,
    pub parsed: Option<MatchData>,
    pub parse_error: Option<String>,
    #[serde(skip)]
    pub available: bool,
}

impl Demo {
    pub fn filename(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    pub fn cache_valid(&self) -> bool {
        self.parsed
            .as_ref()
            .is_some_and(|p| p.parser_version == PARSER_VERSION)
    }

    pub fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.filename().to_lowercase().contains(&query)
            || self.note.to_lowercase().contains(&query)
            || self.tags.iter().any(|t| t.to_lowercase().contains(&query))
            || self.parsed.as_ref().is_some_and(|m| {
                m.map.to_lowercase().contains(&query)
                    || m.server.to_lowercase().contains(&query)
                    || m.teams
                        .iter()
                        .any(|t| t.name.to_lowercase().contains(&query))
                    || m.players.iter().any(|p| {
                        p.name.to_lowercase().contains(&query)
                            || p.steam_id.as_deref().is_some_and(|s| s.contains(&query))
                    })
            })
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchData {
    pub parser_version: String,
    pub map: String,
    pub server: String,
    pub game_build: u32,
    pub network_protocol: Option<u32>,
    pub ticks: u32,
    pub tick_interval: Option<f32>,
    pub duration: Option<f64>,
    pub teams: [Team; 2],
    pub players: Vec<Player>,
    pub rounds: Vec<Round>,
    pub completed_rounds: Option<u32>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Team {
    pub name: String,
    pub score: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub steam_id: Option<String>,
    /// Controller entity index, 1-based. Not the reconnect-sensitive user ID.
    pub entity_index: Option<u32>,
    /// Stable roster team, independent of CT/T side switches.
    pub team: Option<usize>,
    pub kills: Option<u32>,
    pub deaths: Option<u32>,
    pub assists: Option<u32>,
    pub damage: Option<u32>,
    pub headshot_kills: Option<u32>,
    pub score: Option<u32>,
    pub bot: bool,
}

impl Player {
    pub fn key(&self) -> String {
        self.steam_id
            .clone()
            .unwrap_or_else(|| format!("entity:{}:{}", self.entity_index.unwrap_or(0), self.name))
    }

    pub fn adr(&self, completed_rounds: Option<u32>) -> Option<f32> {
        let rounds = completed_rounds.filter(|r| *r > 0)?;
        Some(self.damage? as f32 / rounds as f32)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Round {
    pub number: u32,
    pub start_tick: u32,
    pub end_tick: u32,
    pub winner: Option<usize>,
    pub reason: Option<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub player: String,
    pub steam_id: Option<String>,
    pub round: Option<u32>,
    pub demo_seconds: f64,
    pub duration: f64,
    pub path: PathBuf,
    /// Measured normalized peak amplitudes, not a seeded visualization.
    pub peaks: Vec<f32>,
    #[serde(default)]
    pub kind: ClipKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipKind {
    #[default]
    Segment,
    Everyone,
    PlayerTrack,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub schema: u32,
    pub settings: Settings,
    pub folders: Vec<Folder>,
    pub demos: BTreeMap<String, Demo>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            schema: 1,
            settings: Settings::default(),
            folders: vec![],
            demos: BTreeMap::new(),
        }
    }
}

/// The engine has separate low/high 32-bit masks for player slots 1..=64.
/// Refuse invalid slots rather than generating a plausible, incorrect command.
pub fn voice_command(indices: impl IntoIterator<Item = u32>) -> anyhow::Result<String> {
    let mut mask = 0_u64;
    for index in indices {
        anyhow::ensure!(
            (1..=64).contains(&index),
            "Player slot {index} is outside the supported 1–64 range"
        );
        mask |= 1_u64 << (index - 1);
    }
    anyhow::ensure!(
        mask != 0,
        "Select players with recorded controller indices first"
    );
    // Convars are signed 32-bit integers; preserve bit 31/63 with a signed cast.
    Ok(format!(
        "tv_listen_voice_indices {}; tv_listen_voice_indices_h {}",
        mask as u32 as i32,
        (mask >> 32) as u32 as i32
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_masks_cover_both_words_and_boundaries() {
        assert_eq!(
            voice_command([1, 3, 33]).expect("valid"),
            "tv_listen_voice_indices 5; tv_listen_voice_indices_h 1"
        );
        assert_eq!(
            voice_command([32, 64]).expect("valid"),
            "tv_listen_voice_indices -2147483648; tv_listen_voice_indices_h -2147483648"
        );
        assert!(voice_command([]).is_err());
        assert!(voice_command([0]).is_err());
        assert!(voice_command([65]).is_err());
    }

    #[test]
    fn unavailable_stats_stay_unavailable() {
        assert_eq!(Player::default().adr(Some(12)), None);
        assert_eq!(
            Player {
                damage: Some(1200),
                ..Default::default()
            }
            .adr(Some(12)),
            Some(100.0)
        );
    }

    #[test]
    fn existing_settings_get_the_auto_update_default_without_losing_preferences() {
        let settings: Settings =
            serde_json::from_str(r#"{"background_parse":false,"move_files":true}"#)
                .expect("settings");
        assert!(settings.check_for_updates);
        assert!(!settings.background_parse);
        assert!(settings.move_files);
    }
}
