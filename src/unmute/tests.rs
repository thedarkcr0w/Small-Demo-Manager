use super::*;
use crate::parser;
use source2_demo::{
    prelude::{DemoRunner, EDemoCommands, Interests, Observer, ObserverResult},
    proto::{
        CDemoFileHeader, CDemoFileInfo, CDemoPacket, CDemoStringTables, CMsgVoiceAudio,
        CSvcMsgCreateStringTable,
        c_demo_string_tables::{ItemsT, TableT},
    },
    writer::{BitsWriter, BitstreamWriter, write_demo_message},
};
use std::sync::Mutex;

const ALICE: u64 = 76_561_198_000_000_001;
const BOB: u64 = 76_561_198_000_000_002;

fn quiet() -> Progress {
    Arc::new(|_| {})
}

fn idle() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

fn packet(messages: &[(i32, Vec<u8>)]) -> Vec<u8> {
    let mut data = Vec::new();
    let mut bits = BitstreamWriter::new(&mut data);
    for (kind, payload) in messages {
        bits.write_ubit_var(*kind as u32).expect("type");
        bits.write_var_u32(payload.len() as u32).expect("size");
        bits.write_bytes(payload).expect("payload");
    }
    bits.flush().expect("flush");
    drop(bits);
    CDemoPacket { data: Some(data) }.encode_to_vec()
}

fn voice(xuid: u64, audible_mask: Option<i32>, audio: &[u8]) -> (i32, Vec<u8>) {
    let message = CSvcMsgVoiceData {
        audio: Some(CMsgVoiceAudio {
            format: Some(2),
            voice_data: Some(audio.to_vec()),
            ..Default::default()
        }),
        xuid: (xuid != 0).then_some(xuid),
        audible_mask,
        ..Default::default()
    };
    (SvcMessages::SvcVoiceData as i32, message.encode_to_vec())
}

fn player_info(slot: usize, id: u64, name: &str) -> Vec<u8> {
    CMsgPlayerInfo {
        name: Some(name.into()),
        xuid: Some(id),
        steamid: Some(id),
        userid: Some(slot as i32),
        ..Default::default()
    }
    .encode_to_vec()
}

/// `svc_CreateStringTable` for userinfo in the packed wire format, as the
/// server sends it during sign-on.
fn create_userinfo(players: &[(u64, &str)]) -> (i32, Vec<u8>) {
    let mut data = Vec::new();
    let mut bits = BitstreamWriter::new(&mut data);
    for (slot, (id, name)) in players.iter().enumerate() {
        let value = player_info(slot, *id, name);
        bits.write_bit(true).expect("next index");
        bits.write_bit(true).expect("has key");
        bits.write_bit(false).expect("no key history");
        bits.write_cstring(&slot.to_string()).expect("key");
        bits.write_bit(true).expect("has value");
        bits.write_ubit_var(value.len() as u32).expect("size");
        bits.write_bits_as_bytes(&value, value.len() as u32 * 8)
            .expect("value");
    }
    bits.flush().expect("flush");
    drop(bits);
    let table = CSvcMsgCreateStringTable {
        name: Some(USERINFO.into()),
        num_entries: Some(players.len() as i32),
        user_data_fixed_size: Some(false),
        using_varint_bitcounts: Some(true),
        uncompressed_size: Some(data.len() as i32),
        string_data: Some(data),
        ..Default::default()
    };
    (
        SvcMessages::SvcCreateStringTable as i32,
        table.encode_to_vec(),
    )
}

/// The same table as a full-packet style `CDemoStringTables` snapshot.
fn userinfo(players: &[(u64, &str)]) -> Vec<u8> {
    CDemoStringTables {
        tables: vec![TableT {
            table_name: Some(USERINFO.into()),
            items: players
                .iter()
                .enumerate()
                .map(|(slot, (id, name))| ItemsT {
                    str: Some(slot.to_string()),
                    data: Some(player_info(slot, *id, name)),
                })
                .collect(),
            ..Default::default()
        }],
    }
    .encode_to_vec()
}

/// A structurally real demo: userinfo for two humans and a bot, a voice mask
/// that bans a slot, and voice packets with restrictive audible masks.
fn fixture(path: &Path, packets: &[Vec<u8>]) {
    let mut bytes = b"PBDEMS2\0\0\0\0\0\0\0\0\0".to_vec();
    let mut command = |kind, tick, payload: &[u8]| {
        write_demo_message(&mut bytes, kind, tick, payload).expect("command");
    };
    let header = CDemoFileHeader {
        map_name: Some("de_unmute_test".into()),
        ..Default::default()
    };
    command(EDemoCommands::DemFileHeader, 0, &header.encode_to_vec());
    let players = [(ALICE, "Alice"), (BOB, "Bob"), (0, "BOT Carl")];
    command(
        EDemoCommands::DemSignonPacket,
        u32::MAX,
        &packet(&[create_userinfo(&players)]),
    );
    command(EDemoCommands::DemSyncTick, 0, &[]);
    command(EDemoCommands::DemStringTables, 0, &userinfo(&players));
    for (index, packet) in packets.iter().enumerate() {
        command(EDemoCommands::DemPacket, 10 * (index as u32 + 1), packet);
    }
    command(EDemoCommands::DemStop, 128, &[]);
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

fn muted_packets() -> Vec<Vec<u8>> {
    let mask = CUserMessageVoiceMask {
        gamerules_masks: vec![0b0101, 0],
        ban_masks: vec![0b0010, 0],
        mod_enable: Some(false),
    };
    vec![
        packet(&[
            (EBaseUserMessages::UmVoiceMask as i32, mask.encode_to_vec()),
            voice(ALICE, Some(0), &[1, 2, 3]),
            voice(BOB, None, &[4, 5]),
        ]),
        packet(&[
            voice(ALICE, Some(-1), &[6, 7, 8, 9]),
            voice(0, Some(-1), &[10]),
        ]),
    ]
}

/// Everything a demo says about who is who and who may be heard, read with
/// the application's normal parser configuration.
#[derive(Default, Debug)]
struct Probe {
    ticks: u32,
    voice_packets: u64,
    voice_audio_bytes: u64,
    audible_masks: BTreeMap<Option<i32>, u64>,
    /// xuid -> (packets, audio bytes)
    voice_ids: BTreeMap<u64, (u64, u64)>,
    voice_masks: u64,
    restrictive_voice_masks: u64,
    userinfo: BTreeMap<u64, BTreeSet<String>>,
    controllers: BTreeMap<u64, BTreeSet<String>>,
    muted: BTreeSet<u64>,
    abuse_mute_updates: u64,
    mute_flag_updates: u64,
    mute_flag_types: BTreeSet<String>,
}

impl Probe {
    fn player(&mut self, value: &[u8]) -> ObserverResult {
        let player = CMsgPlayerInfo::decode(value)?;
        // Rows without an account (bots, SourceTV) may carry a non-player
        // `steamid`; they are not identities and are left alone.
        if let Some(id) = player.xuid.filter(|id| *id != 0) {
            assert_eq!(player.steamid, Some(id), "userinfo ids disagree");
            self.userinfo
                .entry(id)
                .or_default()
                .insert(player.name().to_owned());
        }
        Ok(())
    }
}

impl Observer for Probe {
    fn interests(&self) -> Interests {
        Interests::ENTITY_STATE
            | Interests::ENTITY_EVENTS
            | Interests::STRING_TABLE_STATE
            | Interests::STRING_TABLE_ENTRIES
            | Interests::SVC_MESSAGE
            | Interests::BASE_USER_MESSAGE
            | Interests::DEMO_MESSAGE
            | Interests::TICK_END
    }

    fn on_demo_command(
        &mut self,
        _ctx: &source2_demo::Context,
        kind: EDemoCommands,
        bytes: &[u8],
    ) -> ObserverResult {
        if kind == EDemoCommands::DemStringTables {
            for table in CDemoStringTables::decode(bytes)?.tables {
                if table.table_name() == USERINFO {
                    for item in table.items.iter().chain(&table.items_clientside) {
                        self.player(item.data())?;
                    }
                }
            }
        }
        Ok(())
    }

    fn on_string_table(
        &mut self,
        _ctx: &source2_demo::Context,
        table: &source2_demo::StringTable,
        modified: &[i32],
    ) -> ObserverResult {
        if table.name() == USERINFO {
            for index in modified {
                if let Some(value) = table.get_row(*index as usize)?.value() {
                    self.player(value)?;
                }
            }
        }
        Ok(())
    }

    fn on_svc_message(
        &mut self,
        _ctx: &source2_demo::Context,
        kind: SvcMessages,
        bytes: &[u8],
    ) -> ObserverResult {
        if kind == SvcMessages::SvcVoiceData {
            let packet = CSvcMsgVoiceData::decode(bytes)?;
            let audio = packet
                .audio
                .as_ref()
                .map_or(0, |audio| audio.voice_data().len() as u64);
            self.voice_packets += 1;
            self.voice_audio_bytes += audio;
            *self.audible_masks.entry(packet.audible_mask).or_default() += 1;
            let speaker = self.voice_ids.entry(packet.xuid.unwrap_or(0)).or_default();
            speaker.0 += 1;
            speaker.1 += audio;
        }
        Ok(())
    }

    fn on_base_user_message(
        &mut self,
        _ctx: &source2_demo::Context,
        kind: EBaseUserMessages,
        bytes: &[u8],
    ) -> ObserverResult {
        if kind == EBaseUserMessages::UmVoiceMask {
            let mut mask = CUserMessageVoiceMask::decode(bytes)?;
            self.voice_masks += 1;
            self.restrictive_voice_masks += u64::from(open_voice_mask(&mut mask));
        }
        Ok(())
    }

    fn on_entity(
        &mut self,
        _ctx: &source2_demo::Context,
        _event: EntityEvents,
        entity: &Entity,
    ) -> ObserverResult {
        if entity.class().name() != CONTROLLER {
            return Ok(());
        }
        let id = match entity.get_property(STEAM_ID) {
            Ok(FieldValue::Unsigned64(id)) => *id,
            _ => 0,
        };
        if id != 0
            && let Ok(FieldValue::String(name)) = entity.get_property(PLAYER_NAME)
        {
            self.controllers.entry(id).or_default().insert(name.clone());
        }
        let abuse = matches!(
            entity.get_property(ABUSE_MUTE),
            Ok(FieldValue::Boolean(true))
        );
        let flags = entity.get_property(MUTE_FLAGS).ok();
        if let Some(flags) = flags {
            let kind = format!("{flags:?}");
            self.mute_flag_types
                .insert(kind.split('(').next().unwrap_or_default().to_owned());
        }
        let flagged = flags.is_some_and(|flags| cleared_flags(flags).is_some());
        self.abuse_mute_updates += u64::from(abuse);
        self.mute_flag_updates += u64::from(flagged);
        if abuse || flagged {
            self.muted.insert(id);
        }
        Ok(())
    }

    fn on_tick_end(&mut self, ctx: &source2_demo::Context) -> ObserverResult {
        if ctx.tick() != u32::MAX {
            self.ticks = self.ticks.max(ctx.tick());
        }
        Ok(())
    }
}

fn probe(path: &Path) -> Probe {
    let input = BufReader::with_capacity(1024 * 1024, File::open(path).expect("open"));
    let mut parser = Parser::from_reader(input).expect("header");
    let probe = parser.register_observer::<Probe>();
    parser.run_to_end().expect("decode");
    drop(parser);
    std::rc::Rc::try_unwrap(probe)
        .expect("sole owner")
        .into_inner()
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .expect("dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn identities_map_each_steam_id_to_one_valid_synthetic_id() {
    let mut ids = Identities::default();
    let alice = ids.remap(ALICE);
    let bob = ids.remap(BOB);
    assert_ne!(alice, bob);
    assert_eq!(ids.remap(ALICE), alice);
    assert_eq!(ids.remap(BOB), bob);
    assert_eq!(ids.remap(0), 0, "bots keep having no account");
    assert_eq!(ids.synthetic.len(), 2);
    for (original, mapped) in [(ALICE, alice), (BOB, bob)] {
        assert_ne!(mapped, original);
        // Public universe, individual account, instance 1, 32-bit account id.
        assert_eq!(mapped >> 32, STEAM_ID64_INDIVIDUAL_BASE >> 32);
        assert!(mapped - STEAM_ID64_INDIVIDUAL_BASE >= SYNTHETIC_ACCOUNT_ID_BASE);
        assert_eq!(ids.original_of(mapped), original);
        assert_eq!(ids.original_of(original), original);
    }
}

#[test]
fn voice_masks_are_opened_and_already_open_ones_are_left_alone() {
    let mut mask = CUserMessageVoiceMask {
        gamerules_masks: vec![1, u32::MAX],
        ban_masks: vec![0, 8],
        mod_enable: None,
    };
    assert!(open_voice_mask(&mut mask));
    assert_eq!(mask.gamerules_masks, [u32::MAX, u32::MAX]);
    assert_eq!(mask.ban_masks, [0, 0]);
    assert_eq!(mask.mod_enable, Some(true));
    assert!(!open_voice_mask(&mut mask));
}

#[test]
fn mute_flags_clear_to_zero_of_the_decoded_integer_type() {
    assert_eq!(
        cleared_flags(&FieldValue::Unsigned32(6)),
        Some(FieldValue::Unsigned32(0))
    );
    assert_eq!(
        cleared_flags(&FieldValue::Unsigned8(1)),
        Some(FieldValue::Unsigned8(0))
    );
    assert_eq!(
        cleared_flags(&FieldValue::Signed32(-1)),
        Some(FieldValue::Signed32(0))
    );
    assert_eq!(cleared_flags(&FieldValue::Unsigned32(0)), None);
    assert_eq!(cleared_flags(&FieldValue::Boolean(true)), None);
}

#[test]
fn unmutes_voice_and_remaps_identities_consistently() {
    let dir = tempfile::tempdir().expect("dir");
    let (input, output) = (dir.path().join("in.dem"), dir.path().join("out.dem"));
    fixture(&input, &muted_packets());
    let before = files::hash_file(&input).expect("hash");

    let report = unmute(&input, &output, idle(), quiet()).expect("unmute");

    assert_eq!(files::hash_file(&input).expect("hash"), before);
    assert_eq!(files_in(dir.path()), ["in.dem", "out.dem"]);
    assert_eq!(
        report.output_bytes,
        output.metadata().expect("output").len()
    );
    assert_eq!(report.ticks, 20);
    assert_eq!((report.voice_masks, report.voice_masks_changed), (1, 1));
    assert_eq!((report.voice_packets, report.voice_audio_bytes), (4, 10));
    assert_eq!(report.audible_masks_changed, 2);
    assert_eq!(report.identities_remapped, 2);
    // Once in the sign-on table and once in the string-table snapshot.
    assert_eq!(report.userinfo_rows_remapped, 4);
    assert_eq!(report.voice_packet_ids_remapped, 3);
    assert_eq!(report.mute_fields_cleared(), 0);
    let speakers: Vec<_> = report
        .speakers
        .iter()
        .map(|s| {
            (
                s.name.as_str(),
                s.steam_id.clone(),
                s.packets,
                s.audio_bytes,
            )
        })
        .collect();
    assert_eq!(
        speakers,
        [
            ("Alice", Some(ALICE.to_string()), 2, 7),
            ("Bob", Some(BOB.to_string()), 1, 2),
            ("", None, 1, 1),
        ]
    );

    let (source, result) = (probe(&input), probe(&output));
    assert_eq!(source.restrictive_voice_masks, 1);
    assert_eq!((result.voice_masks, result.restrictive_voice_masks), (1, 0));
    assert_eq!(result.voice_packets, source.voice_packets);
    assert_eq!(result.voice_audio_bytes, source.voice_audio_bytes);
    assert_eq!(result.audible_masks, BTreeMap::from([(Some(-1), 4)]));
    let synthetic = |id: u64| -> u64 {
        report
            .identities
            .iter()
            .find(|identity| identity.steam_id == id.to_string())
            .expect("identity")
            .synthetic_steam_id
            .parse()
            .expect("id")
    };
    // The same synthetic id replaces a player in userinfo and in voice
    // packets, and each keeps their name and their audio.
    for (id, name) in [(ALICE, "Alice"), (BOB, "Bob")] {
        let mapped = synthetic(id);
        assert_eq!(result.userinfo[&mapped], BTreeSet::from([name.to_owned()]));
        assert_eq!(result.voice_ids[&mapped], source.voice_ids[&id]);
        assert!(!result.userinfo.contains_key(&id));
        assert!(!result.voice_ids.contains_key(&id));
    }
    assert_eq!(result.voice_ids[&0], source.voice_ids[&0]);
    assert_eq!(result.userinfo.len(), 2);

    let parsed = parser::parse(&output, idle(), quiet(), None).expect("parse output");
    assert_eq!(parsed.game.map, "de_unmute_test");
}

#[test]
fn a_demo_with_nothing_to_change_is_reproduced_byte_for_byte() {
    let dir = tempfile::tempdir().expect("dir");
    let (input, output) = (dir.path().join("in.dem"), dir.path().join("out.dem"));
    fixture(&input, &[packet(&[voice(0, Some(-1), &[1, 2])])]);
    let report = unmute(&input, &output, idle(), quiet()).expect("unmute");
    assert_eq!(report.identities_remapped, 2, "userinfo is still remapped");
    assert_ne!(
        std::fs::read(&input).expect("input"),
        std::fs::read(&output).expect("output")
    );

    // The same rules without applying them must not disturb a single byte.
    let dry = dir.path().join("dry.dem");
    let dry_report = rewrite(
        &input,
        &dry,
        idle(),
        quiet(),
        false,
        Some(WORKER_STACK_BYTES),
    )
    .expect("dry run");
    assert_eq!(dry_report.identities_remapped, 2);
    assert_eq!(
        std::fs::read(&input).expect("input"),
        std::fs::read(&dry).expect("output")
    );
}

#[test]
fn an_existing_output_or_the_input_itself_is_never_overwritten() {
    let dir = tempfile::tempdir().expect("dir");
    let (input, output) = (dir.path().join("in.dem"), dir.path().join("out.dem"));
    fixture(&input, &muted_packets());
    std::fs::write(&output, b"keep me").expect("existing");
    let before = files::hash_file(&input).expect("hash");

    let error = unmute(&input, &output, idle(), quiet()).expect_err("existing output");
    assert!(format!("{error:#}").contains("already exists"));
    assert_eq!(std::fs::read(&output).expect("output"), b"keep me");

    let error = unmute(&input, &input, idle(), quiet()).expect_err("in place");
    assert!(format!("{error:#}").contains("already exists"));
    assert_eq!(files::hash_file(&input).expect("hash"), before);
    assert_eq!(files_in(dir.path()), ["in.dem", "out.dem"]);
}

#[test]
fn an_unfinished_earlier_run_is_reported_not_replaced() {
    let dir = tempfile::tempdir().expect("dir");
    let (input, output) = (dir.path().join("in.dem"), dir.path().join("out.dem"));
    fixture(&input, &muted_packets());
    let partial = dir.path().join("out.dem.partial");
    std::fs::write(&partial, b"someone else's work").expect("partial");

    let error = unmute(&input, &output, idle(), quiet()).expect_err("stale partial");
    assert!(format!("{error:#}").contains("out.dem.partial"));
    assert_eq!(
        std::fs::read(&partial).expect("partial"),
        b"someone else's work"
    );
    assert!(!output.exists());
}

#[test]
fn a_failed_rewrite_leaves_no_partial_or_output() {
    let dir = tempfile::tempdir().expect("dir");
    let (input, output) = (dir.path().join("in.dem"), dir.path().join("out.dem"));
    // Valid command envelopes around a packet that is not a protobuf.
    fixture(&input, &[vec![0xff; 32]]);
    let before = files::hash_file(&input).expect("hash");

    let error = unmute(&input, &output, idle(), quiet()).expect_err("corrupt packet");
    assert!(format!("{error:#}").contains("Rewrite CS2 demo stream"));
    assert_eq!(files_in(dir.path()), ["in.dem"]);
    assert_eq!(files::hash_file(&input).expect("hash"), before);

    let missing = dir.path().join("missing").join("out.dem");
    fixture(&input, &muted_packets());
    assert!(unmute(&input, &missing, idle(), quiet()).is_err());
    assert_eq!(files_in(dir.path()), ["in.dem"]);
}

#[test]
fn cancellation_stops_the_rewrite_and_removes_the_partial_file() {
    let dir = tempfile::tempdir().expect("dir");
    let (input, output) = (dir.path().join("in.dem"), dir.path().join("out.dem"));
    fixture(&input, &muted_packets());

    // Cancelled before it starts: nothing is created.
    let error =
        unmute(&input, &output, Arc::new(AtomicBool::new(true)), quiet()).expect_err("cancelled");
    assert!(format!("{error:#}").contains("cancelled"));
    assert_eq!(files_in(dir.path()), ["in.dem"]);

    // Cancelled from the first progress report made while packets are being
    // rewritten, i.e. after the partial file exists.
    let cancel = idle();
    let reports = Arc::new(Mutex::new(Vec::new()));
    let (signal, seen, partial) = (
        cancel.clone(),
        reports.clone(),
        dir.path().join("out.dem.partial"),
    );
    let error = unmute(
        &input,
        &output,
        cancel,
        Arc::new(move |value| {
            let mut seen = seen.lock().expect("reports");
            seen.push((value, partial.exists()));
            if seen.len() == 2 {
                signal.store(true, Ordering::Relaxed);
            }
        }),
    )
    .expect_err("cancelled");
    assert!(format!("{error:#}").contains("cancelled"));
    let reports = reports.lock().expect("reports");
    assert_eq!(reports[0], (-1.0, false));
    assert!(reports[1].1, "second report comes from the packet rewriter");
    assert_eq!(reports.len(), 2);
    assert_eq!(files_in(dir.path()), ["in.dem"]);
}

fn real_demo() -> PathBuf {
    PathBuf::from(std::env::var_os("SDM_UNMUTE_DEMO").expect("SDM_UNMUTE_DEMO is required"))
}

/// Outputs go to SDM_UNMUTE_OUT when set (and are kept), else to a temporary
/// directory. Never next to the source demo.
fn real_output(name: &str) -> (Option<tempfile::TempDir>, PathBuf) {
    match std::env::var_os("SDM_UNMUTE_OUT") {
        Some(dir) => {
            let path = PathBuf::from(dir).join(name);
            let _ = std::fs::remove_file(&path);
            (None, path)
        }
        None => {
            let dir = tempfile::tempdir().expect("dir");
            let path = dir.path().join(name);
            (Some(dir), path)
        }
    }
}

fn match_json(path: &Path) -> String {
    let parsed = parser::parse(path, idle(), quiet(), None).expect("parse");
    serde_json::to_string_pretty(&parsed.game).expect("json")
}

/// Go/no-go: the vendored writer must reproduce a real demo exactly, both
/// with no rewriter and with every rule decoding but changing nothing.
#[test]
#[ignore = "Set SDM_UNMUTE_DEMO to a real CS2 demo (SDM_UNMUTE_OUT keeps the outputs)"]
fn real_demo_survives_the_writer_unchanged() {
    let input = real_demo();
    let expected = files::hash_file(&input).expect("hash");

    let (_keep, plain) = real_output("identity-no-rewriter.dem");
    let started = Instant::now();
    on_worker_stack(WORKER_STACK_BYTES, || {
        let reader = BufReader::with_capacity(1024 * 1024, File::open(&input)?);
        let mut writer = DemoWriter::new(
            Parser::from_reader(reader)?,
            BufWriter::new(File::create_new(&plain)?),
        );
        writer.run()?;
        writer.into_parts().1.flush()?;
        Ok(())
    })
    .expect("plain writer");
    eprintln!("no rewriter: {:.2?}", started.elapsed());
    assert_eq!(files::hash_file(&plain).expect("hash"), expected);

    let (_keep, dry) = real_output("identity-rules-not-applied.dem");
    let started = Instant::now();
    let report = rewrite(
        &input,
        &dry,
        idle(),
        quiet(),
        false,
        Some(WORKER_STACK_BYTES),
    )
    .expect("dry run");
    eprintln!("rules not applied: {:.2?}", started.elapsed());
    eprintln!("{}", serde_json::to_string_pretty(&report).expect("json"));
    assert_eq!(files::hash_file(&dry).expect("hash"), expected);
    assert_eq!(match_json(&dry), match_json(&input));
    assert_eq!(files::hash_file(&input).expect("hash"), expected);
}

#[test]
#[ignore = "Set SDM_UNMUTE_DEMO to a real CS2 demo (SDM_UNMUTE_OUT keeps the output)"]
fn real_demo_is_unmuted_and_still_parses_identically() {
    let input = real_demo();
    let expected = files::hash_file(&input).expect("hash");
    let (_keep, output) = real_output("unmuted.dem");

    let started = Instant::now();
    let report = unmute(&input, &output, idle(), quiet()).expect("unmute");
    eprintln!("unmute: {:.2?}", started.elapsed());
    eprintln!("{}", serde_json::to_string_pretty(&report).expect("json"));
    assert_eq!(files::hash_file(&input).expect("hash"), expected);
    assert!(!partial_path(&output).expect("partial").exists());

    let (source, result) = (probe(&input), probe(&output));
    eprintln!(
        "source: ticks={} voice={}/{} masks={:?} voice_masks={} restrictive={} muted={:?} abuse_updates={} flag_updates={} flag_types={:?}",
        source.ticks,
        source.voice_packets,
        source.voice_audio_bytes,
        source.audible_masks,
        source.voice_masks,
        source.restrictive_voice_masks,
        source.muted,
        source.abuse_mute_updates,
        source.mute_flag_updates,
        source.mute_flag_types,
    );
    eprintln!(
        "output: ticks={} voice={}/{} masks={:?} voice_masks={} restrictive={} muted={:?} flag_types={:?}",
        result.ticks,
        result.voice_packets,
        result.voice_audio_bytes,
        result.audible_masks,
        result.voice_masks,
        result.restrictive_voice_masks,
        result.muted,
        result.mute_flag_types,
    );

    // Same stream length and the same audio, all of it audible to everyone.
    assert_eq!(result.ticks, source.ticks);
    assert_eq!(report.ticks, source.ticks);
    assert_eq!(result.voice_packets, source.voice_packets);
    assert_eq!(result.voice_audio_bytes, source.voice_audio_bytes);
    assert_eq!(report.voice_packets, source.voice_packets);
    assert_eq!(report.voice_audio_bytes, source.voice_audio_bytes);
    assert!(source.voice_packets > 0, "demo has no voice to verify");
    assert_eq!(
        result.audible_masks,
        BTreeMap::from([(Some(-1), source.voice_packets)])
    );
    assert_eq!(result.voice_masks, source.voice_masks);
    assert_eq!(result.restrictive_voice_masks, 0);

    // Mutes are found exactly where the source has them, and are gone.
    assert_eq!(
        report.abuse_mutes_cleared + report.mute_flags_cleared > 0,
        !source.muted.is_empty()
    );
    let muted: BTreeSet<u64> = report
        .muted_players
        .iter()
        .map(|player| player.steam_id.parse().expect("id"))
        .collect();
    assert_eq!(muted, source.muted);
    assert!(result.muted.is_empty());
    assert_eq!(
        (result.abuse_mute_updates, result.mute_flag_updates),
        (0, 0)
    );

    // Every identity moves to one synthetic id everywhere, with its names.
    let mapping: BTreeMap<u64, u64> = report
        .identities
        .iter()
        .map(|identity| {
            (
                identity.steam_id.parse().expect("id"),
                identity.synthetic_steam_id.parse().expect("id"),
            )
        })
        .collect();
    assert_eq!(report.identities_remapped as usize, mapping.len());
    assert_eq!(
        mapping.values().collect::<BTreeSet<_>>().len(),
        mapping.len(),
        "synthetic ids are unique"
    );
    let moved = |ids: &BTreeMap<u64, BTreeSet<String>>| -> BTreeMap<u64, BTreeSet<String>> {
        ids.iter()
            .map(|(id, names)| (mapping[id], names.clone()))
            .collect()
    };
    assert!(!source.controllers.is_empty(), "demo has no players");
    assert_eq!(result.controllers, moved(&source.controllers));
    assert_eq!(result.userinfo, moved(&source.userinfo));
    let voices: BTreeMap<u64, (u64, u64)> = source
        .voice_ids
        .iter()
        .map(|(id, audio)| (if *id == 0 { 0 } else { mapping[id] }, *audio))
        .collect();
    assert_eq!(result.voice_ids, voices);
    for id in mapping.keys() {
        assert!(!result.controllers.contains_key(id));
        assert!(!result.userinfo.contains_key(id));
        assert!(!result.voice_ids.contains_key(id));
    }

    // The application reads the same match from both files, identities and
    // the cleared mutes aside.
    let source_game = parser::parse(&input, idle(), quiet(), None)
        .expect("parse input")
        .game;
    let mut result_game = parser::parse(&output, idle(), quiet(), None)
        .expect("parse output")
        .game;
    let source_muted: BTreeSet<u64> = source_game
        .players
        .iter()
        .filter(|player| player.comm_muted)
        .filter_map(|player| player.steam_id.as_ref()?.parse().ok())
        .collect();
    eprintln!(
        "parser: ticks={} players={} comm_muted={:?}",
        source_game.ticks,
        source_game.players.len(),
        source_game
            .players
            .iter()
            .filter(|player| player.comm_muted)
            .map(|player| (&player.name, player.comm_mute_flags))
            .collect::<Vec<_>>()
    );
    assert_eq!(source_muted, source.muted);
    assert!(result_game.players.iter().all(|player| !player.comm_muted));
    let originals: BTreeMap<String, String> = mapping
        .iter()
        .map(|(original, mapped)| (mapped.to_string(), original.to_string()))
        .collect();
    for (player, original) in result_game.players.iter_mut().zip(&source_game.players) {
        player.steam_id = player
            .steam_id
            .as_ref()
            .map(|id| originals.get(id).expect("synthetic id").clone());
        player.comm_muted = original.comm_muted;
        player.comm_mute_flags = original.comm_mute_flags;
    }
    assert_eq!(
        serde_json::to_string_pretty(&result_game).expect("json"),
        serde_json::to_string_pretty(&source_game).expect("json")
    );

    // The header offsets the engine seeks to still land on their commands.
    let header = std::fs::read(&output).expect("output");
    for (position, command) in [(8, 2_u8), (12, 15)] {
        let offset = u32::from_le_bytes(header[position..position + 4].try_into().expect("u32"));
        if offset != 0 {
            assert_eq!(
                header[offset as usize] & !64,
                command,
                "header offset {position}"
            );
        }
    }
}

/// Test-only inverse of the unmute rules: marks controllers as muted wherever
/// the demo transmits the mute fields (instance baseline and entity updates).
/// No real muted demo is available, so this builds one from a real match.
#[derive(Default)]
struct InjectMutes {
    baseline_fields: u64,
    update_fields: u64,
}

impl DemoRewriter for InjectMutes {
    fn interests(&self) -> RewriteInterests {
        RewriteInterests::ENTITY_FIELDS
    }

    fn replace_entity_field(
        &mut self,
        _ctx: &source2_demo::Context,
        _event: EntityEvents,
        entity: &Entity,
        field_name: &str,
        value: &FieldValue,
    ) -> Option<FieldValue> {
        let muted = match (field_name, value) {
            (ABUSE_MUTE, FieldValue::Boolean(false)) => FieldValue::Boolean(true),
            (MUTE_FLAGS, FieldValue::Unsigned32(0)) => FieldValue::Unsigned32(2),
            _ => return None,
        };
        // Baselines are decoded through a placeholder entity at index 0;
        // controllers themselves start at index 1.
        if entity.index() == 0 {
            self.baseline_fields += 1;
        } else {
            self.update_fields += 1;
        }
        Some(muted)
    }

    fn should_rewrite_entity(
        &mut self,
        _ctx: &source2_demo::Context,
        _event: EntityEvents,
        entity: &Entity,
    ) -> bool {
        entity.class().name() == CONTROLLER
    }

    fn should_track_entity(
        &mut self,
        _ctx: &source2_demo::Context,
        _event: EntityEvents,
        _entity: &Entity,
    ) -> bool {
        false
    }
}

/// The mute paths on real entity data: inject mutes into a real match, check
/// that the application's parser reports them, then check that unmuting the
/// muted copy yields exactly the unmuted original.
#[test]
#[ignore = "Set SDM_UNMUTE_DEMO to a real CS2 demo (SDM_UNMUTE_OUT keeps the outputs)"]
fn real_demo_with_injected_mutes_is_detected_and_cleared() {
    let input = real_demo();
    let (_keep, muted) = real_output("injected-mutes.dem");
    let (baseline_fields, update_fields) = on_worker_stack(WORKER_STACK_BYTES, || {
        let reader = BufReader::with_capacity(1024 * 1024, File::open(&input)?);
        let mut writer = DemoWriter::new(
            Parser::from_reader(reader)?,
            BufWriter::new(File::create_new(&muted)?),
        );
        let injected = writer.register_rewriter::<InjectMutes>();
        writer.run()?;
        writer.into_parts().1.flush()?;
        let injected = injected.borrow();
        Ok((injected.baseline_fields, injected.update_fields))
    })
    .expect("inject");
    eprintln!("injected: baseline fields={baseline_fields} update fields={update_fields}");
    assert!(
        baseline_fields + update_fields > 0,
        "demo never transmits the mute fields"
    );

    let source = probe(&muted);
    eprintln!(
        "muted copy: muted={:?} abuse_updates={} flag_updates={} flag_types={:?}",
        source.muted, source.abuse_mute_updates, source.mute_flag_updates, source.mute_flag_types
    );
    assert!(!source.muted.is_empty());
    let game = parser::parse(&muted, idle(), quiet(), None)
        .expect("parse muted")
        .game;
    let flagged: BTreeSet<u64> = game
        .players
        .iter()
        .filter(|player| player.comm_muted)
        .filter_map(|player| player.steam_id.as_ref()?.parse().ok())
        .collect();
    eprintln!(
        "parser on muted copy: {:?}",
        game.players
            .iter()
            .map(|player| (&player.name, player.comm_muted, player.comm_mute_flags))
            .collect::<Vec<_>>()
    );
    let humans: BTreeSet<u64> = source.muted.iter().copied().filter(|id| *id != 0).collect();
    assert_eq!(flagged, humans);

    let (_keep, cleared) = real_output("injected-mutes-unmuted.dem");
    let report = unmute(&muted, &cleared, idle(), quiet()).expect("unmute muted copy");
    eprintln!(
        "cleared: abuse={} flags={} muted_players={:?}",
        report.abuse_mutes_cleared,
        report.mute_flags_cleared,
        report
            .muted_players
            .iter()
            .map(|player| &player.name)
            .collect::<Vec<_>>()
    );
    assert!(report.abuse_mutes_cleared > 0);
    assert!(report.mute_flags_cleared > 0);
    let result = probe(&cleared);
    assert!(result.muted.is_empty());
    assert_eq!(
        (result.abuse_mute_updates, result.mute_flag_updates),
        (0, 0)
    );
    let game = parser::parse(&cleared, idle(), quiet(), None)
        .expect("parse cleared")
        .game;
    assert!(game.players.iter().all(|player| !player.comm_muted));

    // Clearing the injected mutes restores the original values, so this must
    // be the very file that unmuting the original produces.
    let (_keep, reference) = real_output("injected-mutes-reference.dem");
    unmute(&input, &reference, idle(), quiet()).expect("unmute original");
    assert_eq!(
        files::hash_file(&cleared).expect("hash"),
        files::hash_file(&reference).expect("hash")
    );
}
/// Measures how much stack the rewrite needs: SDM_UNMUTE_STACK_KB selects the
/// worker stack (0 = run on the calling thread). Too little aborts the test
/// process with a stack overflow, which is the measurement.
#[test]
#[ignore = "Set SDM_UNMUTE_DEMO and SDM_UNMUTE_STACK_KB to probe the stack requirement"]
fn real_demo_stack_requirement() {
    let input = real_demo();
    let kb: usize = std::env::var("SDM_UNMUTE_STACK_KB")
        .expect("SDM_UNMUTE_STACK_KB is required")
        .parse()
        .expect("kilobytes");
    let (_keep, output) = real_output(&format!("stack-{kb}k.dem"));
    let started = Instant::now();
    let report = rewrite(
        &input,
        &output,
        idle(),
        quiet(),
        true,
        (kb > 0).then_some(kb * 1024),
    )
    .expect("unmute");
    eprintln!(
        "stack {kb} KiB: {:.2?}, {} voice packets",
        started.elapsed(),
        report.voice_packets
    );
    let _ = std::fs::remove_file(&output);
}
