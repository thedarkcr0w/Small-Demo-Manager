use super::*;
use std::io::Cursor;
use zip::write::SimpleFileOptions;

pub(super) fn pe() -> Vec<u8> {
    // Minimal structural PE fixture, never executed or shipped as an app.
    let mut bytes = vec![0; 1024];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&128_u32.to_le_bytes());
    bytes[128..132].copy_from_slice(b"PE\0\0");
    bytes[132..134].copy_from_slice(&0x8664_u16.to_le_bytes());
    bytes[150..152].copy_from_slice(&0x22_u16.to_le_bytes());
    bytes[152..154].copy_from_slice(&0x20b_u16.to_le_bytes());
    bytes
}

fn release(version: &str) -> Release {
    let base = format!("{RELEASES}/download/v{version}");
    Release {
        tag_name: format!("v{version}"),
        html_url: format!("{RELEASES}/tag/v{version}"),
        draft: false,
        prerelease: false,
        assets: vec![
            Asset {
                name: ARCHIVE.into(),
                browser_download_url: format!("{base}/{ARCHIVE}"),
                size: 100,
                digest: None,
            },
            Asset {
                name: format!("{ARCHIVE}.sha256"),
                browser_download_url: format!("{base}/{ARCHIVE}.sha256"),
                size: 110,
                digest: None,
            },
        ],
    }
}

fn contents(version: &str) -> Vec<(String, Vec<u8>)> {
    let binary = pe();
    let manifest = Manifest {
        schema: 1,
        application: "small-demo-manager".into(),
        target: "windows-x64".into(),
        version: version.into(),
        executable_sha256: sha(&binary),
    };
    vec![
        (EXECUTABLE.into(), binary),
        (
            MANIFEST.into(),
            serde_json::to_vec(&manifest).expect("manifest"),
        ),
        ("LICENSE".into(), b"fixture notice".to_vec()),
        ("THIRD-PARTY-NOTICES.md".into(), b"fixture notice".to_vec()),
        (
            "LICENSES/Small-Demo-Manager-original-MIT.txt".into(),
            b"fixture notice".to_vec(),
        ),
        (
            "LICENSES/dependencies/INDEX.md".into(),
            b"fixture index".to_vec(),
        ),
    ]
}

fn archive(path: &Path, entries: &[(String, Vec<u8>)]) {
    let mut zip = zip::ZipWriter::new(File::create(path).expect("archive"));
    for (name, bytes) in entries {
        zip.start_file(name, SimpleFileOptions::default())
            .expect("member");
        zip.write_all(bytes).expect("write");
    }
    zip.finish().expect("finish");
}

#[test]
fn selects_highest_stable_verified_package_not_first_api_result() {
    let mut preview = release("9.0.0");
    preview.prerelease = true;
    let mut draft = release("8.0.0");
    draft.draft = true;
    let result = select_release(
        vec![
            release("2.1.0"),
            preview,
            draft,
            release("3.0.0"),
            release("2.2.0"),
        ],
        &semver::Version::parse("2.0.0").expect("version"),
    )
    .expect("release");
    assert_eq!(result.version, "3.0.0");
    assert!(result.newer);
    assert!(
        !select_release(
            vec![release("2.0.0")],
            &semver::Version::parse("3.0.0").expect("version")
        )
        .expect("release")
        .newer
    );
    assert!(
        select_release(
            vec![release("2.0.0")],
            &semver::Version::parse("2.0.0-dev").expect("version")
        )
        .expect("release")
        .newer
    );
}

#[test]
fn legacy_missing_checksum_prerelease_and_foreign_downloads_are_not_offered() {
    let mut old = release("1.0.18");
    old.assets[0].name = "SmallDemoManager-shipping.zip".into();
    let mut incomplete = release("3.0.0");
    incomplete.assets.pop();
    let mut foreign = release("4.0.0");
    foreign.assets[0].browser_download_url = "https://example.com/app.zip".into();
    let preview = release("5.0.0-beta.1");
    assert!(
        select_release(
            vec![old, incomplete, foreign, preview],
            &semver::Version::new(2, 0, 0)
        )
        .is_err()
    );
}

#[test]
fn updater_urls_are_pinned_to_this_repository_and_https_github_hosts() {
    assert!(release_url(&format!("{RELEASES}/download/v2.0.0/{ARCHIVE}"), true).is_ok());
    for url in [
        "http://github.com/thedarkcr0w/Small-Demo-Manager/releases/download/v2/a.zip",
        "https://github.com/other/project/releases/download/v2/a.zip",
        "https://github.com.evil.example/thedarkcr0w/Small-Demo-Manager/releases/download/v2/a.zip",
        "https://github.com@evil.example/a.zip",
    ] {
        assert!(release_url(url, true).is_err(), "{url}");
    }
    assert!(trusted_host(
        &Url::parse(
            "https://release-assets.githubusercontent.com/github-production-release-asset/fixture"
        )
        .expect("URL")
    ));
    assert!(!trusted_host(
        &Url::parse("https://example.com/payload").expect("URL")
    ));
}

#[test]
fn checksums_are_validated_and_bound_to_the_expected_archive() {
    let hash = sha(b"real bytes");
    assert_eq!(
        checksum(format!("{}  {ARCHIVE}\r\n", hash.to_uppercase()).as_bytes()).expect("checksum"),
        hash
    );
    for text in [
        format!("{hash}  other.zip"),
        format!("{hash}\n{hash}"),
        "not a checksum".into(),
        "0".repeat(63),
    ] {
        assert!(checksum(text.as_bytes()).is_err());
    }
    let mut asset = release("2.0.0").assets.remove(0);
    asset.digest = Some(format!("sha256:{hash}"));
    assert!(api_digest(&asset, &hash).is_ok());
    assert!(api_digest(&asset, &sha(b"wrong")).is_err());
}

#[test]
fn streamed_downloads_hash_real_bytes_and_reject_short_oversized_or_cancelled_data() {
    let bytes = b"downloaded package bytes";
    let mut output = Vec::new();
    assert_eq!(
        stream(
            &mut Cursor::new(bytes),
            &mut output,
            bytes.len() as u64,
            1024,
            &AtomicBool::new(false),
            &|_, _| {}
        )
        .expect("stream"),
        sha(bytes)
    );
    assert_eq!(output, bytes);
    assert!(
        stream(
            &mut Cursor::new(bytes),
            &mut Vec::new(),
            100,
            1024,
            &AtomicBool::new(false),
            &|_, _| {}
        )
        .is_err()
    );
    assert!(
        stream(
            &mut Cursor::new(bytes),
            &mut Vec::new(),
            1,
            1024,
            &AtomicBool::new(false),
            &|_, _| {}
        )
        .is_err()
    );
    let mut output = Vec::new();
    assert!(
        stream(
            &mut Cursor::new(bytes),
            &mut output,
            bytes.len() as u64,
            1024,
            &AtomicBool::new(true),
            &|_, _| {}
        )
        .is_err()
    );
    assert!(output.is_empty());
}

#[test]
fn valid_update_package_verifies_manifest_executable_and_notices() {
    let dir = tempfile::tempdir().expect("dir");
    let zip = dir.path().join("update.zip");
    archive(&zip, &contents("2.1.0"));
    let files = unpack(
        &zip,
        &dir.path().join("payload"),
        "2.1.0",
        &AtomicBool::new(false),
    )
    .expect("package");
    assert_eq!(files.last().expect("last").path, EXECUTABLE);
    assert!(
        files
            .iter()
            .any(|f| f.path == "LICENSES/dependencies/INDEX.md")
    );
}

#[test]
fn windows_zip_separators_are_normalized_before_security_checks() {
    let dir = tempfile::tempdir().expect("dir");
    let zip = dir.path().join("update.zip");
    let entries: Vec<_> = contents("2.1.0")
        .into_iter()
        .map(|(name, bytes)| (name.replace('/', "\\"), bytes))
        .collect();
    archive(&zip, &entries);
    let files = unpack(
        &zip,
        &dir.path().join("payload"),
        "2.1.0",
        &AtomicBool::new(false),
    )
    .expect("Windows package");
    assert!(
        files
            .iter()
            .any(|f| f.path == "LICENSES/dependencies/INDEX.md")
    );
    let mut entries = contents("2.1.0");
    entries.push(("docs\\..\\..\\escape.txt".into(), b"unsafe".to_vec()));
    archive(&zip, &entries);
    assert!(
        unpack(
            &zip,
            &dir.path().join("unsafe"),
            "2.1.0",
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(!dir.path().join("escape.txt").exists());
}

#[test]
fn case_insensitive_duplicates_and_symlinks_are_rejected() {
    let dir = tempfile::tempdir().expect("dir");
    let zip = dir.path().join("update.zip");
    let mut entries = contents("2.1.0");
    entries.push(("LICENSES/notice.txt".into(), b"one".to_vec()));
    entries.push(("LICENSES/NOTICE.TXT".into(), b"two".to_vec()));
    archive(&zip, &entries);
    assert!(
        unpack(
            &zip,
            &dir.path().join("duplicate"),
            "2.1.0",
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let mut writer = zip::ZipWriter::new(File::create(&zip).expect("archive"));
    writer
        .add_symlink("docs/link", "../../outside", SimpleFileOptions::default())
        .expect("link");
    writer.finish().expect("finish");
    assert!(
        unpack(
            &zip,
            &dir.path().join("symlink"),
            "2.1.0",
            &AtomicBool::new(false)
        )
        .is_err()
    );
}

#[test]
#[ignore = "Set SDM_UPDATE_PACKAGE to the ZIP produced by scripts/package.ps1"]
fn packaged_release_passes_the_actual_updater_validation() {
    let zip = PathBuf::from(std::env::var_os("SDM_UPDATE_PACKAGE").expect("SDM_UPDATE_PACKAGE"));
    let sidecar = fs::read(zip.with_extension("zip.sha256")).expect("sidecar");
    assert_eq!(
        file_sha(&zip).expect("archive hash"),
        checksum(&sidecar).expect("checksum")
    );
    let dir = tempfile::tempdir().expect("dir");
    let files = unpack(
        &zip,
        &dir.path().join("payload"),
        env!("CARGO_PKG_VERSION"),
        &AtomicBool::new(false),
    )
    .expect("real package validation");
    let mut actual: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
    actual.sort_unstable();
    let mut expected = vec![
        EXECUTABLE,
        MANIFEST,
        "LICENSE",
        "THIRD-PARTY-NOTICES.md",
        "LICENSES/Small-Demo-Manager-original-MIT.txt",
        "LICENSES/dependencies/INDEX.md",
    ];
    expected.sort_unstable();
    assert_eq!(
        actual, expected,
        "release ZIP must not contain documentation or screenshots"
    );
    assert_eq!(files.last().expect("last").path, EXECUTABLE);
}

#[test]
#[ignore = "Read-only network integration test against the official GitHub releases API"]
fn live_official_release_check_never_offers_legacy_packages() {
    match check() {
        Ok(update) => {
            assert_eq!(update.archive.name, ARCHIVE);
            assert_eq!(update.checksum.name, format!("{ARCHIVE}.sha256"));
            release_url(&update.url, false).expect("official release");
            println!("Compatible release: {}", update.version);
        }
        Err(error) => {
            let message = format!("{error:#}");
            assert!(message.contains("No compatible app release"), "{message}");
            println!("{message}");
        }
    }
}

#[test]
fn update_paths_cannot_escape_or_replace_user_data() {
    for name in [
        "../escape.exe",
        "/absolute",
        "docs/../../escape",
        "docs\\escape.exe",
        "docs/a:stream",
        "docs/CON.txt",
        "docs/file. ",
        "Library.native.json",
        "Audio.native/clip.wav",
        "Backups.native/match.dem",
        "other.dll",
    ] {
        assert!(safe_path(name).is_err(), "{name}");
    }
    let dir = tempfile::tempdir().expect("dir");
    let zip = dir.path().join("update.zip");
    let mut entries = contents("2.1.0");
    entries.push(("../escape.exe".into(), b"bad".to_vec()));
    archive(&zip, &entries);
    assert!(
        unpack(
            &zip,
            &dir.path().join("payload"),
            "2.1.0",
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(!dir.path().join("escape.exe").exists());
}

#[test]
fn legacy_wrong_version_and_tampered_binary_packages_fail_closed() {
    for variant in 0..4 {
        let dir = tempfile::tempdir().expect("dir");
        let zip = dir.path().join("update.zip");
        let mut entries = contents("2.1.0");
        match variant {
            0 => entries.retain(|(name, _)| name != MANIFEST),
            1 => {
                entries[0].1[600] ^= 1;
            }
            2 => {
                entries.push(("license".into(), b"duplicate".to_vec()));
            }
            _ => {}
        }
        archive(&zip, &entries);
        let version = if variant == 3 { "9.0.0" } else { "2.1.0" };
        assert!(
            unpack(
                &zip,
                &dir.path().join("payload"),
                version,
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}

#[test]
fn executable_validation_rejects_wrong_architecture_and_corrupt_headers() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join(EXECUTABLE);
    let mut bytes = pe();
    fs::write(&path, &bytes).expect("write");
    validate_executable(&path).expect("PE");
    bytes[132..134].copy_from_slice(&0x14c_u16.to_le_bytes());
    fs::write(&path, &bytes).expect("write");
    assert!(validate_executable(&path).is_err());
    bytes[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
    fs::write(&path, bytes).expect("write");
    assert!(validate_executable(&path).is_err());
}
