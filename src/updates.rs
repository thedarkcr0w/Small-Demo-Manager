pub mod installer;

use anyhow::{Context, Result, ensure};
use reqwest::{Url, blocking::Client};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pub const RELEASES: &str = "https://github.com/thedarkcr0w/Small-Demo-Manager/releases";
pub const ARCHIVE: &str = "SmallDemoManager-native-windows-x64.zip";
pub const EXECUTABLE: &str = "SmallDemoManager.exe";
pub const MANIFEST: &str = "update-manifest.json";
const API: &str = "https://api.github.com/repos/thedarkcr0w/Small-Demo-Manager/releases";
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;
const MAX_UNPACKED: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Update {
    pub version: String,
    pub url: String,
    pub newer: bool,
    archive: Asset,
    checksum: Asset,
}

#[derive(Clone, Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Clone, Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub application: String,
    pub target: String,
    pub version: String,
    pub executable_sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

pub struct PreparedUpdate {
    pub version: String,
    pub(crate) directory: tempfile::TempDir,
    pub(crate) files: Vec<PackageFile>,
}

pub enum Event {
    Progress {
        message: String,
        fraction: Option<f32>,
    },
    Checked(Result<Update, String>),
    Prepared(Result<PreparedUpdate, String>),
    Handoff(Result<(), String>),
}

pub struct Task {
    pub events: mpsc::Receiver<Event>,
    pub cancel: Arc<AtomicBool>,
}

impl Task {
    /// Separate from demo parsing: a slow network must not stall the library.
    pub fn start(
        repaint: impl Fn() + Send + 'static,
        work: impl FnOnce(&AtomicBool, &dyn Fn(String, Option<f32>)) -> Event + Send + 'static,
    ) -> Self {
        let (tx, events) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancelled = cancel.clone();
        std::thread::spawn(move || {
            let progress = |message, fraction| {
                let _ = tx.send(Event::Progress { message, fraction });
                repaint();
            };
            let event = work(&cancelled, &progress);
            let _ = tx.send(event);
            repaint();
        });
        Self { events, cancel }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn client(timeout: Duration) -> Result<Client> {
    Ok(Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        // The blocking client's timeout also bounds a stalled read/cancel.
        .timeout(timeout)
        .user_agent(concat!("SmallDemoManager/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || !trusted_host(attempt.url()) {
                attempt.error("Update redirect left the trusted GitHub download hosts")
            } else {
                attempt.follow()
            }
        }))
        .build()?)
}

fn trusted_host(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "api.github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
                    | "github-releases.githubusercontent.com"
            )
        )
}

fn release_url(url: &str, download: bool) -> Result<()> {
    let url = Url::parse(url)?;
    let prefix = if download {
        "/thedarkcr0w/Small-Demo-Manager/releases/download/"
    } else {
        "/thedarkcr0w/Small-Demo-Manager/releases/tag/"
    };
    ensure!(
        trusted_host(&url)
            && url.host_str() == Some("github.com")
            && url.path().starts_with(prefix)
            && url.query().is_none()
            && url.fragment().is_none(),
        "Unexpected update URL"
    );
    Ok(())
}

fn select_release(releases: Vec<Release>, local: &semver::Version) -> Result<Update> {
    let mut candidates = Vec::new();
    for release in releases {
        if release.draft || release.prerelease {
            continue;
        }
        let Ok(version) = semver::Version::parse(release.tag_name.trim_start_matches('v')) else {
            continue;
        };
        if !version.pre.is_empty() {
            continue;
        }
        let Some(archive) = release.assets.iter().find(|a| a.name == ARCHIVE) else {
            continue;
        };
        let Some(checksum) = release
            .assets
            .iter()
            .find(|a| a.name == format!("{ARCHIVE}.sha256"))
        else {
            continue;
        };
        if archive.size == 0
            || archive.size > MAX_DOWNLOAD
            || checksum.size == 0
            || checksum.size > 4096
        {
            continue;
        }
        if release_url(&release.html_url, false).is_err()
            || release_url(&archive.browser_download_url, true).is_err()
            || release_url(&checksum.browser_download_url, true).is_err()
        {
            continue;
        }
        // Both assets must belong to this same versioned release download path.
        let archive_url = Url::parse(&archive.browser_download_url)?;
        let checksum_url = Url::parse(&checksum.browser_download_url)?;
        let parent = |url: &Url| url.path().rsplit_once('/').map(|(path, _)| path.to_owned());
        if parent(&archive_url) != parent(&checksum_url) {
            continue;
        }
        candidates.push((version, release.html_url, archive.clone(), checksum.clone()));
    }
    let (version, url, archive, checksum) = candidates.into_iter().max_by(|a, b| a.0.cmp(&b.0))
        .context("No compatible app release with a ZIP and SHA-256 checksum has been published yet. Older .NET releases cannot be installed by this updater")?;
    Ok(Update {
        newer: version > *local,
        version: version.to_string(),
        url,
        archive,
        checksum,
    })
}

pub fn check() -> Result<Update> {
    let client = client(Duration::from_secs(20))?;
    let response = client
        .get(format!("{API}?per_page=100"))
        .header("Accept", "application/vnd.github+json")
        .send()?
        .error_for_status()?;
    let mut bytes = Vec::new();
    response.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 4 * 1024 * 1024,
        "Release metadata exceeds the safety limit"
    );
    select_release(
        serde_json::from_slice(&bytes)?,
        &semver::Version::parse(env!("CARGO_PKG_VERSION"))?,
    )
}

fn cancelled(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Update download cancelled; the installed app was not changed"
    );
    Ok(())
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn file_sha(path: &Path) -> Result<String> {
    Ok(crate::files::hash_file(path)?
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn checksum(text: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(text)?
        .trim_start_matches('\u{feff}')
        .trim();
    let fields: Vec<_> = text.split_whitespace().collect();
    ensure!(
        !fields.is_empty() && fields.len() <= 2 && !text.contains(['\n', '\r']),
        "Invalid SHA-256 sidecar"
    );
    let hash = fields[0];
    ensure!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid SHA-256 hash"
    );
    if fields.len() == 2 {
        ensure!(
            fields[1].trim_start_matches('*') == ARCHIVE,
            "Checksum names a different archive"
        );
    }
    Ok(hash.to_ascii_lowercase())
}

fn api_digest(asset: &Asset, actual: &str) -> Result<()> {
    if let Some(digest) = &asset.digest {
        ensure!(
            digest.eq_ignore_ascii_case(&format!("sha256:{actual}")),
            "{} does not match GitHub's asset digest",
            asset.name
        );
    }
    Ok(())
}

fn stream(
    source: &mut impl Read,
    target: &mut impl Write,
    size: u64,
    limit: u64,
    cancel: &AtomicBool,
    progress: &dyn Fn(String, Option<f32>),
) -> Result<String> {
    ensure!(
        size > 0 && size <= limit,
        "Update download exceeds the safety limit"
    );
    let mut total = 0_u64;
    let mut hash = Sha256::new();
    let mut buffer = [0; 128 * 1024];
    loop {
        cancelled(cancel)?;
        let n = source.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        ensure!(
            total <= size && total <= limit,
            "Update download is larger than advertised"
        );
        target.write_all(&buffer[..n])?;
        hash.update(&buffer[..n]);
        progress(
            format!(
                "Downloading update · {:.1} / {:.1} MB",
                total as f64 / 1048576.0,
                size as f64 / 1048576.0
            ),
            Some(total as f32 / size as f32),
        );
    }
    ensure!(
        total == size,
        "Incomplete update download: expected {size} bytes, received {total}"
    );
    Ok(format!("{:x}", hash.finalize()))
}

pub fn download(
    update: &Update,
    data_dir: &Path,
    cancel: &AtomicBool,
    progress: &dyn Fn(String, Option<f32>),
) -> Result<PreparedUpdate> {
    ensure!(
        update.newer,
        "This release is not newer than the installed app"
    );
    cancelled(cancel)?;
    release_url(&update.archive.browser_download_url, true)?;
    release_url(&update.checksum.browser_download_url, true)?;
    let client = client(Duration::from_secs(30))?;
    progress("Reading release checksum…".into(), None);
    let response = client
        .get(&update.checksum.browser_download_url)
        .send()?
        .error_for_status()?;
    let mut sidecar = Vec::new();
    response.take(4097).read_to_end(&mut sidecar)?;
    ensure!(
        sidecar.len() <= 4096 && sidecar.len() as u64 == update.checksum.size,
        "Invalid checksum download size"
    );
    api_digest(&update.checksum, &sha(&sidecar))?;
    let expected = checksum(&sidecar)?;
    api_digest(&update.archive, &expected)?;
    cancelled(cancel)?;
    let updates = data_dir.join("Updates");
    fs::create_dir_all(&updates)?;
    let directory = tempfile::Builder::new()
        .prefix("update-")
        .tempdir_in(updates)?;
    let archive = directory.path().join("package.zip");
    let mut file = File::create(&archive)?;
    let mut response = client
        .get(&update.archive.browser_download_url)
        .send()?
        .error_for_status()?;
    if let Some(length) = response.content_length() {
        ensure!(
            length == update.archive.size,
            "Unexpected archive content length"
        );
    }
    let actual = stream(
        &mut response,
        &mut file,
        update.archive.size,
        MAX_DOWNLOAD,
        cancel,
        progress,
    )?;
    file.sync_all()?;
    ensure!(
        actual == expected,
        "Update SHA-256 verification failed; the installed app was not changed"
    );
    drop(file);
    progress("Checksum verified · checking update package…".into(), None);
    let files = unpack(
        &archive,
        &directory.path().join("payload"),
        &update.version,
        cancel,
    )?;
    Ok(PreparedUpdate {
        version: update.version.clone(),
        directory,
        files,
    })
}

pub(crate) fn safe_path(name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty() && !name.contains(['\\', ':']) && !name.chars().any(char::is_control),
        "Unsafe update member path"
    );
    for component in name.split('/') {
        ensure!(
            !component.is_empty()
                && component != "."
                && component != ".."
                && !component.ends_with(['.', ' '])
                && !component.contains(['<', '>', '"', '|', '?', '*']),
            "Unsafe update path component"
        );
        let stem = component
            .split('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        ensure!(
            !matches!(
                stem.as_str(),
                "CON"
                    | "PRN"
                    | "AUX"
                    | "NUL"
                    | "COM1"
                    | "COM2"
                    | "COM3"
                    | "COM4"
                    | "COM5"
                    | "COM6"
                    | "COM7"
                    | "COM8"
                    | "COM9"
                    | "LPT1"
                    | "LPT2"
                    | "LPT3"
                    | "LPT4"
                    | "LPT5"
                    | "LPT6"
                    | "LPT7"
                    | "LPT8"
                    | "LPT9"
            ),
            "Reserved Windows update filename"
        );
    }
    ensure!(
        matches!(
            name,
            EXECUTABLE
                | MANIFEST
                | "README.md"
                | "PATCHNOTES.md"
                | "showcase.png"
                | "LICENSE"
                | "THIRD-PARTY-NOTICES.md"
        ) || name.starts_with("LICENSES/")
            || name.starts_with("docs/"),
        "Unexpected update member: {name}"
    );
    Ok(PathBuf::from(name))
}

pub(crate) fn validate_executable(path: &Path) -> Result<()> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    ensure!(
        (512..=MAX_DOWNLOAD).contains(&size),
        "Invalid Windows executable size"
    );
    let mut dos = [0; 64];
    file.read_exact(&mut dos)?;
    ensure!(&dos[..2] == b"MZ", "Update is not a Windows executable");
    let offset = u32::from_le_bytes(dos[60..64].try_into()?);
    ensure!(
        (64..=1024 * 1024).contains(&offset) && u64::from(offset) + 26 <= size,
        "Invalid PE header offset"
    );
    file.seek(SeekFrom::Start(offset.into()))?;
    let mut pe = [0; 26];
    file.read_exact(&mut pe)?;
    ensure!(
        &pe[..4] == b"PE\0\0"
            && u16::from_le_bytes(pe[4..6].try_into()?) == 0x8664
            && u16::from_le_bytes(pe[24..26].try_into()?) == 0x20b
            && u16::from_le_bytes(pe[22..24].try_into()?) & 0x2000 == 0,
        "Update must be a Windows x64 application, not a DLL or another architecture"
    );
    Ok(())
}

fn unpack(
    archive: &Path,
    destination: &Path,
    version: &str,
    cancel: &AtomicBool,
) -> Result<Vec<PackageFile>> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    ensure!(zip.len() <= 4096, "Update ZIP contains too many files");
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    let mut total = 0_u64;
    fs::create_dir(destination)?;
    for index in 0..zip.len() {
        cancelled(cancel)?;
        let mut entry = zip.by_index(index)?;
        // Windows Compress-Archive emits backslash separators. Normalize first,
        // then apply the same traversal/ADS/duplicate checks to every platform.
        let normalized = entry.name().replace('\\', "/");
        let is_directory = entry.is_dir() || normalized.ends_with('/');
        let name = normalized.trim_end_matches('/').to_owned();
        ensure!(
            entry
                .unix_mode()
                .is_none_or(|mode| mode & 0o170000 != 0o120000),
            "Update ZIP contains a symbolic link"
        );
        // Empty directory records are harmless but still validate their namespace.
        if is_directory && matches!(name.as_str(), "docs" | "LICENSES") {
            continue;
        }
        let path = safe_path(&name)?;
        ensure!(
            seen.insert(name.to_ascii_lowercase()),
            "Duplicate update member: {name}"
        );
        let target = destination.join(path);
        if is_directory {
            fs::create_dir_all(target)?;
            continue;
        }
        total = total
            .checked_add(entry.size())
            .context("Update ZIP size overflow")?;
        ensure!(
            total <= MAX_UNPACKED && entry.size() <= MAX_DOWNLOAD,
            "Update ZIP exceeds the unpacked safety limit"
        );
        fs::create_dir_all(target.parent().context("Missing update member parent")?)?;
        let mut file = File::options().write(true).create_new(true).open(&target)?;
        let declared = entry.size();
        let copied = std::io::copy(
            &mut (&mut entry).take(declared.saturating_add(1)),
            &mut file,
        )?;
        ensure!(copied == declared, "Update ZIP member has an invalid size");
        file.sync_all()?;
        files.push(PackageFile {
            path: name,
            size: copied,
            sha256: file_sha(&target)?,
        });
    }
    for required in [
        EXECUTABLE,
        MANIFEST,
        "LICENSE",
        "THIRD-PARTY-NOTICES.md",
        "LICENSES/Small-Demo-Manager-original-MIT.txt",
        "LICENSES/dependencies/INDEX.md",
    ] {
        ensure!(
            files.iter().any(|f| f.path == required),
            "Update package is missing {required}"
        );
    }
    let manifest_file = destination.join(MANIFEST);
    ensure!(
        manifest_file.metadata()?.len() <= 4096,
        "Update manifest is too large"
    );
    let manifest: Manifest = serde_json::from_reader(File::open(manifest_file)?)?;
    ensure!(
        manifest.schema == 1
            && manifest.application == "small-demo-manager"
            && manifest.target == "windows-x64"
            && manifest.version == version,
        "Update manifest does not match this app, platform, or release version"
    );
    let exe = destination.join(EXECUTABLE);
    ensure!(
        file_sha(&exe)?.eq_ignore_ascii_case(&manifest.executable_sha256),
        "Update executable does not match the package manifest"
    );
    validate_executable(&exe)?;
    // Install executable last, once accompanying notices/docs are in place.
    files.sort_by_key(|f| f.path == EXECUTABLE);
    Ok(files)
}

#[cfg(test)]
mod tests;
