use crate::model::FileStamp;
use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};

pub fn stamp(path: &Path) -> Result<FileStamp> {
    let meta = path.metadata()?;
    ensure!(meta.is_file(), "Not a regular file: {}", path.display());
    Ok(FileStamp {
        size: meta.len(),
        modified_ns: meta
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
    })
}

pub fn path_key(path: &Path) -> String {
    let canonical = path.canonicalize().ok();
    let path = canonical.as_deref().unwrap_or(path);
    let path = path.to_string_lossy().replace('\\', "/");
    // canonicalize() uses extended Windows paths; legacy JSON and picker paths
    // generally do not. They must still refer to the same library entry.
    let path = if let Some(rest) = path.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else {
        path.strip_prefix("//?/").unwrap_or(&path).to_owned()
    };
    if cfg!(windows) {
        path.to_lowercase()
    } else {
        path
    }
}

pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    }
}

pub fn short_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))[..24].to_owned()
}

/// Lightweight relocation fingerprint. Used only for otherwise missing entries.
/// Actual move/copy verification below always hashes the entire file.
pub fn fingerprint(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hash = Sha256::new();
    hash.update(size.to_le_bytes());
    let mut buffer = vec![0; 65536];
    let n = file.read(&mut buffer)?;
    hash.update(&buffer[..n]);
    if size > 65536 {
        file.seek(SeekFrom::Start(size.saturating_sub(65536)))?;
        let n = file.read(&mut buffer)?;
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn hash_file(path: &Path) -> Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hash.finalize().into())
}

pub fn demo_filename(input: &str) -> Result<String> {
    ensure!(
        !input.chars().any(char::is_control),
        "Filename contains a control character"
    );
    let name = input.trim();
    ensure!(!name.is_empty(), "A filename is required");
    ensure!(
        name.len() <= 180,
        "Filename is too long (maximum 180 bytes)"
    );
    ensure!(
        !name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c)),
        "Filename contains a path separator or a forbidden character"
    );
    ensure!(
        !name.ends_with(['.', ' ']) && name != "." && name != "..",
        "Filename must not end with a dot or space"
    );
    let stem = name
        .strip_suffix(".dem")
        .or_else(|| name.strip_suffix(".DEM"))
        .unwrap_or(name);
    let device = stem.split('.').next().unwrap_or("").to_uppercase();
    ensure!(
        ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
        ]
        .contains(&device.as_str()),
        "That filename is reserved by Windows"
    );
    ensure!(!stem.is_empty(), "A filename is required before .dem");
    Ok(format!("{stem}.dem"))
}

pub fn is_demo(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("dem"))
}

pub fn validate_demo(path: &Path) -> Result<()> {
    validate_demo_cancellable(path, &AtomicBool::new(false))
}

pub fn validate_demo_cancellable(path: &Path, cancel: &AtomicBool) -> Result<()> {
    ensure!(!cancel.load(Ordering::Relaxed), "Operation cancelled");
    let mut file = File::open(path).with_context(|| format!("Open {}", path.display()))?;
    let size = file.metadata()?.len();
    ensure!(size > 16, "Truncated demo header");
    let mut header = [0; 16];
    file.read_exact(&mut header)?;
    ensure!(
        &header[..8] == b"PBDEMS2\0",
        "Not a CS2 / Source 2 demo (Source 1 demos are not supported)"
    );
    let offset = u32::from_le_bytes(header[8..12].try_into()?) as u64;
    ensure!(
        offset == 0 || (offset >= 16 && offset < size),
        "Demo is unfinished or has a corrupt file-info offset. Wait until recording/downloading finishes"
    );
    validate_outer_frames(&mut file, size, cancel)?;
    Ok(())
}

fn validate_outer_frames(file: &mut File, size: u64, cancel: &AtomicBool) -> Result<()> {
    use std::io::BufReader;
    fn varint(reader: &mut impl Read) -> Result<u32> {
        let mut value = 0_u32;
        for shift in (0..35).step_by(7) {
            let mut byte = [0];
            reader
                .read_exact(&mut byte)
                .context("Truncated demo command header")?;
            ensure!(shift != 28 || byte[0] <= 15, "Invalid demo command varint");
            value |= (byte[0] as u32 & 127) << shift;
            if byte[0] & 128 == 0 {
                return Ok(value);
            }
        }
        bail!("Invalid demo command varint")
    }
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(16))?;
    let mut position = 16;
    while position < size {
        ensure!(!cancel.load(Ordering::Relaxed), "Operation cancelled");
        let command = varint(&mut reader)?;
        let tick = varint(&mut reader)?;
        let length = varint(&mut reader)? as u64;
        let payload_start = reader.stream_position()?;
        ensure!(
            length <= size.saturating_sub(payload_start),
            "Truncated demo command {} at tick {tick}: needs {length} bytes, only {} remain. Finish downloading/recording this demo before parsing it",
            command & !64,
            size.saturating_sub(payload_start)
        );
        ensure!(
            length <= 128 * 1024 * 1024,
            "Demo command exceeds the 128 MiB safety limit"
        );
        reader.seek_relative(length as i64)?;
        position = payload_start + length;
    }
    Ok(())
}

/// Commit a fully written, verified copy without ever overwriting a destination.
/// Delete the source only after the destination and optional backup are durable.
pub fn transfer(
    source: &Path,
    destination: &Path,
    move_source: bool,
    backup_dir: Option<&Path>,
) -> Result<()> {
    ensure!(
        path_key(source) != path_key(destination),
        "Source and destination are the same file"
    );
    ensure!(
        !destination.exists(),
        "Destination already exists: {}",
        destination.display()
    );
    validate_demo(source)?;
    let before = stamp(source)?;
    if move_source && let Some(dir) = backup_dir {
        fs::create_dir_all(dir)?;
        let name = source.file_name().context("Source has no filename")?;
        let backup = unique_path(dir, &name.to_string_lossy());
        verified_copy(source, &backup)?;
    }
    verified_copy(source, destination)?;
    if move_source {
        ensure!(
            stamp(source)? == before,
            "Source changed during transfer; original was kept"
        );
        fs::remove_file(source).context(
            "Copy succeeded, but the original could not be removed; both files were kept",
        )?;
    }
    Ok(())
}

fn verified_copy(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .context("Destination has no parent directory")?;
    ensure!(
        parent.is_dir(),
        "Destination directory does not exist: {}",
        parent.display()
    );
    let before = stamp(source)?;
    let mut input = File::open(source)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    let mut source_hash = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        temp.write_all(&buffer[..n])?;
        source_hash.update(&buffer[..n]);
    }
    temp.as_file().sync_all()?;
    ensure!(
        stamp(source)? == before,
        "Source changed during copy; nothing was committed"
    );
    let expected: [u8; 32] = source_hash.finalize().into();
    ensure!(
        hash_file(temp.path())? == expected,
        "SHA-256 verification failed; original was kept"
    );
    // persist_noclobber uses a no-replace operation, including on Windows.
    temp.persist_noclobber(destination)
        .map_err(|e| e.error)
        .context("Commit verified copy (destination must not exist)")?;
    Ok(())
}

pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 1.. {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

pub fn rename(source: &Path, name: &str) -> Result<PathBuf> {
    let target = source
        .parent()
        .context("No parent folder")?
        .join(demo_filename(name)?);
    ensure!(
        path_key(source) != path_key(&target),
        "Choose a different filename"
    );
    // Use the same no-clobber verified transfer as cross-volume moves.
    transfer(source, &target, true, None)?;
    Ok(target)
}

/// ZIP import accepts demo members only, never trusts archive member paths, and
/// limits decompressed size. Source archives and external files are never deleted.
pub fn import(paths: &[PathBuf], destination: &Path) -> Result<Vec<PathBuf>> {
    ensure!(
        destination.is_dir(),
        "Add or select an available watched folder first"
    );
    let mut imported = Vec::new();
    for source in paths {
        if is_demo(source) {
            validate_demo(source)?;
            if source
                .parent()
                .is_some_and(|p| path_key(p) == path_key(destination))
            {
                imported.push(source.clone());
            } else {
                let name = source.file_name().context("No filename")?.to_string_lossy();
                let target = unique_path(destination, &demo_filename(&name)?);
                transfer(source, &target, false, None)?;
                imported.push(target);
            }
        } else if source
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
        {
            let mut archive = zip::ZipArchive::new(File::open(source)?)?;
            let mut total = 0_u64;
            ensure!(archive.len() <= 10_000, "ZIP contains too many entries");
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i)?;
                let Some(enclosed) = entry.enclosed_name() else {
                    bail!("ZIP contains an unsafe member path");
                };
                if !is_demo(&enclosed) || entry.is_dir() {
                    continue;
                }
                total = total.saturating_add(entry.size());
                ensure!(
                    total <= 8 * 1024 * 1024 * 1024,
                    "ZIP demo output exceeds the 8 GiB safety limit"
                );
                let name = demo_filename(
                    &enclosed
                        .file_name()
                        .context("No member filename")?
                        .to_string_lossy(),
                )?;
                let target = unique_path(destination, &name);
                let mut temp = tempfile::NamedTempFile::new_in(destination)?;
                let limit = entry.size().saturating_add(1);
                let written = std::io::copy(&mut (&mut entry).take(limit), &mut temp)?;
                ensure!(
                    written == entry.size(),
                    "ZIP member size differs from its declared size"
                );
                temp.as_file().sync_all()?;
                validate_demo(temp.path())?;
                temp.persist_noclobber(&target).map_err(|e| e.error)?;
                imported.push(target);
            }
        } else {
            bail!(
                "Unsupported import: {}. Choose .dem or .zip; extract RAR/7z first",
                source.display()
            );
        }
    }
    ensure!(!imported.is_empty(), "No .dem files found in the import");
    Ok(imported)
}

pub fn reveal(path: &Path) -> Result<()> {
    ensure!(path.exists(), "File is no longer available");
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .spawn()?;
    }
    #[cfg(not(windows))]
    open::that(path.parent().unwrap_or(path))?;
    Ok(())
}

pub fn atomic_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path.parent().context("Missing data directory")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut writer = BufWriter::new(&mut temp);
        serde_json::to_writer_pretty(&mut writer, value)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;

    fn fake_demo(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .expect("file");
        f.write_all(b"PBDEMS2\0\x10\0\0\0\0\0\0\0\x02\0\0")
            .expect("write");
        path
    }

    #[test]
    fn rejects_traversal_reserved_names_and_empty_stems() {
        for name in [
            "../bad", "C:\\bad", "CON", "nul.txt", "a/b", "a.", ".dem", "", "evil\n",
        ] {
            assert!(demo_filename(name).is_err(), "accepted {name}");
        }
        assert_eq!(demo_filename("match.dem").expect("name"), "match.dem");
    }

    #[test]
    fn verified_copy_and_move_keep_backups_and_never_overwrite() {
        let dir = tempfile::tempdir().expect("dir");
        let source = fake_demo(dir.path(), "source.dem");
        let target = dir.path().join("copy.dem");
        transfer(&source, &target, false, None).expect("copy");
        assert!(source.exists());
        assert_eq!(
            hash_file(&source).expect("hash"),
            hash_file(&target).expect("hash")
        );
        assert!(transfer(&source, &target, true, None).is_err());
        assert!(source.exists());
        let moved = dir.path().join("moved.dem");
        let backup = dir.path().join("backups");
        transfer(&source, &moved, true, Some(&backup)).expect("move");
        assert!(!source.exists());
        assert!(backup.join("source.dem").exists());
    }

    #[test]
    fn corrupt_and_incomplete_headers_are_rejected() {
        let dir = tempfile::tempdir().expect("dir");
        let file = dir.path().join("bad.dem");
        fs::write(&file, b"PBDEMS2\0\0\0\0\0\0\0\0\0").expect("write");
        assert!(validate_demo(&file).is_err());
    }

    #[test]
    fn zip_import_rejects_traversal_and_keeps_the_source_archive() {
        use zip::write::SimpleFileOptions;
        let dir = tempfile::tempdir().expect("dir");
        let archive_path = dir.path().join("unsafe.zip");
        let mut archive = zip::ZipWriter::new(File::create(&archive_path).expect("file"));
        archive
            .start_file("../escape.dem", SimpleFileOptions::default())
            .expect("entry");
        archive
            .write_all(b"PBDEMS2\0\x10\0\0\0\0\0\0\0\x02\0\0")
            .expect("write");
        archive.finish().expect("finish");
        let destination = dir.path().join("demos");
        fs::create_dir(&destination).expect("dir");
        assert!(import(std::slice::from_ref(&archive_path), &destination).is_err());
        assert!(archive_path.exists());
        assert!(!dir.path().join("escape.dem").exists());
    }

    #[test]
    fn zip_import_flattens_safe_members_and_does_not_overwrite() {
        use zip::write::SimpleFileOptions;
        let dir = tempfile::tempdir().expect("dir");
        let archive_path = dir.path().join("matches.zip");
        let mut archive = zip::ZipWriter::new(File::create(&archive_path).expect("file"));
        archive
            .start_file("nested/match.dem", SimpleFileOptions::default())
            .expect("entry");
        archive
            .write_all(b"PBDEMS2\0\x10\0\0\0\0\0\0\0\x02\0\0")
            .expect("write");
        archive.finish().expect("finish");
        let destination = dir.path().join("demos");
        fs::create_dir(&destination).expect("dir");
        let original = fake_demo(&destination, "match.dem");
        let before = hash_file(&original).expect("hash");
        let imported = import(std::slice::from_ref(&archive_path), &destination).expect("import");
        assert_eq!(imported, [destination.join("match (1).dem")]);
        assert_eq!(hash_file(&original).expect("hash"), before);
        assert!(archive_path.exists());
    }

    #[test]
    fn windows_extended_paths_match_legacy_paths() {
        assert_eq!(
            path_key(Path::new(r"\\?\C:\Demos\match.dem")),
            path_key(Path::new(r"C:\Demos\match.dem"))
        );
        assert_eq!(
            path_key(Path::new(r"\\?\UNC\server\share\match.dem")),
            path_key(Path::new(r"\\server\share\match.dem"))
        );
    }
}
