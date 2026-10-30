use crate::{files, model::*};
use anyhow::{Context, Result, ensure};
use std::{
    collections::HashSet,
    fs::{self, File},
    path::{Path, PathBuf},
};

pub struct Store {
    pub root: PathBuf,
    pub library: Library,
    _lock: File,
}

impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("native.lock"))?;
        lock.try_lock()
            .context("The native library is already open in another instance")?;
        let path = root.join("Library.native.json");
        let library = if path.exists() {
            let library: Library =
                serde_json::from_reader(File::open(&path)?).with_context(|| {
                    format!(
                        "Cannot read {}. Original data has NOT been replaced",
                        path.display()
                    )
                })?;
            ensure!(
                library.schema == 1,
                "Unsupported library schema {}; original data has NOT been replaced",
                library.schema
            );
            library
        } else {
            migrate_legacy(&root)?
        };
        Ok(Self {
            root,
            library,
            _lock: lock,
        })
    }

    pub fn save(&self) -> Result<()> {
        files::atomic_json(&self.root.join("Library.native.json"), &self.library)
    }

    pub fn add_folder(&mut self, path: PathBuf) -> Result<String> {
        let path = path
            .canonicalize()
            .context("Watched folder is not available")?;
        ensure!(path.is_dir(), "Not a directory");
        let key = files::path_key(&path);
        if let Some(folder) = self
            .library
            .folders
            .iter()
            .find(|f| files::path_key(&f.path) == key)
        {
            return Ok(folder.id.clone());
        }
        let id = format!("f_{}", files::short_hash(key.as_bytes()));
        self.library.folders.push(Folder {
            id: id.clone(),
            label: path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned(),
            path,
        });
        self.save()?;
        Ok(id)
    }

    pub fn apply_scan(&mut self, scan: Scan) {
        for demo in self.library.demos.values_mut() {
            demo.available = false;
        }
        for entry in scan.entries {
            let key = files::path_key(&entry.path);
            let existing = self
                .library
                .demos
                .values()
                .find(|d| files::path_key(&d.path) == key)
                .map(|d| d.id.clone());
            let relocated = if existing.is_none() {
                let candidates: Vec<_> = self
                    .library
                    .demos
                    .values()
                    .filter(|d| d.fingerprint == entry.fingerprint && !d.path.exists())
                    .map(|d| d.id.clone())
                    .collect();
                (candidates.len() == 1).then(|| candidates[0].clone())
            } else {
                None
            };
            let id = existing
                .or(relocated)
                .unwrap_or_else(|| format!("d_{}", files::short_hash(key.as_bytes())));
            let demo = self
                .library
                .demos
                .entry(id.clone())
                .or_insert_with(|| Demo {
                    id,
                    ..Default::default()
                });
            if demo.stamp != entry.stamp || !demo.cache_valid() {
                demo.parsed = None;
                demo.parse_error = None;
            }
            demo.path = entry.path;
            demo.stamp = entry.stamp;
            demo.fingerprint = entry.fingerprint;
            demo.folder_id = entry.folder_id;
            demo.available = true;
        }
    }

    pub fn relocate(&mut self, id: &str, destination: PathBuf, copying: bool) -> Result<String> {
        let known_folder = self
            .library
            .folders
            .iter()
            .find(|f| files::path_key(&destination).starts_with(&(files::path_key(&f.path) + "/")))
            .map(|f| f.id.clone());
        // A transferred demo must not disappear on the next scan simply because
        // the CS2 destination had not previously been watched.
        let folder_id = match known_folder {
            Some(id) => id,
            None => self.add_folder(
                destination
                    .parent()
                    .context("Destination has no parent folder")?
                    .to_owned(),
            )?,
        };
        let original = self
            .library
            .demos
            .get(id)
            .context("Demo no longer in library")?
            .clone();
        let stamp = files::stamp(&destination)?;
        if copying {
            let key = files::path_key(&destination);
            let new_id = format!("d_{}", files::short_hash(key.as_bytes()));
            self.library.demos.insert(
                new_id.clone(),
                Demo {
                    id: new_id.clone(),
                    path: destination,
                    folder_id,
                    stamp,
                    available: true,
                    ..original
                },
            );
            self.save()?;
            Ok(new_id)
        } else {
            let demo = self
                .library
                .demos
                .get_mut(id)
                .context("Demo no longer in library")?;
            demo.path = destination;
            demo.stamp = stamp;
            demo.folder_id = folder_id;
            demo.available = true;
            self.save()?;
            Ok(id.to_owned())
        }
    }
}

pub fn data_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SDM_DATA_DIR") {
        return Ok(path.into());
    }
    Ok(directories::BaseDirs::new()
        .context("Local application data directory unavailable")?
        .data_local_dir()
        .join("Small-Demo-Manager"))
}

pub struct ScannedDemo {
    pub path: PathBuf,
    pub folder_id: String,
    pub stamp: FileStamp,
    pub fingerprint: String,
}

#[derive(Default)]
pub struct Scan {
    pub entries: Vec<ScannedDemo>,
    pub warnings: Vec<String>,
}

pub fn scan(folders: &[Folder]) -> Scan {
    let mut result = Scan::default();
    let mut seen = HashSet::new();
    for folder in folders {
        let mut queue = vec![(folder.path.clone(), 0)];
        while let Some((path, depth)) = queue.pop() {
            let entries = match fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(e) => {
                    result.warnings.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        result.warnings.push(e.to_string());
                        continue;
                    }
                };
                let path = entry.path();
                let ty = match entry.file_type() {
                    Ok(t) => t,
                    Err(e) => {
                        result.warnings.push(e.to_string());
                        continue;
                    }
                };
                if ty.is_dir() && depth < 16 {
                    queue.push((path, depth + 1));
                    continue;
                }
                if !ty.is_file() || !files::is_demo(&path) {
                    continue;
                }
                if !seen.insert(files::path_key(&path)) {
                    continue;
                }
                match (files::stamp(&path), files::fingerprint(&path)) {
                    (Ok(stamp), Ok(fingerprint)) => result.entries.push(ScannedDemo {
                        path,
                        folder_id: folder.id.clone(),
                        stamp,
                        fingerprint,
                    }),
                    (Err(e), _) | (_, Err(e)) => {
                        result.warnings.push(format!("{}: {e}", path.display()))
                    }
                }
            }
        }
    }
    result
}

/// Read only. The .NET files and cached (potentially inaccurate) statistics are
/// never rewritten or imported into the native parse cache.
fn migrate_legacy(root: &Path) -> Result<Library> {
    let mut library = Library::default();
    let path = root.join("Library.json");
    if path.exists() {
        let value: serde_json::Value = serde_json::from_reader(File::open(&path)?)
            .context("Legacy Library.json is corrupt; migration stopped without modifying it")?;
        if let Some(folders) = value.get("folders").and_then(|v| v.as_array()) {
            for folder in folders {
                let path = PathBuf::from(folder["path"].as_str().unwrap_or(""));
                if path.as_os_str().is_empty() {
                    continue;
                }
                library.folders.push(Folder {
                    id: folder["id"].as_str().unwrap_or("").to_owned(),
                    label: folder["label"].as_str().unwrap_or("Demos").to_owned(),
                    path,
                });
            }
        }
        if let Some(demos) = value.get("demos").and_then(|v| v.as_object()) {
            for (id, old) in demos {
                let path = PathBuf::from(old["fullPath"].as_str().unwrap_or(""));
                if path.as_os_str().is_empty() {
                    continue;
                }
                library.demos.insert(
                    id.clone(),
                    Demo {
                        id: id.clone(),
                        path,
                        folder_id: old["folderId"].as_str().unwrap_or("").to_owned(),
                        favorite: old["fav"].as_bool().unwrap_or(false),
                        tags: old["tags"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|t| t.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        note: old["note"].as_str().unwrap_or("").to_owned(),
                        ..Default::default()
                    },
                );
            }
        }
    }
    let config = root.join("Config.json");
    if config.exists() {
        let value: serde_json::Value = serde_json::from_reader(File::open(config)?)?;
        library.settings.cs2_path = PathBuf::from(value["CS2DemoPath"].as_str().unwrap_or(""));
        library.settings.move_files = value["MoveOnImport"].as_bool().unwrap_or(false);
        library.settings.backup_before_move = value["AutoBackup"].as_bool().unwrap_or(true);
    }
    Ok(library)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_data_is_not_silently_overwritten() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("Library.native.json");
        fs::write(&path, "broken").expect("write");
        assert!(Store::open(dir.path().into()).is_err());
        assert_eq!(fs::read_to_string(path).expect("read"), "broken");
    }

    #[test]
    fn migration_preserves_user_metadata_not_invented_stats() {
        let dir = tempfile::tempdir().expect("dir");
        let old = r#"{"folders":[{"id":"f1","label":"Matches","path":"C:\\Demos"}],"demos":{"d1":{"fullPath":"C:\\Demos\\match.dem","folderId":"f1","fav":true,"tags":["review"],"note":"round 3","cached":{"rating":"1.99"}}}}"#;
        fs::write(dir.path().join("Library.json"), old).expect("write");
        let store = Store::open(dir.path().into()).expect("store");
        let demo = &store.library.demos["d1"];
        assert!(demo.favorite);
        assert_eq!(demo.note, "round 3");
        assert_eq!(demo.tags, ["review"]);
        assert!(demo.parsed.is_none());
        store.save().expect("save");
        assert_eq!(
            fs::read_to_string(dir.path().join("Library.json")).expect("read"),
            old
        );
    }

    #[test]
    fn file_changes_invalidate_cache_but_not_annotations() {
        let dir = tempfile::tempdir().expect("dir");
        let mut store = Store::open(dir.path().into()).expect("store");
        let entry = |size| ScannedDemo {
            path: dir.path().join("a.dem"),
            folder_id: "f".into(),
            stamp: FileStamp {
                size,
                modified_ns: size as u128,
            },
            fingerprint: "hash".into(),
        };
        store.apply_scan(Scan {
            entries: vec![entry(20)],
            ..Default::default()
        });
        let demo = store.library.demos.values_mut().next().expect("demo");
        demo.note = "keep".into();
        demo.parsed = Some(MatchData {
            parser_version: PARSER_VERSION.into(),
            ..Default::default()
        });
        store.apply_scan(Scan {
            entries: vec![entry(30)],
            ..Default::default()
        });
        let demo = store.library.demos.values().next().expect("demo");
        assert!(demo.parsed.is_none());
        assert_eq!(demo.note, "keep");
    }

    #[test]
    fn a_second_instance_cannot_overwrite_the_open_library() {
        let dir = tempfile::tempdir().expect("dir");
        let _first = Store::open(dir.path().into()).expect("first");
        assert!(Store::open(dir.path().into()).is_err());
    }

    #[test]
    fn moved_demos_stay_indexed_and_keep_annotations() {
        let dir = tempfile::tempdir().expect("dir");
        let mut store = Store::open(dir.path().join("state")).expect("store");
        let source = dir.path().join("source");
        let destination = dir.path().join("cs2");
        fs::create_dir(&source).expect("dir");
        fs::create_dir(&destination).expect("dir");
        fs::write(source.join("match.dem"), b"data").expect("write");
        store.add_folder(source.clone()).expect("folder");
        store.apply_scan(scan(&store.library.folders));
        let id = store.library.demos.keys().next().expect("id").clone();
        store.library.demos.get_mut(&id).expect("demo").note = "keep this".into();
        let target = destination.join("renamed.dem");
        fs::rename(source.join("match.dem"), &target).expect("rename fixture");
        store.relocate(&id, target, false).expect("relocate");
        store.apply_scan(scan(&store.library.folders));
        assert!(store.library.demos[&id].available);
        assert_eq!(store.library.demos[&id].note, "keep this");
    }
}
