use crate::{
    files,
    model::{Clip, Player},
};
use anyhow::{Context, Result, ensure};
use source2_demo::proto::{CMsgVoiceAudio, CSvcMsgVoiceData};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

const RATE: u32 = 48000;

struct Speaker {
    decoder: opus::Decoder,
    steam_id: Option<String>,
    label: String,
    start_tick: u32,
    last_tick: u32,
    round: Option<u32>,
    samples: Vec<i16>,
}

/// Decodes on the parser thread. Memory is bounded by active clips, not the
/// entire demo. A new extraction uses a separate directory and never deletes
/// previous audio, even on cancellation or decoder failure.
pub struct AudioWriter {
    root: PathBuf,
    speakers: HashMap<String, Speaker>,
    clips: Vec<Clip>,
    filter: Option<String>,
    interval: Option<f32>,
}

impl AudioWriter {
    pub fn new(output: &Path, player_key: Option<String>) -> Result<Self> {
        fs::create_dir_all(output)?;
        ensure!(
            !output.join("clips.json").exists(),
            "This output directory already contains an extraction. Choose a new directory; existing clips were not replaced"
        );
        Ok(Self {
            root: output.to_owned(),
            speakers: HashMap::new(),
            clips: vec![],
            filter: player_key,
            interval: None,
        })
    }

    pub fn push(
        &mut self,
        message: CSvcMsgVoiceData,
        tick: u32,
        round: Option<u32>,
        interval: Option<f32>,
    ) -> Result<()> {
        let Some(audio) = message.audio else {
            return Ok(());
        };
        if audio.voice_data().is_empty() {
            return Ok(());
        }
        let steam = message.xuid.filter(|id| *id != 0).map(|id| id.to_string());
        let entity = message
            .entity
            .filter(|i| *i > 0)
            .or_else(|| message.client_deprecated.filter(|i| *i >= 0).map(|i| i + 1));
        let key = steam
            .clone()
            .unwrap_or_else(|| format!("entity:{}", entity.unwrap_or(-1)));
        if self.filter.as_ref().is_some_and(|f| f != &key) {
            return Ok(());
        }
        ensure!(
            audio.format() as i32 == 2,
            "Unsupported recorded voice codec {:?}; only raw Opus is supported. No clips were replaced",
            audio.format()
        );
        let interval = interval
            .context("Voice packet has no recorded tick interval; refusing to invent timestamps")?;
        self.interval = Some(interval);
        ensure!(tick != u32::MAX, "Voice packet has an invalid demo tick");
        let split = self.speakers.get(&key).is_some_and(|s| {
            (tick.saturating_sub(s.last_tick) as f32 * interval) > 0.75
                || s.round != round
                || s.samples.len() >= RATE as usize * 30
        });
        if split {
            self.flush(&key)?;
        }
        if !self.speakers.contains_key(&key) {
            self.speakers.insert(
                key.clone(),
                Speaker {
                    decoder: opus::Decoder::new(RATE, opus::Channels::Mono)?,
                    steam_id: steam.clone(),
                    label: steam.unwrap_or_else(|| {
                        format!("Unknown speaker (slot {})", entity.unwrap_or(-1))
                    }),
                    start_tick: tick,
                    last_tick: tick,
                    round,
                    samples: vec![],
                },
            );
        }
        let speaker = self
            .speakers
            .get_mut(&key)
            .context("Missing speaker decoder")?;
        let packets = opus_packets(&audio)?;
        let mut decoded = [0_i16; 5760]; // Opus maximum: 120 ms at 48 kHz.
        for packet in packets {
            let count = speaker
                .decoder
                .decode(packet, &mut decoded, false)
                .context("Invalid Opus voice packet")?;
            speaker.samples.extend_from_slice(&decoded[..count]);
        }
        speaker.last_tick = tick;
        Ok(())
    }

    fn flush(&mut self, key: &str) -> Result<()> {
        let Some(speaker) = self.speakers.remove(key) else {
            return Ok(());
        };
        if speaker.samples.is_empty() {
            return Ok(());
        }
        let folder = self.root.join(files::short_hash(key.as_bytes()));
        fs::create_dir_all(&folder)?;
        let id = format!(
            "{}_{}_{}",
            files::short_hash(self.root.to_string_lossy().as_bytes()),
            speaker.start_tick,
            self.clips.len()
        );
        let path = files::unique_path(
            &folder,
            &format!("round_{}_tick_{id}.wav", speaker.round.unwrap_or(0)),
        );
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec)?;
        for &sample in &speaker.samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        self.clips.push(Clip {
            id,
            player: speaker.label,
            steam_id: speaker.steam_id,
            round: speaker.round,
            demo_seconds: speaker.start_tick as f64
                * self.interval.context("Missing tick interval")? as f64,
            duration: speaker.samples.len() as f64 / RATE as f64,
            path,
            peaks: waveform(&speaker.samples, 64),
        });
        Ok(())
    }

    pub fn finish(mut self, players: &[Player]) -> Result<Vec<Clip>> {
        for key in self.speakers.keys().cloned().collect::<Vec<_>>() {
            self.flush(&key)?;
        }
        for clip in &mut self.clips {
            if let Some(player) = players
                .iter()
                .find(|p| p.steam_id.is_some() && p.steam_id == clip.steam_id)
            {
                clip.player = player.name.clone();
            }
        }
        self.clips.sort_by(|a, b| {
            a.demo_seconds
                .total_cmp(&b.demo_seconds)
                .then(a.player.cmp(&b.player))
        });
        files::atomic_json(&self.root.join("clips.json"), &self.clips)?;
        Ok(self.clips)
    }
}

fn opus_packets(audio: &CMsgVoiceAudio) -> Result<Vec<&[u8]>> {
    let bytes = audio.voice_data();
    if audio.packet_offsets.is_empty() {
        ensure!(
            audio.num_packets.unwrap_or(1) <= 1,
            "Voice message declares multiple Opus packets without offsets"
        );
        return Ok(vec![bytes]);
    }
    // Valve records cumulative END offsets, padded with unused zero entries.
    // num_packets is authoritative: [32, 0, 0, 0] with n=1 is one 32-byte packet.
    let count = audio
        .num_packets
        .map(|n| n as usize)
        .unwrap_or_else(|| audio.packet_offsets.iter().take_while(|&&n| n != 0).count());
    ensure!(
        count > 0 && count <= audio.packet_offsets.len(),
        "Invalid Opus packet count"
    );
    let mut starts = vec![0_usize];
    starts.extend(audio.packet_offsets.iter().take(count).map(|n| *n as usize));
    ensure!(
        starts
            .windows(2)
            .all(|w| w[0] < w[1] && w[1] <= bytes.len())
            && starts.last() == Some(&bytes.len()),
        "Invalid Opus packet offsets {:?} ({} encoded bytes, num_packets={:?})",
        audio.packet_offsets,
        bytes.len(),
        audio.num_packets
    );
    Ok(starts.windows(2).map(|w| &bytes[w[0]..w[1]]).collect())
}

pub fn waveform(samples: &[i16], bars: usize) -> Vec<f32> {
    if samples.is_empty() || bars == 0 {
        return vec![];
    }
    samples
        .chunks(samples.len().div_ceil(bars))
        .map(|chunk| {
            chunk
                .iter()
                .map(|v| (*v as f32).abs() / 32768.0)
                .fold(0.0_f32, f32::max)
        })
        .collect()
}

pub fn list_clips(base: &Path) -> Result<Vec<Clip>> {
    if !base.is_dir() {
        return Ok(vec![]);
    }
    let mut clips = Vec::new();
    for entry in fs::read_dir(base)? {
        let manifest = entry?.path().join("clips.json");
        if !manifest.is_file() {
            continue;
        }
        let entries: Vec<Clip> = serde_json::from_reader(BufReader::new(File::open(&manifest)?))
            .with_context(|| format!("Read voice manifest {}", manifest.display()))?;
        clips.extend(entries.into_iter().filter(|c| c.path.is_file()));
    }
    Ok(clips)
}

pub struct Playback {
    _stream: rodio::OutputStream,
    sink: rodio::Sink,
    pub clip_id: String,
}

impl Playback {
    pub fn play(clip: &Clip) -> Result<Self> {
        let stream = rodio::OutputStreamBuilder::open_default_stream()
            .context("No audio output device is available")?;
        let sink = rodio::Sink::connect_new(stream.mixer());
        let source = rodio::Decoder::new(BufReader::new(File::open(&clip.path)?))?;
        sink.append(source);
        Ok(Self {
            _stream: stream,
            sink,
            clip_id: clip.id.clone(),
        })
    }
    pub fn position(&self) -> f64 {
        self.sink.get_pos().as_secs_f64()
    }
    pub fn finished(&self) -> bool {
        self.sink.empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waveform_measures_samples_and_silence() {
        assert_eq!(waveform(&[0, 0, 0, 0], 2), [0.0, 0.0]);
        assert_eq!(waveform(&[0, i16::MIN, 0, 16384], 2), [1.0, 0.5]);
    }
    #[test]
    fn packet_offsets_are_checked_before_slicing() {
        let audio = CMsgVoiceAudio {
            voice_data: Some(vec![1, 2, 3]),
            packet_offsets: vec![0, 9],
            ..Default::default()
        };
        assert!(opus_packets(&audio).is_err());
    }

    #[test]
    fn valve_packet_end_offsets_ignore_unused_padding() {
        let audio = CMsgVoiceAudio {
            voice_data: Some(vec![1, 2, 3, 4, 5]),
            num_packets: Some(2),
            packet_offsets: vec![2, 5, 0, 0],
            ..Default::default()
        };
        assert_eq!(
            opus_packets(&audio).expect("packets"),
            [vec![1, 2], vec![3, 4, 5]]
        );
    }
    #[test]
    fn real_opus_roundtrip_writes_playable_wav_and_manifest() {
        let dir = tempfile::tempdir().expect("dir");
        let mut encoder = opus::Encoder::new(RATE, opus::Channels::Mono, opus::Application::Voip)
            .expect("encoder");
        let input: Vec<i16> = (0..960)
            .map(|i| ((i as f32 * 0.05).sin() * 10000.0) as i16)
            .collect();
        let mut encoded = [0; 4000];
        let n = encoder.encode(&input, &mut encoded).expect("encode");
        let mut writer = AudioWriter::new(dir.path(), None).expect("writer");
        for packet in 0..2 {
            writer
                .push(
                    CSvcMsgVoiceData {
                        xuid: Some(76561198000000001),
                        audio: Some(CMsgVoiceAudio {
                            format: Some(2),
                            voice_data: Some(encoded[..n].to_vec()),
                            section_number: Some(packet),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    64 + packet,
                    Some(1),
                    Some(1.0 / 64.0),
                )
                .expect("push");
        }
        let clips = writer.finish(&[]).expect("finish");
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].duration, 0.04);
        assert_eq!(clips[0].demo_seconds, 1.0);
        let wav = hound::WavReader::open(&clips[0].path).expect("wav");
        assert_eq!(wav.spec().sample_rate, RATE);
        assert_eq!(wav.duration(), 1920);
    }
}
