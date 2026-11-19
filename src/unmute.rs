// The demo rewrite rules in this module (which controller fields, voice masks,
// voice packets and identities are rewritten, and how) are ported from
// OdyCSUnmuter, https://github.com/OdyCS/OdyCSUnmuter, commit f62bb17.
//
// MIT License
//
// Copyright (c) 2026 Ody
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Writes a copy of a demo in which globally communication-muted players are
//! audible during CS2 playback. The source demo is only ever read.
//!
//! CS2 silences a recorded player when their controller carries a
//! communication-abuse mute, when a voice mask bans their slot, or when a voice
//! packet's audible mask excludes the viewer. It also applies the viewer's own
//! account-level mutes by SteamID, so every real SteamID is replaced by one
//! synthetic ID, consistently in controllers, `userinfo` and voice packets.

use crate::{files, parser::Progress};
use anyhow::{Context as _, Result, anyhow, bail, ensure};
use serde::Serialize;
use source2_demo::{
    Entity, EntityEvents, FieldValue,
    error::ParserError,
    prelude::{EBaseUserMessages, Message, Parser, SvcMessages},
    proto::{CMsgPlayerInfo, CSvcMsgVoiceData, CUserMessageVoiceMask},
    writer::{DemoRewriter, DemoWriter, MessageRewrite, RewriteInterests, StringTableEntryUpdate},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const CONTROLLER: &str = "CCSPlayerController";
const STEAM_ID: &str = "m_steamID";
const PLAYER_NAME: &str = "m_iszPlayerName";
const ABUSE_MUTE: &str = "m_bHasCommunicationAbuseMute";
const MUTE_FLAGS: &str = "m_uiCommunicationMuteFlags";
const USERINFO: &str = "userinfo";

/// SteamID64 of individual account 0 in the public universe.
const STEAM_ID64_INDIVIDUAL_BASE: u64 = 76_561_197_960_265_728;
/// Synthetic IDs use account numbers far above any issued account, but still
/// inside the 32-bit account field so they stay well-formed SteamID64 values.
const SYNTHETIC_ACCOUNT_ID_BASE: u64 = 4_000_000_000;

/// The writer owns a parser whose field-path buffers live inline and is moved
/// by value. On a 178 MB match the rewrite overflowed a 896 KiB stack in
/// release builds and a 2 MiB stack (the default for spawned threads) in debug
/// builds; it ran in 1 MiB and 2.5 MiB respectively. The address space is only
/// reserved, so the margin costs nothing.
const WORKER_STACK_BYTES: usize = 16 * 1024 * 1024;

/// What an unmute run found and changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UnmuteReport {
    /// Last recorded tick of the rewritten stream.
    pub ticks: u32,
    pub output_bytes: u64,
    /// `m_bHasCommunicationAbuseMute` updates that were true and were cleared.
    pub abuse_mutes_cleared: u64,
    /// `m_uiCommunicationMuteFlags` updates that were non-zero and were cleared.
    pub mute_flags_cleared: u64,
    /// `CUserMessageVoiceMask` messages seen / changed.
    pub voice_masks: u64,
    pub voice_masks_changed: u64,
    pub voice_packets: u64,
    pub voice_audio_bytes: u64,
    /// Voice packets whose audible mask was not already "everyone".
    pub audible_masks_changed: u64,
    /// Distinct real SteamIDs replaced by a synthetic one.
    pub identities_remapped: u64,
    pub controller_ids_remapped: u64,
    pub userinfo_rows_remapped: u64,
    pub voice_packet_ids_remapped: u64,
    /// Players that carried a communication mute in the source demo.
    pub muted_players: Vec<UnmuteIdentity>,
    pub identities: Vec<UnmuteIdentity>,
    /// Most audio first.
    pub speakers: Vec<UnmuteSpeaker>,
}

impl UnmuteReport {
    pub fn mute_fields_cleared(&self) -> u64 {
        self.abuse_mutes_cleared + self.mute_flags_cleared
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UnmuteIdentity {
    /// Empty when the demo never named this SteamID.
    pub name: String,
    /// Real SteamID64 from the source demo.
    pub steam_id: String,
    /// SteamID64 written to the output instead.
    pub synthetic_steam_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UnmuteSpeaker {
    pub name: String,
    /// Real SteamID64 from the source demo; `None` for packets without one.
    pub steam_id: Option<String>,
    pub packets: u64,
    pub audio_bytes: u64,
}

/// Writes an unmuted copy of `input` to `output`.
///
/// The copy is written to `<output>.partial` and renamed only once it is
/// complete and passes demo validation. An existing `output` (which includes
/// `output == input`) or `.partial` file is an error, never replaced. The
/// partial file is removed on any failure or cancellation.
pub fn unmute(
    input: &Path,
    output: &Path,
    cancel: Arc<AtomicBool>,
    progress: Progress,
) -> Result<UnmuteReport> {
    rewrite(
        input,
        output,
        cancel,
        progress,
        true,
        Some(WORKER_STACK_BYTES),
    )
}

fn partial_path(output: &Path) -> Result<PathBuf> {
    let mut name = output
        .file_name()
        .context("Output path has no filename")?
        .to_owned();
    name.push(".partial");
    // Absolute, so cleanup still finds the file if the working directory moves.
    Ok(std::path::absolute(output.with_file_name(name))?)
}

/// `apply == false` runs every decoder and rule but changes nothing, which
/// must reproduce the input byte for byte. `stack == None` stays on the
/// calling thread. Both exist to verify the writer against real demos.
fn rewrite(
    input: &Path,
    output: &Path,
    cancel: Arc<AtomicBool>,
    progress: Progress,
    apply: bool,
    stack: Option<usize>,
) -> Result<UnmuteReport> {
    // No tick percentage exists until the first recorded tick.
    progress(-1.0);
    ensure!(
        !output.exists(),
        "Output already exists: {}",
        output.display()
    );
    let partial = partial_path(output)?;
    ensure!(
        partial.parent().is_some_and(Path::is_dir),
        "Output directory does not exist: {}",
        output.display()
    );
    files::validate_demo_cancellable(input, &cancel)?;
    let before = files::stamp(input)?;

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .with_context(|| {
            format!(
                "Create {} (an unfinished earlier run must be removed first)",
                partial.display()
            )
        })?;
    // From here on the partial file is deleted unless it is committed.
    let partial = tempfile::TempPath::try_from_path(&partial).inspect_err(|_| {
        let _ = std::fs::remove_file(&partial);
    })?;

    let work = {
        let (cancel, progress) = (cancel.clone(), progress.clone());
        move || write_demo(input, file, cancel, progress, apply)
    };
    let mut report = match stack {
        Some(bytes) => on_worker_stack(bytes, work),
        None => work(),
    }?;

    ensure!(!cancel.load(Ordering::Relaxed), "Operation cancelled");
    ensure!(
        files::stamp(input)? == before,
        "Demo changed while it was being rewritten; no output was kept"
    );
    progress(-1.0);
    files::validate_demo_cancellable(&partial, &cancel)
        .context("Rewritten demo failed validation; no output was kept")?;
    report.output_bytes = partial.metadata()?.len();
    // A no-replace rename: a file that appeared at `output` meanwhile wins.
    partial
        .persist_noclobber(output)
        .map_err(|e| e.error)
        .context("Commit rewritten demo (output must not exist)")?;
    progress(1.0);
    Ok(report)
}

/// A decoder panic must not unwind into the caller, and the rewrite must not
/// depend on how much stack the caller happens to have left.
fn on_worker_stack<T: Send>(bytes: usize, work: impl FnOnce() -> Result<T> + Send) -> Result<T> {
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("demo-unmute".into())
            .stack_size(bytes)
            .spawn_scoped(scope, work)
            .context("Start demo rewrite thread")?;
        worker.join().unwrap_or_else(|_| {
            bail!(
                "The demo decoder rejected a malformed or unsupported stream. Original demo was not changed and no output was kept; include this demo's build number when reporting compatibility issues"
            )
        })
    })
}

fn write_demo(
    input: &Path,
    file: File,
    cancel: Arc<AtomicBool>,
    progress: Progress,
    apply: bool,
) -> Result<UnmuteReport> {
    let reader = BufReader::with_capacity(1024 * 1024, File::open(input)?);
    let parser = Parser::from_reader(reader).context("Read CS2 demo header")?;
    let total_ticks = parser.replay_info().playback_ticks().max(0) as u32;
    let mut writer = DemoWriter::new(parser, BufWriter::with_capacity(1024 * 1024, file));
    let rules = writer.add_rewriter(Rules::new(apply, cancel, progress, total_ticks));
    writer
        .run()
        .map_err(anyhow::Error::from)
        .context("Rewrite CS2 demo stream")?;
    let (_, mut output) = writer.into_parts();
    output.flush().context("Write rewritten demo")?;
    output
        .get_ref()
        .sync_all()
        .context("Write rewritten demo")?;
    drop(output);
    Ok(rules.borrow().report())
}

/// One synthetic SteamID64 per real one, stable for the whole demo.
#[derive(Default)]
struct Identities {
    synthetic: HashMap<u64, u64>,
    original: HashMap<u64, u64>,
}

impl Identities {
    /// Zero means "no account" (bots, HLTV) and stays zero.
    fn remap(&mut self, original: u64) -> u64 {
        if original == 0 {
            return 0;
        }
        if let Some(mapped) = self.synthetic.get(&original) {
            return *mapped;
        }
        let mapped =
            STEAM_ID64_INDIVIDUAL_BASE + SYNTHETIC_ACCOUNT_ID_BASE + self.synthetic.len() as u64;
        self.synthetic.insert(original, mapped);
        self.original.insert(mapped, original);
        mapped
    }

    /// Entity state the writer keeps may already hold a rewritten ID.
    fn original_of(&self, id: u64) -> u64 {
        self.original.get(&id).copied().unwrap_or(id)
    }
}

#[derive(Default)]
struct Speaker {
    packets: u64,
    audio_bytes: u64,
}

struct Rules {
    apply: bool,
    cancel: Arc<AtomicBool>,
    progress: Progress,
    total_ticks: u32,
    last_progress: Instant,
    messages: u32,
    report: UnmuteReport,
    identities: Identities,
    names: HashMap<u64, String>,
    muted: BTreeSet<u64>,
    speakers: BTreeMap<u64, Speaker>,
}

impl Rules {
    fn new(apply: bool, cancel: Arc<AtomicBool>, progress: Progress, total_ticks: u32) -> Self {
        Self {
            apply,
            cancel,
            progress,
            total_ticks,
            // Report the first decoded message at once instead of after 100 ms.
            last_progress: Instant::now() - Duration::from_secs(1),
            messages: 0,
            report: UnmuteReport::default(),
            identities: Identities::default(),
            names: HashMap::new(),
            muted: BTreeSet::new(),
            speakers: BTreeMap::new(),
        }
    }

    /// Called for every packet message, so cancellation is seen within one
    /// demo packet. The clock is only read every few messages.
    fn heartbeat(&mut self, tick: u32) -> Result<(), ParserError> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(anyhow!("Operation cancelled").into());
        }
        if tick != u32::MAX {
            self.report.ticks = self.report.ticks.max(tick);
        }
        let due = self.messages.is_multiple_of(64);
        self.messages = self.messages.wrapping_add(1);
        if due && self.last_progress.elapsed() >= Duration::from_millis(100) {
            (self.progress)(if self.total_ticks > 0 && tick > 0 && tick != u32::MAX {
                (tick as f32 / self.total_ticks as f32).clamp(0.0, 0.999)
            } else {
                -1.0
            });
            self.last_progress = Instant::now();
        }
        Ok(())
    }

    fn remember_name(&mut self, original: u64, name: Option<&str>) {
        if original != 0
            && let Some(name) = name.filter(|name| !name.is_empty())
        {
            self.names
                .entry(original)
                .or_insert_with(|| name.to_owned());
        }
    }

    /// Real SteamID of a controller whose state the writer tracks.
    fn controller_id(&self, entity: &Entity) -> u64 {
        match entity.get_property(STEAM_ID) {
            Ok(FieldValue::Unsigned64(id)) => self.identities.original_of(*id),
            _ => 0,
        }
    }

    fn note_muted(&mut self, entity: &Entity) {
        let id = self.controller_id(entity);
        if id != 0 {
            self.muted.insert(id);
        }
    }

    fn voice_mask(&mut self, payload: &[u8]) -> Result<MessageRewrite, ParserError> {
        let mut mask = CUserMessageVoiceMask::decode(payload)?;
        self.report.voice_masks += 1;
        if !open_voice_mask(&mut mask) {
            return Ok(MessageRewrite::Keep);
        }
        self.report.voice_masks_changed += 1;
        Ok(self.replace(mask.encode_to_vec()))
    }

    fn voice_packet(&mut self, payload: &[u8]) -> Result<MessageRewrite, ParserError> {
        let mut packet = CSvcMsgVoiceData::decode(payload)?;
        let original = packet.xuid.unwrap_or(0);
        let audio_bytes = packet
            .audio
            .as_ref()
            .map_or(0, |audio| audio.voice_data().len() as u64);
        self.report.voice_packets += 1;
        self.report.voice_audio_bytes += audio_bytes;
        let speaker = self.speakers.entry(original).or_default();
        speaker.packets += 1;
        speaker.audio_bytes += audio_bytes;

        let mut changed = false;
        if packet.audible_mask != Some(-1) {
            packet.audible_mask = Some(-1);
            self.report.audible_masks_changed += 1;
            changed = true;
        }
        if original != 0 {
            packet.xuid = Some(self.identities.remap(original));
            self.report.voice_packet_ids_remapped += 1;
            changed = true;
        }
        if !changed {
            return Ok(MessageRewrite::Keep);
        }
        Ok(self.replace(packet.encode_to_vec()))
    }

    fn replace(&self, payload: Vec<u8>) -> MessageRewrite {
        if self.apply {
            MessageRewrite::Replace(payload)
        } else {
            MessageRewrite::Keep
        }
    }

    fn identity(&self, original: u64) -> UnmuteIdentity {
        UnmuteIdentity {
            name: self.names.get(&original).cloned().unwrap_or_default(),
            steam_id: original.to_string(),
            synthetic_steam_id: self
                .identities
                .synthetic
                .get(&original)
                .copied()
                .unwrap_or(0)
                .to_string(),
        }
    }

    fn report(&self) -> UnmuteReport {
        let mut identities: Vec<_> = self
            .identities
            .synthetic
            .keys()
            .map(|id| self.identity(*id))
            .collect();
        identities.sort_by(|a, b| a.synthetic_steam_id.cmp(&b.synthetic_steam_id));
        let mut speakers: Vec<_> = self
            .speakers
            .iter()
            .map(|(id, speaker)| UnmuteSpeaker {
                name: self.names.get(id).cloned().unwrap_or_default(),
                steam_id: (*id != 0).then(|| id.to_string()),
                packets: speaker.packets,
                audio_bytes: speaker.audio_bytes,
            })
            .collect();
        speakers.sort_by(|a, b| {
            b.audio_bytes
                .cmp(&a.audio_bytes)
                .then(b.packets.cmp(&a.packets))
                .then(a.steam_id.cmp(&b.steam_id))
        });
        UnmuteReport {
            identities_remapped: identities.len() as u64,
            muted_players: self.muted.iter().map(|id| self.identity(*id)).collect(),
            identities,
            speakers,
            ..self.report.clone()
        }
    }
}

/// Lets every slot hear every slot and lifts every ban. Returns whether the
/// message changed.
fn open_voice_mask(mask: &mut CUserMessageVoiceMask) -> bool {
    let mut changed = false;
    for word in &mut mask.gamerules_masks {
        changed |= *word != u32::MAX;
        *word = u32::MAX;
    }
    for word in &mut mask.ban_masks {
        changed |= *word != 0;
        *word = 0;
    }
    changed |= mask.mod_enable != Some(true);
    mask.mod_enable = Some(true);
    changed
}

/// Zero in the integer type the field decoded to, or `None` when the value is
/// already zero or is not an integer. The writer re-encodes with the field's
/// own decoder, which rejects any other variant.
fn cleared_flags(value: &FieldValue) -> Option<FieldValue> {
    Some(match value {
        FieldValue::Unsigned8(v) if *v != 0 => FieldValue::Unsigned8(0),
        FieldValue::Unsigned16(v) if *v != 0 => FieldValue::Unsigned16(0),
        FieldValue::Unsigned32(v) if *v != 0 => FieldValue::Unsigned32(0),
        FieldValue::Unsigned64(v) if *v != 0 => FieldValue::Unsigned64(0),
        FieldValue::Signed8(v) if *v != 0 => FieldValue::Signed8(0),
        FieldValue::Signed16(v) if *v != 0 => FieldValue::Signed16(0),
        FieldValue::Signed32(v) if *v != 0 => FieldValue::Signed32(0),
        FieldValue::Signed64(v) if *v != 0 => FieldValue::Signed64(0),
        _ => return None,
    })
}

impl DemoRewriter for Rules {
    fn interests(&self) -> RewriteInterests {
        RewriteInterests::PACKET_MESSAGE
            | RewriteInterests::STRING_TABLE_ENTRIES
            | RewriteInterests::ENTITY_FIELDS
    }

    fn rewrite_packet_message(
        &mut self,
        ctx: &source2_demo::Context,
        _tick: u32,
        msg_type: i32,
        payload: &[u8],
    ) -> Result<MessageRewrite, ParserError> {
        self.heartbeat(ctx.tick())?;
        if msg_type == SvcMessages::SvcVoiceData as i32 {
            self.voice_packet(payload)
        } else if msg_type == EBaseUserMessages::UmVoiceMask as i32 {
            self.voice_mask(payload)
        } else {
            Ok(MessageRewrite::Keep)
        }
    }

    fn rewrite_string_table_entry(
        &mut self,
        _ctx: &source2_demo::Context,
        _tick: u32,
        table_name: &str,
        entry: &mut StringTableEntryUpdate,
    ) -> Result<(), ParserError> {
        if table_name != USERINFO {
            return Ok(());
        }
        let Some(value) = entry.value() else {
            return Ok(());
        };
        // A row that cannot be read cannot be kept consistent: fail the run.
        let mut player = CMsgPlayerInfo::decode(value)?;
        let original = player.xuid.or(player.steamid).unwrap_or(0);
        self.remember_name(original, player.name.as_deref());
        if original == 0 {
            return Ok(());
        }
        let mapped = self.identities.remap(original);
        self.report.userinfo_rows_remapped += 1;
        if self.apply {
            player.xuid = Some(mapped);
            player.steamid = Some(mapped);
            entry.set_value(player.encode_to_vec());
        }
        Ok(())
    }

    fn replace_entity_field(
        &mut self,
        _ctx: &source2_demo::Context,
        _event: EntityEvents,
        entity: &Entity,
        field_name: &str,
        value: &FieldValue,
    ) -> Option<FieldValue> {
        if entity.class().name() != CONTROLLER {
            return None;
        }
        match field_name {
            ABUSE_MUTE => {
                let FieldValue::Boolean(true) = value else {
                    return None;
                };
                self.report.abuse_mutes_cleared += 1;
                self.note_muted(entity);
                self.apply.then_some(FieldValue::Boolean(false))
            }
            MUTE_FLAGS => {
                let cleared = cleared_flags(value)?;
                self.report.mute_flags_cleared += 1;
                self.note_muted(entity);
                self.apply.then_some(cleared)
            }
            STEAM_ID => {
                let FieldValue::Unsigned64(original) = *value else {
                    return None;
                };
                if original == 0 {
                    return None;
                }
                if let Ok(FieldValue::String(name)) = entity.get_property(PLAYER_NAME) {
                    self.remember_name(original, Some(name));
                }
                let mapped = self.identities.remap(original);
                self.report.controller_ids_remapped += 1;
                self.apply.then_some(FieldValue::Unsigned64(mapped))
            }
            PLAYER_NAME => {
                if let FieldValue::String(name) = value {
                    self.remember_name(self.controller_id(entity), Some(name));
                }
                None
            }
            _ => None,
        }
    }

    /// Only controllers are decoded for rewriting; every other entity is
    /// skipped bit-exactly.
    fn should_rewrite_entity(
        &mut self,
        _ctx: &source2_demo::Context,
        _event: EntityEvents,
        entity: &Entity,
    ) -> bool {
        entity.class().name() == CONTROLLER
    }

    /// Controller state is kept so a mute can be attributed to a SteamID.
    fn should_track_entity(
        &mut self,
        _ctx: &source2_demo::Context,
        _event: EntityEvents,
        entity: &Entity,
    ) -> bool {
        entity.class().name() == CONTROLLER
    }
}

#[cfg(test)]
mod tests;
