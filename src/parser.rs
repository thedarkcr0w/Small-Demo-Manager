use crate::{audio::AudioWriter, files, model::*};
use anyhow::{Context as _, Result, ensure};
use source2_demo::{
    prelude::*,
    proto::{CDemoFileHeader, CSvcMsgServerInfo, CSvcMsgVoiceData},
};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::BufReader,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub type Progress = Arc<dyn Fn(f32) + Send + Sync>;

pub struct Parsed {
    pub game: MatchData,
    pub clips: Vec<Clip>,
}

pub fn parse(
    path: &Path,
    cancel: Arc<AtomicBool>,
    progress: Progress,
    audio: Option<AudioWriter>,
) -> Result<Parsed> {
    // Header/envelope validation and sign-on data do not have a tick percentage.
    // Show indeterminate progress instead of a misleading 0%, and allow Cancel
    // before the entity decoder has reached its first normal tick.
    progress(-1.0);
    files::validate_demo_cancellable(path, &cancel)?;
    let before = files::stamp(path)?;
    // A bad upstream decoder must not unwind through the native event loop.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Parsed> {
        let input = BufReader::with_capacity(1024 * 1024, File::open(path)?);
        let mut parser = Parser::from_reader(input).context("Read CS2 demo header")?;
        let info = parser.replay_info().clone();
        let warnings = if info.playback_ticks.is_none() {
            vec![
                "No final metadata block recorded; results cover the available demo stream only"
                    .to_owned(),
            ]
        } else {
            vec![]
        };
        let observer = parser.add_observer(MatchObserver {
            cancel,
            progress: progress.clone(),
            audio,
            total_ticks: info.playback_ticks().max(0) as u32,
            game: MatchData {
                parser_version: PARSER_VERSION.to_owned(),
                warnings,
                duration: info
                    .playback_time
                    .filter(|t| t.is_finite() && *t > 0.0)
                    .map(f64::from),
                ..Default::default()
            },
            ..Default::default()
        });
        parser.run_to_end().context("Decode CS2 demo stream")?;
        let mut observer = observer.borrow_mut();
        observer.snapshot(parser.context());
        if parser.context().game_build() != 0 {
            observer.game.game_build = parser.context().game_build();
        }
        observer.game.ticks = observer.max_tick;
        if let Some(interval) = observer.game.tick_interval {
            observer.game.duration = Some(observer.max_tick as f64 * interval as f64);
        }
        let mut players: Vec<_> = observer.players.values().cloned().collect();
        players.sort_by(|a, b| {
            a.team
                .cmp(&b.team)
                .then(b.score.cmp(&a.score))
                .then(b.kills.cmp(&a.kills))
                .then(a.name.cmp(&b.name))
        });
        observer.game.players = players;
        if observer.game.players.is_empty() {
            observer
                .game
                .warnings
                .push("No player-controller data was recorded in this demo".into());
        }
        if observer.game.tick_interval.is_none() {
            observer.game.warnings.push(
                "Tick interval is missing; voice timestamps cannot be calculated reliably".into(),
            );
        }
        if observer.game.rounds.is_empty() {
            observer.game.warnings.push(
                "No completed competitive rounds recorded; ADR and round timeline are unavailable"
                    .into(),
            );
        }
        ensure!(
            files::stamp(path)? == before,
            "Demo changed during parsing; result was not cached"
        );
        ensure!(
            !observer.cancel.load(Ordering::Relaxed),
            "Operation cancelled"
        );
        let game = observer.game.clone();
        let clips = if let Some(audio) = observer.audio.take() {
            audio.finish(&game.players)?
        } else {
            vec![]
        };
        progress(1.0);
        Ok(Parsed { game, clips })
    }));
    match result {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "The demo decoder rejected a malformed or unsupported stream. Original demo was not changed; include this demo's build number when reporting compatibility issues"
        ),
    }
}

struct MatchObserver {
    game: MatchData,
    players: BTreeMap<String, Player>,
    roster: HashMap<String, usize>,
    side_team: HashMap<u32, usize>,
    total_ticks: u32,
    max_tick: u32,
    current_round_start: u32,
    current_round: Option<u32>,
    previous_scores: Option<[u32; 2]>,
    warmup: bool,
    live: bool,
    cancel: Arc<AtomicBool>,
    progress: Progress,
    last_progress: Instant,
    audio: Option<AudioWriter>,
}

impl Default for MatchObserver {
    fn default() -> Self {
        Self {
            game: MatchData::default(),
            players: BTreeMap::new(),
            roster: HashMap::new(),
            side_team: HashMap::new(),
            total_ticks: 0,
            max_tick: 0,
            current_round_start: 0,
            current_round: None,
            previous_scores: None,
            warmup: false,
            live: false,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(|_| {}),
            last_progress: Instant::now(),
            audio: None,
        }
    }
}

pub fn uint(entity: &Entity, name: &str) -> Option<u32> {
    match entity.get_property(name).ok()? {
        FieldValue::Unsigned8(v) => Some(*v as u32),
        FieldValue::Unsigned16(v) => Some(*v as u32),
        FieldValue::Unsigned32(v) => Some(*v),
        FieldValue::Unsigned64(v) => u32::try_from(*v).ok(),
        FieldValue::Signed8(v) => u32::try_from(*v).ok(),
        FieldValue::Signed16(v) => u32::try_from(*v).ok(),
        FieldValue::Signed32(v) => u32::try_from(*v).ok(),
        FieldValue::Signed64(v) => u32::try_from(*v).ok(),
        _ => None,
    }
}

fn text(entity: &Entity, name: &str) -> Option<String> {
    match entity.get_property(name).ok()? {
        FieldValue::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn flag(entity: &Entity, name: &str) -> Option<bool> {
    match entity.get_property(name).ok()? {
        FieldValue::Boolean(v) => Some(*v),
        _ => None,
    }
}

fn steam_id(entity: &Entity) -> Option<String> {
    match entity.get_property("m_steamID").ok()? {
        FieldValue::Unsigned64(v) if *v != 0 => Some(v.to_string()),
        _ => None,
    }
}

fn event_int(event: &GameEvent<'_>, name: &str) -> Option<i32> {
    match event.get_value(name).ok()? {
        EventValue::Int(v) => Some(*v),
        EventValue::Byte(v) => Some(*v as i32),
        _ => None,
    }
}

impl MatchObserver {
    fn report_progress(&mut self, tick: u32) -> ObserverResult {
        ensure!(!self.cancel.load(Ordering::Relaxed), "Operation cancelled");
        if self.last_progress.elapsed() >= Duration::from_millis(100) {
            (self.progress)(if self.total_ticks > 0 && tick > 0 && tick != u32::MAX {
                (tick as f32 / self.total_ticks as f32).clamp(0.0, 0.999)
            } else {
                -1.0
            });
            self.last_progress = Instant::now();
        }
        Ok(())
    }

    fn snapshot(&mut self, ctx: &source2_demo::Context) {
        if let Ok(rules) = ctx.entities().get_by_class_name("CCSGameRulesProxy") {
            self.warmup = flag(rules, "m_pGameRules.m_bWarmupPeriod").unwrap_or(false);
        }
        let controllers: Vec<_> = ctx
            .entities()
            .iter()
            .filter(|e| e.class().name() == "CCSPlayerController")
            .collect();
        // Existing roster members tell us which stable team occupies each side.
        // This works for halftime and every overtime switch without MR12 guesses.
        let mut votes = [[0_usize; 2]; 4];
        for entity in &controllers {
            let Some(side) = uint(entity, "m_iTeamNum").filter(|s| *s == 2 || *s == 3) else {
                continue;
            };
            if let Some(id) = steam_id(entity)
                && let Some(&team) = self.roster.get(&id)
            {
                votes[side as usize][team] += 1;
            }
        }
        for side in [2_u32, 3] {
            let v = votes[side as usize];
            if v[0] != v[1] {
                self.side_team.insert(side, usize::from(v[1] > v[0]));
            }
        }
        // Initial A/B labels are CT/T, then tracked by real roster identity.
        self.side_team.entry(3).or_insert(0);
        self.side_team.entry(2).or_insert(1);
        for entity in controllers {
            let Some(name) = text(entity, "m_iszPlayerName") else {
                continue;
            };
            let side = uint(entity, "m_iTeamNum");
            let steam = steam_id(entity);
            let key = steam
                .clone()
                .unwrap_or_else(|| format!("entity:{}:{name}", entity.index()));
            let old = self.players.get(&key).cloned().unwrap_or_default();
            let team = side
                .and_then(|s| self.side_team.get(&s).copied())
                .or(old.team);
            if !self.warmup
                && let (Some(id), Some(team)) = (&steam, team)
            {
                self.roster.entry(id.clone()).or_insert(team);
            }
            let stat = |suffix: &str| {
                uint(
                    entity,
                    &format!("m_pActionTrackingServices.m_matchStats.{suffix}"),
                )
            };
            let player = Player {
                name,
                steam_id: steam,
                entity_index: Some(entity.index()),
                team,
                kills: stat("m_iKills").or(old.kills),
                deaths: stat("m_iDeaths").or(old.deaths),
                assists: stat("m_iAssists").or(old.assists),
                damage: stat("m_iDamage").or(old.damage),
                headshot_kills: stat("m_iHeadShotKills").or(old.headshot_kills),
                score: uint(entity, "m_iScore").or(old.score),
                bot: flag(entity, "m_bIsHLTV").unwrap_or(false)
                    || (old.bot && old.steam_id.is_none()),
            };
            if player.team.is_some() {
                self.players.insert(key, player);
            }
        }
        for entity in ctx
            .entities()
            .iter()
            .filter(|e| e.class().name() == "CCSTeam")
        {
            let Some(side) = uint(entity, "m_iTeamNum") else {
                continue;
            };
            let Some(&team) = self.side_team.get(&side) else {
                continue;
            };
            if let Some(name) =
                text(entity, "m_szClanTeamname").or_else(|| text(entity, "m_szTeamname"))
            {
                self.game.teams[team].name = name;
            }
            if let Some(score) = uint(entity, "m_iScore") {
                self.game.teams[team].score = Some(score);
            }
        }
        if let (Some(a), Some(b)) = (self.game.teams[0].score, self.game.teams[1].score) {
            self.record_scores([a, b], ctx.tick());
        }
    }

    /// Recent CS2 recordings do not always transmit round_start/round_end.
    /// Record only observed +1 scoreboard transitions, never fill missing rounds
    /// by guessing winners from the final score or an assumed MR12 halftime.
    fn record_scores(&mut self, scores: [u32; 2], tick: u32) {
        let previous = self.previous_scores.replace(scores);
        if self.warmup {
            return;
        }
        let total = scores[0].saturating_add(scores[1]);
        self.game.completed_rounds = Some(total);
        let Some(previous) = previous else {
            return;
        };
        if !self.live || tick == u32::MAX {
            return;
        }
        let winner = if scores[0] == previous[0].saturating_add(1) && scores[1] == previous[1] {
            Some(0)
        } else if scores[1] == previous[1].saturating_add(1) && scores[0] == previous[0] {
            Some(1)
        } else {
            None
        };
        if let Some(winner) = winner
            && !self.game.rounds.iter().any(|r| r.number == total)
        {
            self.game.rounds.push(Round {
                number: total,
                start_tick: self.current_round_start,
                end_tick: tick,
                winner: Some(winner),
                reason: None,
            });
        }
    }
}

impl Observer for MatchObserver {
    fn interests(&self) -> Interests {
        Interests::ENTITY_STATE
            | Interests::STRING_TABLE_STATE
            | Interests::BASE_GAME_EVENT
            | Interests::SVC_MESSAGE
            | Interests::DEMO_MESSAGE
            | Interests::TICK_END
    }

    fn on_demo_command(
        &mut self,
        ctx: &source2_demo::Context,
        kind: EDemoCommands,
        bytes: &[u8],
    ) -> ObserverResult {
        self.report_progress(ctx.tick())?;
        if kind == EDemoCommands::DemFileHeader {
            let header = CDemoFileHeader::decode(bytes)?;
            ensure!(
                header
                    .game
                    .as_deref()
                    .is_none_or(|game| game.is_empty() || matches!(game, "csgo" | "cs2")),
                "This demo records {}, not Counter-Strike 2",
                header.game()
            );
            self.game.game_build = header.build_num().max(0) as u32;
            self.game.network_protocol = header.patch_version.and_then(|v| u32::try_from(v).ok());
            self.game.map = header
                .map_name()
                .replace('\\', "/")
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_owned();
            self.game.server = header.server_name().to_owned();
        }
        Ok(())
    }

    fn on_svc_message(
        &mut self,
        ctx: &source2_demo::Context,
        kind: SvcMessages,
        bytes: &[u8],
    ) -> ObserverResult {
        match kind {
            SvcMessages::SvcServerInfo => {
                let info = CSvcMsgServerInfo::decode(bytes)?;
                if let Some(interval) = info
                    .tick_interval
                    .filter(|v| v.is_finite() && *v > 0.0 && *v < 1.0)
                {
                    self.game.tick_interval = Some(interval);
                }
                if !info.map_name().is_empty() {
                    self.game.map = info
                        .map_name()
                        .replace('\\', "/")
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                        .to_owned();
                }
                if !info.host_name().is_empty() {
                    self.game.server = info.host_name().to_owned();
                }
            }
            SvcMessages::SvcVoiceData => {
                if let Some(audio) = &mut self.audio {
                    let msg = CSvcMsgVoiceData::decode(bytes)?;
                    audio.push(msg, ctx.tick(), self.current_round, self.game.tick_interval)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_tick_end(&mut self, ctx: &source2_demo::Context) -> ObserverResult {
        self.report_progress(ctx.tick())?;
        if ctx.tick() == u32::MAX {
            return Ok(());
        }
        self.max_tick = self.max_tick.max(ctx.tick());
        // One snapshot per recorded second, plus every round boundary.
        if ctx.tick().is_multiple_of(64) {
            self.snapshot(ctx);
        }
        Ok(())
    }

    fn on_game_event(
        &mut self,
        ctx: &source2_demo::Context,
        event: &GameEvent<'_>,
    ) -> ObserverResult {
        if std::env::var_os("SDM_TRACE_DEMO").is_some() && event.name().starts_with("round_") {
            eprintln!(
                "{} tick={} warmup={} live={} winner={:?}",
                event.name(),
                ctx.tick(),
                self.warmup,
                self.live,
                event_int(event, "winner")
            );
        }
        match event.name() {
            "round_announce_match_start" => {
                self.game.rounds.clear();
                self.roster.clear();
                self.side_team.clear();
                self.players.clear();
                self.live = true;
                self.current_round = Some(1);
                self.previous_scores = Some([0, 0]);
            }
            "round_start" | "round_poststart" => {
                self.snapshot(ctx);
                self.current_round_start = ctx.tick();
                if !self.warmup {
                    self.live = true;
                    self.current_round = self.game.completed_rounds.map(|n| n + 1);
                } else {
                    self.current_round = None;
                }
            }
            "round_end" => {
                self.snapshot(ctx);
                if self.warmup || !self.live {
                    return Ok(());
                }
                if let Some(round) = self.game.rounds.last_mut()
                    && Some(round.number) == self.game.completed_rounds
                {
                    round.end_tick = ctx.tick();
                    round.reason = event_int(event, "reason");
                }
            }
            "round_officially_ended" | "cs_win_panel_match" => self.snapshot(ctx),
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use source2_demo::{
        proto::CDemoFileInfo,
        writer::{write_demo_message, write_demo_message_with_compression},
    };

    fn minimal_demo(path: &Path, compressed_header: bool) {
        let mut bytes = b"PBDEMS2\0\0\0\0\0\0\0\0\0".to_vec();
        let header = CDemoFileHeader {
            map_name: Some("de_protocol_test".into()),
            server_name: Some("Fixture server".into()),
            ..Default::default()
        };
        write_demo_message_with_compression(
            &mut bytes,
            EDemoCommands::DemFileHeader,
            0,
            &header.encode_to_vec(),
            compressed_header,
        )
        .expect("header");
        write_demo_message(&mut bytes, EDemoCommands::DemSyncTick, 0, &[]).expect("sync");
        write_demo_message(&mut bytes, EDemoCommands::DemStop, 128, &[]).expect("stop");
        let info_offset = bytes.len() as u32;
        let info = CDemoFileInfo {
            playback_ticks: Some(128),
            playback_time: Some(2.0),
            ..Default::default()
        };
        write_demo_message(
            &mut bytes,
            EDemoCommands::DemFileInfo,
            128,
            &info.encode_to_vec(),
        )
        .expect("info");
        bytes[8..12].copy_from_slice(&info_offset.to_le_bytes());
        std::fs::write(path, bytes).expect("fixture");
    }

    #[test]
    fn reads_real_binary_protocol_headers_without_fabricating_stats() {
        for compressed in [false, true] {
            let dir = tempfile::tempdir().expect("dir");
            let path = dir.path().join("protocol.dem");
            minimal_demo(&path, compressed);
            let before = files::hash_file(&path).expect("hash");
            let parsed = parse(
                &path,
                Arc::new(AtomicBool::new(false)),
                Arc::new(|_| {}),
                None,
            )
            .expect("parse");
            assert_eq!(parsed.game.map, "de_protocol_test");
            assert_eq!(parsed.game.duration, Some(2.0));
            assert!(parsed.game.players.is_empty());
            assert!(parsed.game.rounds.is_empty());
            assert_eq!(parsed.game.teams[0].score, None);
            assert_eq!(files::hash_file(&path).expect("hash"), before);
        }
    }

    #[test]
    fn cancellation_reaches_the_parser_and_keeps_the_source() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("cancel.dem");
        minimal_demo(&path, false);
        let before = files::hash_file(&path).expect("hash");
        let result = parse(
            &path,
            Arc::new(AtomicBool::new(true)),
            Arc::new(|_| {}),
            None,
        );
        assert!(result.is_err());
        assert_eq!(files::hash_file(&path).expect("hash"), before);
    }

    #[test]
    fn cancellation_can_stop_preflight_before_the_first_tick() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("preflight.dem");
        minimal_demo(&path, false);
        let before = files::hash_file(&path).expect("hash");
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let result = parse(
            &path,
            cancel,
            Arc::new(move |progress| {
                assert!(progress < 0.0, "preflight has no tick percentage");
                signal.store(true, Ordering::Relaxed);
            }),
            None,
        );
        assert!(format!("{:#}", result.err().expect("cancelled")).contains("cancelled"));
        assert_eq!(files::hash_file(&path).expect("hash"), before);
    }

    #[test]
    fn signon_progress_is_indeterminate_and_cancel_does_not_require_a_tick() {
        let values = Arc::new(std::sync::Mutex::new(Vec::new()));
        let output = values.clone();
        let mut observer = MatchObserver {
            total_ticks: 100,
            progress: Arc::new(move |value| output.lock().expect("progress").push(value)),
            last_progress: Instant::now() - Duration::from_secs(1),
            ..Default::default()
        };
        observer.report_progress(0).expect("signon");
        observer.last_progress = Instant::now() - Duration::from_secs(1);
        observer.report_progress(50).expect("recorded tick");
        assert_eq!(*values.lock().expect("progress"), [-1.0, 0.5]);
        observer.cancel.store(true, Ordering::Relaxed);
        assert!(observer.report_progress(0).is_err());
    }

    #[test]
    fn server_recording_without_a_final_info_offset_is_read_only_and_explicit() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("server.dem");
        minimal_demo(&path, false);
        let mut bytes = std::fs::read(&path).expect("read");
        bytes[8..12].fill(0);
        std::fs::write(&path, &bytes).expect("write");
        let before = files::hash_file(&path).expect("hash");
        let parsed = parse(
            &path,
            Arc::new(AtomicBool::new(false)),
            Arc::new(|_| {}),
            None,
        )
        .expect("parse");
        assert_eq!(parsed.game.map, "de_protocol_test");
        assert!(
            parsed
                .game
                .warnings
                .iter()
                .any(|w| w.contains("No final metadata"))
        );
        assert_eq!(files::hash_file(&path).expect("hash"), before);
    }

    #[test]
    fn round_results_use_observed_score_changes_not_generated_sequences() {
        let mut observer = MatchObserver {
            live: true,
            current_round_start: 10,
            ..Default::default()
        };
        observer.record_scores([0, 0], 10);
        observer.record_scores([1, 0], 100);
        observer.record_scores([1, 0], 101);
        observer.record_scores([1, 1], 200);
        assert_eq!(observer.game.rounds.len(), 2);
        assert_eq!(observer.game.rounds[0].winner, Some(0));
        assert_eq!(observer.game.rounds[1].winner, Some(1));
        observer.record_scores([3, 4], 500); // missing segment: do not fabricate its winners
        assert_eq!(observer.game.rounds.len(), 2);
        assert_eq!(observer.game.completed_rounds, Some(7));
    }
}
