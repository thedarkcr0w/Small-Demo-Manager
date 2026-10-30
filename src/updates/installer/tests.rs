use super::*;

fn fixture() -> (tempfile::TempDir, Plan) {
    let temp = tempfile::tempdir().expect("dir");
    let install = temp.path().join("app");
    let root = temp.path().join("update");
    let data = temp.path().join("data");
    fs::create_dir_all(&install).expect("app");
    fs::create_dir_all(root.join("payload")).expect("payload");
    fs::create_dir_all(&data).expect("data");
    fs::write(install.join(EXECUTABLE), b"previous app bytes").expect("old app");
    fs::write(install.join("README.md"), b"old readme").expect("old docs");
    fs::write(install.join("my-private-notes.txt"), b"untouched").expect("private");
    fs::write(
        data.join("Library.native.json"),
        b"user library must stay intact",
    )
    .expect("data");
    let payload = root.join("payload");
    let entries = [
        ("README.md", b"new readme".to_vec()),
        (MANIFEST, b"fixture manifest".to_vec()),
        (EXECUTABLE, crate::updates::tests::pe()),
    ];
    let files = entries
        .into_iter()
        .map(|(name, bytes)| {
            let path = payload.join(name);
            fs::write(&path, &bytes).expect("payload");
            PackageFile {
                path: name.into(),
                sha256: file_sha(&path).expect("hash"),
                size: bytes.len() as u64,
            }
        })
        .collect();
    let executable = install.join(EXECUTABLE);
    let plan = Plan {
        schema: 1,
        parent_pid: std::process::id(),
        root,
        data_dir: data,
        startup_demo: None,
        version: "2.1.0".into(),
        previous_sha256: file_sha(&executable).expect("hash"),
        executable,
        files,
    };
    (temp, plan)
}

#[test]
fn successful_install_keeps_verified_backup_and_preserves_unmanaged_files_and_data() {
    let (_temp, plan) = fixture();
    transaction(&plan, || Ok(())).expect("install");
    assert_eq!(
        fs::read(plan.root.join("backup").join(EXECUTABLE)).expect("backup"),
        b"previous app bytes"
    );
    assert_eq!(
        file_sha(&plan.executable).expect("installed"),
        plan.files.last().expect("exe").sha256
    );
    assert_eq!(
        fs::read(plan.executable.with_file_name("my-private-notes.txt")).expect("private"),
        b"untouched"
    );
    assert_eq!(
        fs::read(plan.data_dir.join("Library.native.json")).expect("data"),
        b"user library must stay intact"
    );
}

#[test]
fn failed_replacement_startup_restores_previous_executable_and_docs() {
    let (_temp, plan) = fixture();
    let result = transaction(&plan, || bail!("Simulated replacement startup failure"));
    assert!(result.is_err());
    assert_eq!(
        fs::read(&plan.executable).expect("restored app"),
        b"previous app bytes"
    );
    assert_eq!(
        fs::read(plan.executable.with_file_name("README.md")).expect("restored docs"),
        b"old readme"
    );
    assert!(!plan.executable.with_file_name(MANIFEST).exists());
    assert_eq!(
        fs::read(plan.data_dir.join("Library.native.json")).expect("data"),
        b"user library must stay intact"
    );
}

#[test]
fn tampered_payload_is_rejected_before_any_installed_file_is_modified() {
    let (_temp, plan) = fixture();
    fs::write(plan.root.join("payload").join(EXECUTABLE), b"tampered").expect("tamper");
    assert!(transaction(&plan, || Ok(())).is_err());
    assert_eq!(
        fs::read(plan.executable.with_file_name("README.md")).expect("unchanged docs"),
        b"old readme"
    );
    assert_eq!(
        file_sha(&plan.executable).expect("old hash"),
        plan.previous_sha256
    );
    assert!(!plan.root.join("backup").exists());
}

#[test]
fn installation_failure_rolls_back_files_already_replaced() {
    let (_temp, mut plan) = fixture();
    // A directory where a managed file should be causes a deterministic I/O
    // failure after README was replaced, without relying on host permissions.
    fs::create_dir(plan.executable.with_file_name(MANIFEST)).expect("conflict");
    assert!(transaction(&plan, || Ok(())).is_err());
    assert_eq!(
        fs::read(plan.executable.with_file_name("README.md")).expect("restored"),
        b"old readme"
    );
    assert_eq!(
        file_sha(&plan.executable).expect("old hash"),
        plan.previous_sha256
    );
    // A concurrently replaced installed binary must also never be overwritten.
    plan.previous_sha256 = "different".into();
    assert!(transaction(&plan, || Ok(())).is_err());
}
