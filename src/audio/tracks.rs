use super::RATE;
use crate::{
    files,
    model::{Clip, ClipKind},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

struct Input {
    start: u64,
    end: u64,
    path: PathBuf,
}

pub(super) fn render(
    root: &Path,
    clips: &[Clip],
    everyone: bool,
    cancel: &AtomicBool,
) -> Result<Vec<Clip>> {
    let segments: Vec<_> = clips
        .iter()
        .filter(|clip| clip.kind == ClipKind::Segment)
        .collect();
    if segments.is_empty() {
        return Ok(vec![]);
    }
    let mut tracks = Vec::new();
    if everyone {
        tracks.push(mix(
            root,
            &segments,
            "everyone",
            "Everyone",
            None,
            ClipKind::Everyone,
            cancel,
        )?);
    }
    let mut speakers = BTreeMap::<&str, Vec<&Clip>>::new();
    for clip in segments {
        if let Some(steam) = clip.steam_id.as_deref() {
            speakers.entry(steam).or_default().push(clip);
        }
    }
    for (steam, clips) in speakers {
        let name = format!("player_{}", files::short_hash(steam.as_bytes()));
        tracks.push(mix(
            root,
            &clips,
            &name,
            &clips[0].player,
            Some(steam),
            ClipKind::PlayerTrack,
            cancel,
        )?);
    }
    Ok(tracks)
}

fn open(path: &Path) -> Result<hound::WavReader<BufReader<File>>> {
    let reader = hound::WavReader::open(path)
        .with_context(|| format!("Read voice clip {}", path.display()))?;
    let spec = reader.spec();
    ensure!(
        spec.channels == 1
            && spec.sample_rate == RATE
            && spec.bits_per_sample == 16
            && spec.sample_format == hound::SampleFormat::Int,
        "Voice track requires 48 kHz mono 16-bit PCM clips"
    );
    Ok(reader)
}

fn mix(
    root: &Path,
    clips: &[&Clip],
    name: &str,
    label: &str,
    steam: Option<&str>,
    kind: ClipKind,
    cancel: &AtomicBool,
) -> Result<Clip> {
    let mut inputs = Vec::with_capacity(clips.len());
    for clip in clips {
        ensure!(!cancel.load(Ordering::Relaxed), "Operation cancelled");
        ensure!(
            clip.demo_seconds.is_finite() && (0.0..=u32::MAX as f64).contains(&clip.demo_seconds),
            "Invalid recorded voice timestamp"
        );
        let samples = open(&clip.path)?.len();
        ensure!(samples > 0, "Recorded voice clip is empty");
        let start = (clip.demo_seconds * RATE as f64).round() as u64;
        inputs.push(Input {
            start,
            end: start
                .checked_add(samples.into())
                .context("Voice timestamp overflow")?,
            path: clip.path.clone(),
        });
    }
    inputs.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then(left.path.cmp(&right.path))
    });
    let mut windows: Vec<(u64, u64)> = Vec::new();
    for input in &inputs {
        if let Some(last) = windows.last_mut().filter(|last| input.start <= last.1) {
            last.1 = last.1.max(input.end);
        } else {
            windows.push((input.start, input.end));
        }
    }
    // Keep recorded ordering and overlaps, but do not write hours of silent gaps.
    let total: u64 = windows.iter().map(|(start, end)| end - start).sum();
    ensure!(
        total > 0 && total <= RATE as u64 * 60 * 60 * 6,
        "Voice track exceeds the six-hour WAV safety limit"
    );
    let directory = root.join("tracks");
    fs::create_dir_all(&directory)?;
    let path = files::unique_path(&directory, &format!("{name}.wav"));
    let output = File::options().write(true).create_new(true).open(&path)?;
    let mut writer = hound::WavWriter::new(
        BufWriter::new(output),
        hound::WavSpec {
            channels: 1,
            sample_rate: RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    let bucket = total.div_ceil(64);
    let mut peaks = vec![0_f32; total.div_ceil(bucket) as usize];
    let mut written = 0_u64;
    let mut next = 0;
    let mut active: Vec<(usize, hound::WavReader<BufReader<File>>)> = Vec::new();
    for (start, end) in windows {
        let mut cursor = start;
        while cursor < end {
            ensure!(!cancel.load(Ordering::Relaxed), "Operation cancelled");
            let chunk_end = (cursor + 4_800).min(end);
            active.retain(|(index, _)| inputs[*index].end > cursor);
            while next < inputs.len() && inputs[next].start < chunk_end {
                if inputs[next].end > cursor {
                    active.push((next, open(&inputs[next].path)?));
                }
                next += 1;
            }
            let count = (chunk_end - cursor) as usize;
            let mut sum = vec![0_i64; count];
            let mut voices = vec![0_u16; count];
            for (index, reader) in &mut active {
                let input = &inputs[*index];
                let from = input.start.max(cursor);
                let to = input.end.min(chunk_end);
                if to <= from {
                    continue;
                }
                reader.seek((from - input.start) as u32)?;
                let length = (to - from) as usize;
                let offset = (from - cursor) as usize;
                let mut actual = 0;
                for (index, sample) in reader.samples::<i16>().take(length).enumerate() {
                    sum[offset + index] += i64::from(sample?);
                    voices[offset + index] = voices[offset + index]
                        .checked_add(1)
                        .context("Too many overlapping voice clips")?;
                    actual += 1;
                }
                ensure!(
                    actual == length,
                    "Voice clip changed or was truncated while mixing"
                );
            }
            for (sum, voices) in sum.into_iter().zip(voices) {
                ensure!(
                    voices > 0,
                    "Missing voice samples inside a recorded interval"
                );
                // Average overlapping voices for headroom, without changing solo audio.
                let sample =
                    (sum / i64::from(voices)).clamp(i16::MIN as i64, i16::MAX as i64) as i16;
                writer.write_sample(sample)?;
                let peak = (sample as f32).abs() / 32768.0;
                let slot = (written / bucket) as usize;
                peaks[slot] = peaks[slot].max(peak);
                written += 1;
            }
            cursor = chunk_end;
        }
    }
    writer.finalize()?;
    ensure!(written == total, "Voice track length mismatch");
    Ok(Clip {
        id: format!(
            "track_{}_{}",
            files::short_hash(root.to_string_lossy().as_bytes()),
            name
        ),
        player: label.into(),
        steam_id: steam.map(str::to_owned),
        round: None,
        demo_seconds: inputs[0].start as f64 / RATE as f64,
        duration: total as f64 / RATE as f64,
        path,
        peaks,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(root: &Path, name: &str, start: f64, steam: &str, samples: &[i16]) -> Clip {
        let path = root.join(format!("{name}.wav"));
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 1,
                sample_rate: RATE,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .expect("fixture WAV");
        for sample in samples {
            writer.write_sample(*sample).expect("sample");
        }
        writer.finalize().expect("finalize");
        Clip {
            id: name.into(),
            player: steam.into(),
            steam_id: Some(steam.into()),
            demo_seconds: start,
            duration: samples.len() as f64 / RATE as f64,
            path,
            ..Default::default()
        }
    }

    fn samples(clip: &Clip) -> Vec<i16> {
        open(&clip.path)
            .expect("track")
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .expect("samples")
    }

    #[test]
    fn everyone_mix_preserves_order_overlaps_and_separate_player_tracks() {
        let root = tempfile::tempdir().expect("dir");
        let clips = vec![
            clip(
                root.path(),
                "a",
                0.0,
                "first",
                &[1_000, 2_000, 3_000, 4_000],
            ),
            clip(
                root.path(),
                "b",
                2.0 / RATE as f64,
                "second",
                &[5_000, 6_000],
            ),
            clip(root.path(), "c", 1.0, "first", &[7_000]),
        ];
        let before = files::hash_file(&clips[0].path).expect("hash");
        let tracks = render(root.path(), &clips, true, &AtomicBool::new(false)).expect("render");
        assert_eq!(tracks.len(), 3);
        let everyone = tracks
            .iter()
            .find(|track| track.kind == ClipKind::Everyone)
            .expect("everyone");
        assert_eq!(samples(everyone), [1_000, 2_000, 4_000, 5_000, 7_000]);
        let first = tracks
            .iter()
            .find(|track| track.steam_id.as_deref() == Some("first"))
            .expect("first");
        assert_eq!(samples(first), [1_000, 2_000, 3_000, 4_000, 7_000]);
        let second = tracks
            .iter()
            .find(|track| track.steam_id.as_deref() == Some("second"))
            .expect("second");
        assert_eq!(samples(second), [5_000, 6_000]);
        assert_eq!(files::hash_file(&clips[0].path).expect("hash"), before);
        assert_eq!(
            everyone.peaks,
            super::super::waveform(&samples(everyone), 64)
        );
        let repeated =
            render(root.path(), &clips, true, &AtomicBool::new(false)).expect("no overwrite");
        assert_ne!(everyone.path, repeated[0].path);
        assert_eq!(samples(everyone), [1_000, 2_000, 4_000, 5_000, 7_000]);
    }

    #[test]
    fn player_extraction_does_not_mislabel_a_partial_mix_as_everyone() {
        let root = tempfile::tempdir().expect("dir");
        let clips = vec![clip(root.path(), "a", 0.0, "first", &[100, -100])];
        let tracks = render(root.path(), &clips, false, &AtomicBool::new(false)).expect("player");
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].kind, ClipKind::PlayerTrack);
        assert!(render(root.path(), &clips, true, &AtomicBool::new(true)).is_err());
    }
}
