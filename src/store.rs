use crate::{files, model::*};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
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
        let mut library = if path.exists() {
            let library: Library = serde_json::from_reader(BufReader::new(File::open(&path)?))
                .with_context(|| {
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
            Library::default()
        };
        // .NET folder IDs are f-<8 hex digits>; native IDs are f_<hash>.
        // Ignore folders imported by 2.0.0, but retain their inactive annotations.
        library
            .folders
            .retain(|folder| !legacy_folder_id(&folder.id));
        // 2.0.0 enabled bulk parsing by default, including after legacy migration.
        // Require an explicit choice in Settings before indexing an entire library.
        if !library.settings.background_parse_opt_in {
            library.settings.background_parse = false;
        }
        Ok(Self {
            root,
            library,
            _lock: lock,
        })
    }

    pub fn save(&self) -> Result<()> {
        files::atomic_json(&self.root.join("Library.native.json"), &self.library)
    }

    pub fn restore_update_demo(&self, path: PathBuf) -> Option<PathBuf> {
        let mut legacy = false;
        for demo in self.library.demos.values().filter(|demo| demo.path == path) {
            if !legacy_folder_id(&demo.folder_id) {
                return Some(path);
            }
            legacy = true;
        }
        // An old updater can pass its selected legacy demo to the replacement.
        // Do not silently re-add the legacy watched folder during that restart.
        (!legacy).then_some(path)
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
        // Canonicalizing every saved path for every scanned file made startup
        // quadratic, with filesystem calls on the window thread for each pair.
        // Resolve each path once, then use in-memory indexes for reconciliation.
        let mut by_path = HashMap::new();
        for demo in self.library.demos.values_mut() {
            demo.available = false;
            if legacy_folder_id(&demo.folder_id) {
                continue;
            }
            by_path
                .entry(files::path_key(&demo.path))
                .or_insert_with(|| demo.id.clone());
        }
        let entries: Vec<_> = scan
            .entries
            .into_iter()
            .map(|entry| (files::path_key(&entry.path), entry))
            .collect();
        let new_fingerprints: HashSet<_> = entries
            .iter()
            .filter(|(key, _)| !by_path.contains_key(key))
            .map(|(_, entry)| entry.fingerprint.as_str())
            .collect();
        let mut missing_by_fingerprint: HashMap<&str, Vec<String>> = HashMap::new();
        for demo in self.library.demos.values() {
            if !legacy_folder_id(&demo.folder_id)
                && new_fingerprints.contains(demo.fingerprint.as_str())
                && !demo.path.exists()
            {
                missing_by_fingerprint
                    .entry(&demo.fingerprint)
                    .or_default()
                    .push(demo.id.clone());
            }
        }
        // These IDs remain valid while the library is updated; do not retain
        // references to its entries or reuse a moved entry for a second file.
        let mut missing_by_fingerprint: HashMap<String, Vec<String>> = missing_by_fingerprint
            .into_iter()
            .map(|(fingerprint, ids)| (fingerprint.to_owned(), ids))
            .collect();
        for (key, entry) in entries {
            let existing = by_path.get(&key).cloned();
            let relocated = if existing.is_none() {
                missing_by_fingerprint
                    .get(&entry.fingerprint)
                    .filter(|ids| ids.len() == 1)
                    .map(|ids| ids[0].clone())
            } else {
                None
            };
            if relocated.is_some() {
                missing_by_fingerprint.remove(&entry.fingerprint);
            }
            let id = existing
                .or(relocated)
                .unwrap_or_else(|| format!("d_{}", files::short_hash(key.as_bytes())));
            by_path.insert(key, id.clone());
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

fn legacy_folder_id(id: &str) -> bool {
    id.strip_prefix("f-").is_some_and(|suffix| {
        suffix.len() == 8 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
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
    fn legacy_files_are_ignored_even_when_corrupt_and_are_never_rewritten() {
        let dir = tempfile::tempdir().expect("dir");
        let old = "corrupt legacy library - do not read or replace";
        let config = "corrupt legacy settings - do not read or replace";
        fs::write(dir.path().join("Library.json"), old).expect("old library");
        fs::write(dir.path().join("Config.json"), config).expect("old config");
        let store = Store::open(dir.path().into()).expect("store");
        assert!(store.library.folders.is_empty());
        assert!(store.library.demos.is_empty());
        assert!(!store.library.settings.background_parse);
        store.save().expect("save");
        assert_eq!(
            fs::read_to_string(dir.path().join("Library.json")).expect("read"),
            old
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("Config.json")).expect("config"),
            config
        );
    }

    #[test]
    fn imported_legacy_folders_are_not_scanned_but_annotations_survive() {
        let dir = tempfile::tempdir().expect("dir");
        let mut library = Library {
            folders: vec![
                Folder {
                    id: "f-aabbccdd".into(),
                    path: dir.path().join("legacy"),
                    ..Default::default()
                },
                Folder {
                    id: "f_native".into(),
                    path: dir.path().join("native"),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        library.demos.insert(
            "old-record".into(),
            Demo {
                id: "old-record".into(),
                folder_id: "f-aabbccdd".into(),
                path: dir.path().join("legacy/missing.dem"),
                note: "keep these annotations".into(),
                favorite: true,
                tags: vec!["review".into()],
                ..Default::default()
            },
        );
        files::atomic_json(&dir.path().join("Library.native.json"), &library).expect("fixture");
        let mut store = Store::open(dir.path().into()).expect("store");
        assert_eq!(store.library.folders.len(), 1);
        assert_eq!(store.library.folders[0].id, "f_native");
        assert!(
            store
                .restore_update_demo(dir.path().join("legacy/missing.dem"))
                .is_none()
        );
        assert!(
            store
                .restore_update_demo(dir.path().join("native/selected.dem"))
                .is_some()
        );
        store.apply_scan(Scan::default());
        store.save().expect("save");
        let demo = &store.library.demos["old-record"];
        assert!(!demo.available);
        assert_eq!(demo.note, "keep these annotations");
        assert!(demo.favorite);
        assert_eq!(demo.tags, ["review"]);
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

    #[test]
    fn rescan_matches_external_renames_without_losing_annotations() {
        let dir = tempfile::tempdir().expect("dir");
        let demos = dir.path().join("demos");
        fs::create_dir(&demos).expect("folder");
        fs::write(demos.join("before.dem"), b"unique fixture").expect("fixture");
        let mut store = Store::open(dir.path().join("data")).expect("store");
        store.add_folder(demos.clone()).expect("watch folder");
        store.apply_scan(scan(&store.library.folders));
        let id = store.library.demos.keys().next().expect("id").clone();
        let demo = store.library.demos.get_mut(&id).expect("demo");
        demo.note = "keep the annotation".into();
        demo.tags = vec!["review".into()];
        demo.favorite = true;
        fs::rename(demos.join("before.dem"), demos.join("after.dem")).expect("rename fixture");
        store.apply_scan(scan(&store.library.folders));
        assert_eq!(store.library.demos.len(), 1);
        let demo = &store.library.demos[&id];
        assert!(demo.available);
        assert_eq!(demo.filename(), "after.dem");
        assert_eq!(demo.note, "keep the annotation");
        assert_eq!(demo.tags, ["review"]);
        assert!(demo.favorite);
    }

    #[test]
    fn rescan_does_not_guess_between_identical_missing_files() {
        let dir = tempfile::tempdir().expect("dir");
        let demos = dir.path().join("demos");
        fs::create_dir(&demos).expect("folder");
        for name in ["first.dem", "second.dem"] {
            fs::write(demos.join(name), b"identical fixture").expect("fixture");
        }
        let mut store = Store::open(dir.path().join("data")).expect("store");
        store.add_folder(demos.clone()).expect("watch folder");
        store.apply_scan(scan(&store.library.folders));
        for demo in store.library.demos.values_mut() {
            demo.note = format!("annotation for {}", demo.filename());
        }
        for name in ["first.dem", "second.dem"] {
            fs::remove_file(demos.join(name)).expect("remove only this test fixture");
        }
        fs::write(demos.join("third.dem"), b"identical fixture").expect("new fixture");
        store.apply_scan(scan(&store.library.folders));
        assert_eq!(store.library.demos.len(), 3);
        for demo in store.library.demos.values() {
            if demo.filename() == "third.dem" {
                assert!(demo.available);
                assert!(demo.note.is_empty());
            } else {
                assert!(!demo.available);
                assert_eq!(demo.note, format!("annotation for {}", demo.filename()));
            }
        }
    }

    #[test]
    fn one_missing_entry_cannot_be_reused_for_two_new_paths() {
        let dir = tempfile::tempdir().expect("dir");
        let demos = dir.path().join("demos");
        fs::create_dir(&demos).expect("folder");
        fs::write(demos.join("before.dem"), b"fixture").expect("fixture");
        let mut store = Store::open(dir.path().join("data")).expect("store");
        store.add_folder(demos.clone()).expect("watch folder");
        store.apply_scan(scan(&store.library.folders));
        store.library.demos.values_mut().next().expect("demo").note = "original annotation".into();
        fs::rename(demos.join("before.dem"), demos.join("after.dem")).expect("rename fixture");
        fs::copy(demos.join("after.dem"), demos.join("copy.dem")).expect("copy fixture");
        store.apply_scan(scan(&store.library.folders));
        assert_eq!(store.library.demos.len(), 2);
        assert!(store.library.demos.values().all(|demo| demo.available));
        assert_eq!(
            store
                .library
                .demos
                .values()
                .filter(|demo| !demo.note.is_empty())
                .count(),
            1
        );
    }

    #[test]
    fn buffered_save_and_load_preserve_large_unicode_annotations() {
        let dir = tempfile::tempdir().expect("dir");
        let note = "Сачувај белешке · keep notes · 🎧\n".repeat(1_000);
        let mut store = Store::open(dir.path().into()).expect("store");
        store.library.demos.insert(
            "fixture".into(),
            Demo {
                id: "fixture".into(),
                note: note.clone(),
                favorite: true,
                tags: vec!["review".into()],
                ..Default::default()
            },
        );
        store.save().expect("buffered atomic save");
        drop(store);
        let reopened = Store::open(dir.path().into()).expect("buffered load");
        let demo = &reopened.library.demos["fixture"];
        assert_eq!(demo.note, note);
        assert!(demo.favorite);
        assert_eq!(demo.tags, ["review"]);
    }

    #[test]
    fn bulk_parsing_is_off_until_the_user_explicitly_opts_in() {
        let dir = tempfile::tempdir().expect("dir");
        fs::write(
            dir.path().join("Library.native.json"),
            r#"{"schema":1,"settings":{"background_parse":true},"demos":{"kept":{"id":"kept","note":"keep notes","favorite":true}}}"#,
        )
        .expect("2.0.0 fixture");
        let mut store = Store::open(dir.path().into()).expect("store");
        assert!(!store.library.settings.background_parse);
        assert!(store.library.demos["kept"].favorite);
        assert_eq!(store.library.demos["kept"].note, "keep notes");
        store.library.settings.background_parse = true;
        store.library.settings.background_parse_opt_in = true;
        store.save().expect("save explicit preference");
        drop(store);
        let reopened = Store::open(dir.path().into()).expect("reopen");
        assert!(reopened.library.settings.background_parse);
        assert!(reopened.library.settings.background_parse_opt_in);
    }
}
