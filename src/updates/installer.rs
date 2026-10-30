use super::{
    EXECUTABLE, MANIFEST, PackageFile, PreparedUpdate, file_sha, safe_path, validate_executable,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: u32,
    pub parent_pid: u32,
    pub root: PathBuf,
    pub executable: PathBuf,
    pub data_dir: PathBuf,
    pub startup_demo: Option<PathBuf>,
    pub version: String,
    pub previous_sha256: String,
    pub(crate) files: Vec<PackageFile>,
}

#[derive(Serialize, Deserialize)]
struct Started {
    pid: u32,
    version: String,
}

fn load(path: &Path) -> Result<Plan> {
    ensure!(
        path.metadata()?.len() <= 2 * 1024 * 1024,
        "Update plan is too large"
    );
    let plan: Plan = serde_json::from_reader(File::open(path)?)?;
    ensure!(
        plan.schema == 1
            && plan.root.is_absolute()
            && plan.executable.is_absolute()
            && plan.data_dir.is_absolute(),
        "Invalid update plan"
    );
    ensure!(
        path.canonicalize()? == plan.root.join("install.json").canonicalize()?,
        "Unexpected update plan path"
    );
    ensure!(
        !plan.files.is_empty() && plan.files.len() <= 4096,
        "Invalid update file count"
    );
    let mut seen = std::collections::HashSet::new();
    for file in &plan.files {
        safe_path(&file.path)?;
        ensure!(
            seen.insert(file.path.to_ascii_lowercase()),
            "Duplicate update destination"
        );
    }
    ensure!(
        plan.files.last().is_some_and(|f| f.path == EXECUTABLE)
            && plan.files.iter().any(|f| f.path == MANIFEST),
        "Invalid update executable ordering"
    );
    Ok(plan)
}

/// Called off the UI thread only after the library was successfully saved.
/// Keep staging alive only when the child has opened the real parent process.
pub fn handoff(
    prepared: PreparedUpdate,
    data_dir: PathBuf,
    startup_demo: Option<PathBuf>,
) -> Result<()> {
    ensure!(
        cfg!(windows),
        "Automatic installation is currently supported on Windows only"
    );
    let executable = std::env::current_exe()?.canonicalize()?;
    ensure!(
        executable
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case(EXECUTABLE)),
        "Rename the app to SmallDemoManager.exe before updating, or install the ZIP manually"
    );
    let install_dir = executable
        .parent()
        .context("App directory is unavailable")?;
    let probe = tempfile::NamedTempFile::new_in(install_dir).context("The app folder is not writable. Extract the release ZIP to a folder you own, or move the app there before updating")?;
    drop(probe);
    let root = prepared.directory.path().canonicalize()?;
    let plan = Plan {
        schema: 1,
        parent_pid: std::process::id(),
        root: root.clone(),
        executable: executable.clone(),
        data_dir: data_dir.canonicalize()?,
        startup_demo,
        version: prepared.version,
        previous_sha256: file_sha(&executable)?,
        files: prepared.files,
    };
    let helper = root.join("updater.exe");
    fs::copy(&executable, &helper)?;
    ensure!(
        file_sha(&helper)? == plan.previous_sha256,
        "Update helper copy did not verify"
    );
    crate::files::atomic_json(&root.join("install.json"), &plan)?;
    let mut child = Command::new(&helper)
        .arg("--apply-update")
        .arg(root.join("install.json"))
        .current_dir(&root)
        .spawn()
        .context("Could not start the update helper; the installed app was not changed")?;
    let start = Instant::now();
    loop {
        if root.join("helper-ready").is_file() {
            let _ = prepared.directory.keep();
            return Ok(());
        }
        if let Some(status) = child.try_wait()? {
            bail!(
                "Update helper exited before installation ({status}). The installed app was not changed"
            );
        }
        if start.elapsed() >= Duration::from_secs(15) {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Update helper did not become ready. The installed app was not changed");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn plain(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        !metadata.is_symlink(),
        "Refusing a symbolic link in the update destination"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "Refusing a Windows reparse point in the update destination"
        );
    }
    Ok(())
}

fn destination(root: &Path, name: &str) -> Result<PathBuf> {
    let relative = safe_path(name)?;
    let mut current = root.to_owned();
    for part in relative.components() {
        current.push(part);
        if current.exists() {
            plain(&current)?;
        }
    }
    Ok(current)
}

fn replace(source: &Path, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .context("Missing update destination parent")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut File::open(source)?, &mut temp)?;
    temp.as_file().sync_all()?;
    ensure!(
        file_sha(source)? == file_sha(temp.path())?,
        "Staged file copy did not verify"
    );
    temp.persist(target).map_err(|e| e.error)?;
    Ok(())
}

struct Changed {
    target: PathBuf,
    backup: Option<PathBuf>,
}

fn restore(changed: &[Changed]) -> Result<()> {
    let mut errors = Vec::new();
    for file in changed.iter().rev() {
        let result = if let Some(backup) = &file.backup {
            replace(backup, &file.target)
        } else {
            fs::remove_file(&file.target).map_err(Into::into)
        };
        if let Err(e) = result {
            errors.push(format!("{}: {e:#}", file.target.display()));
        }
    }
    ensure!(
        errors.is_empty(),
        "Rollback needs manual recovery from the update backup: {}",
        errors.join("; ")
    );
    Ok(())
}

fn transaction(plan: &Plan, launch: impl FnOnce() -> Result<()>) -> Result<()> {
    let install = plan
        .executable
        .parent()
        .context("Missing installation directory")?;
    ensure!(
        plan.executable
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case(EXECUTABLE)),
        "Unexpected installed executable name"
    );
    ensure!(
        file_sha(&plan.executable)? == plan.previous_sha256,
        "The installed app changed while the update was being prepared"
    );
    let payload = plan.root.join("payload");
    // Validate every source and destination before replacing even one file.
    for file in &plan.files {
        let source = destination(&payload, &file.path)?;
        ensure!(
            source.metadata()?.len() == file.size && file_sha(&source)? == file.sha256,
            "Prepared update file changed: {}",
            file.path
        );
        destination(install, &file.path)?;
    }
    validate_executable(&payload.join(EXECUTABLE))?;
    let mut changed = Vec::new();
    let applied = (|| -> Result<()> {
        for file in &plan.files {
            let target = destination(install, &file.path)?;
            let backup = if target.exists() {
                ensure!(
                    target.is_file(),
                    "Update destination is not a file: {}",
                    target.display()
                );
                let backup = plan.root.join("backup").join(&file.path);
                fs::create_dir_all(backup.parent().context("Missing backup parent")?)?;
                fs::copy(&target, &backup)?;
                File::options().write(true).open(&backup)?.sync_all()?;
                ensure!(
                    file_sha(&target)? == file_sha(&backup)?,
                    "Previous version backup did not verify"
                );
                Some(backup)
            } else {
                None
            };
            replace(&payload.join(&file.path), &target)
                .with_context(|| format!("Replace {}", target.display()))?;
            changed.push(Changed { target, backup });
        }
        launch()
    })();
    if let Err(error) = applied {
        restore(&changed).with_context(|| format!("Installation failed: {error:#}"))?;
        bail!("{error:#}. The previous app files were restored");
    }
    Ok(())
}

fn launch(plan: &Plan, confirming: bool) -> Result<Child> {
    let mut command = Command::new(&plan.executable);
    command
        .current_dir(plan.executable.parent().context("Missing app directory")?)
        .env("SDM_DATA_DIR", &plan.data_dir);
    if confirming {
        command
            .arg("--update-restart")
            .arg(plan.root.join("install.json"));
    } else if let Some(demo) = &plan.startup_demo {
        command.arg(demo);
    }
    Ok(command.spawn()?)
}

fn restart_and_confirm(plan: &Plan) -> Result<()> {
    let mut child = launch(plan, true)?;
    let start = Instant::now();
    let result = loop {
        if let Ok(file) = File::open(plan.root.join("started.json"))
            && let Ok(started) = serde_json::from_reader::<_, Started>(file)
            && started.pid == child.id()
            && started.version == plan.version
        {
            break Ok(());
        }
        if let Some(status) = child.try_wait()? {
            break Err(anyhow::anyhow!(
                "The updated app failed its startup check ({status})"
            ));
        }
        if start.elapsed() >= Duration::from_secs(30) {
            break Err(anyhow::anyhow!(
                "The updated app did not confirm a successful startup"
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if result.is_err() {
        // Only terminate the replacement process we just launched, never another instance.
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

/// Validate internal restart arguments before creating the GUI. The updater waits
/// for confirmation from the real app after its window and library have opened.
pub fn restart_context(path: &Path) -> Result<(Plan, Option<PathBuf>)> {
    let plan = load(path)?;
    ensure!(
        std::env::current_exe()?.canonicalize()? == plan.executable.canonicalize()?,
        "Update restart targets another executable"
    );
    ensure!(
        plan.version == env!("CARGO_PKG_VERSION"),
        "Release version does not match the installed executable"
    );
    let expected = plan
        .files
        .iter()
        .find(|f| f.path == EXECUTABLE)
        .context("Missing updated executable")?;
    ensure!(
        file_sha(&plan.executable)? == expected.sha256,
        "Updated executable verification failed"
    );
    let demo = plan.startup_demo.clone();
    Ok((plan, demo))
}

pub fn acknowledge(plan: &Plan) -> Result<()> {
    crate::files::atomic_json(
        &plan.root.join("started.json"),
        &Started {
            pid: std::process::id(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
    )
}

#[cfg(windows)]
pub fn run(path: &Path) -> Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
    };
    let plan = load(path)?;
    ensure!(
        std::env::current_exe()?.canonicalize()? == plan.root.join("updater.exe").canonicalize()?
            && file_sha(&std::env::current_exe()?)? == plan.previous_sha256,
        "Updater helper does not match the approved plan"
    );
    let install = plan.executable.parent().context("Missing app directory")?;
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(install.join(".sdm-update.lock"))?;
    lock.try_lock()
        .context("Another update is already running for this installation")?;
    struct Handle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    let parent = Handle(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, plan.parent_pid) });
    ensure!(
        !parent.0.is_null(),
        "Could not open the app process; no files were changed"
    );
    fs::write(plan.root.join("helper-ready"), b"ready")?;
    let wait = unsafe { WaitForSingleObject(parent.0, 120_000) };
    ensure!(
        wait == WAIT_OBJECT_0,
        "The app did not close within two minutes; no update was installed"
    );
    let result = transaction(&plan, || restart_and_confirm(&plan));
    if let Err(error) = &result {
        let message = format!(
            "Update could not be installed: {error:#}\n\nBackup and details: {}",
            plan.root.display()
        );
        let _ = fs::write(plan.root.join("error.txt"), &message);
        if file_sha(&plan.executable).ok().as_deref() == Some(&plan.previous_sha256) {
            let _ = launch(&plan, false);
        }
        rfd::MessageDialog::new()
            .set_title("Small Demo Manager update failed")
            .set_description(&message)
            .set_level(rfd::MessageLevel::Error)
            .show();
    } else {
        fs::write(
            plan.root.join("installed.txt"),
            format!(
                "Installed {}. Previous app files are in backup/. User data was not replaced.\n",
                plan.version
            ),
        )?;
    }
    result
}

#[cfg(not(windows))]
pub fn run(_path: &Path) -> Result<()> {
    bail!("The update installer is supported on Windows only")
}

#[cfg(test)]
mod tests;
