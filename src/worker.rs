use crate::{
    audio::AudioWriter,
    files,
    model::{FileStamp, Folder},
    parser, steam, store,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{self, Receiver, Sender},
    },
};

pub enum Job {
    Scan(Vec<Folder>),
    Parse {
        id: String,
        path: PathBuf,
        stamp: FileStamp,
        output: Option<PathBuf>,
        player: Option<String>,
        cancel: Arc<AtomicBool>,
    },
    Import {
        paths: Vec<PathBuf>,
        destination: PathBuf,
    },
    Transfer {
        id: String,
        source: PathBuf,
        destination: PathBuf,
        moving: bool,
        backup: Option<PathBuf>,
    },
    Rename {
        id: String,
        source: PathBuf,
        name: String,
    },
    Trash {
        id: String,
        path: PathBuf,
    },
    Detect,
}

pub enum Event {
    Progress(f32),
    Scanned(store::Scan),
    Parsed {
        id: String,
        stamp: FileStamp,
        extracted: bool,
        result: Result<Box<parser::Parsed>, String>,
    },
    Imported(Result<Vec<PathBuf>, String>),
    Transferred {
        id: String,
        destination: PathBuf,
        copying: bool,
        result: Result<(), String>,
    },
    Trashed {
        id: String,
        result: Result<(), String>,
    },
    Detected(Result<PathBuf, String>),
}

pub struct Worker {
    pub sender: Sender<Job>,
    pub events: Receiver<Event>,
}

/// Stack for threads that decode demos. The entity decoder recurses deeply: a real
/// 178 MB demo needs ~1 MiB in release and ~2.5 MiB in debug, more than the 2 MiB
/// default for spawned threads (and Windows' 1 MiB main thread). Overflowing aborts
/// the whole process, so reserve generously; untouched stack costs only address space.
pub const DECODER_STACK_BYTES: usize = 16 * 1024 * 1024;

impl Worker {
    pub fn start(repaint: impl Fn() + Send + 'static) -> Self {
        let (sender, jobs) = mpsc::channel();
        let (events, receiver) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("sdm-worker".into())
            .stack_size(DECODER_STACK_BYTES)
            .spawn(move || {
                while let Ok(job) = jobs.recv() {
                    let event = match job {
                        Job::Scan(folders) => Event::Scanned(store::scan(&folders)),
                        Job::Parse {
                            id,
                            path,
                            stamp,
                            output,
                            player,
                            cancel,
                        } => {
                            let extracted = output.is_some();
                            let tx = events.clone();
                            let result = (|| -> anyhow::Result<_> {
                                let writer = output
                                    .as_ref()
                                    .map(|path| AudioWriter::new(path, player))
                                    .transpose()?;
                                parser::parse(
                                    &path,
                                    cancel,
                                    Arc::new(move |p| {
                                        let _ = tx.send(Event::Progress(p));
                                    }),
                                    writer,
                                )
                            })()
                            .map(Box::new)
                            .map_err(|e| format!("{e:#}"));
                            Event::Parsed {
                                id,
                                stamp,
                                extracted,
                                result,
                            }
                        }
                        Job::Import { paths, destination } => Event::Imported(
                            files::import(&paths, &destination).map_err(|e| format!("{e:#}")),
                        ),
                        Job::Transfer {
                            id,
                            source,
                            destination,
                            moving,
                            backup,
                        } => {
                            let result =
                                files::transfer(&source, &destination, moving, backup.as_deref())
                                    .map_err(|e| format!("{e:#}"));
                            Event::Transferred {
                                id,
                                destination,
                                copying: !moving,
                                result,
                            }
                        }
                        Job::Rename { id, source, name } => match files::rename(&source, &name) {
                            Ok(destination) => Event::Transferred {
                                id,
                                destination,
                                copying: false,
                                result: Ok(()),
                            },
                            Err(e) => Event::Transferred {
                                id,
                                destination: source,
                                copying: false,
                                result: Err(format!("{e:#}")),
                            },
                        },
                        Job::Trash { id, path } => Event::Trashed {
                            id,
                            result: trash::delete(path).map_err(|e| e.to_string()),
                        },
                        Job::Detect => Event::Detected(steam::detect().map_err(|e| e.to_string())),
                    };
                    if events.send(event).is_err() {
                        break;
                    }
                    repaint();
                }
            });
        // If the thread cannot be created, its channel ends are dropped: every job
        // send then fails and the UI reports it, instead of the app panicking here.
        if let Err(error) = spawned {
            eprintln!("Could not start the worker thread: {error}");
        }
        Self {
            sender,
            events: receiver,
        }
    }
}
