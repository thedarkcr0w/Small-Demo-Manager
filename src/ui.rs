mod theme;
mod updater;
mod views;

use eframe::egui::{self, *};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use small_demo_manager::model::Layout as LibraryLayout;
use small_demo_manager::{
    audio::{self, Playback},
    files,
    model::*,
    steam,
    store::{self, Store},
    updates,
    worker::{Event, Job, Worker},
};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use theme::*;

#[derive(Clone, PartialEq)]
enum View {
    All,
    Recent,
    Favorites,
    Folder(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Sort {
    Newest,
    Oldest,
    Largest,
    Longest,
    Map,
}

enum Dialog {
    Settings,
    About,
    InstallUpdate,
    Rename {
        id: String,
        name: String,
        to_cs2: bool,
    },
    RemoveFolder(String),
    Trash(String),
}

struct Active {
    label: String,
    progress: Option<f32>,
    cancel: Option<Arc<AtomicBool>>,
    parsing: bool,
}

pub struct App {
    store: Option<Store>,
    worker: Worker,
    active: Option<Active>,
    rescan_pending: bool,
    query: String,
    view: View,
    sort: Sort,
    map_filter: String,
    tag_filter: BTreeSet<String>,
    selected: Option<String>,
    startup: Option<PathBuf>,
    selected_players: BTreeSet<String>,
    voice_open: bool,
    selected_speaker: Option<String>,
    clip_query: String,
    clips: Vec<Clip>,
    playback: Option<Playback>,
    dialog: Option<Dialog>,
    settings_draft: Settings,
    cs2_input: String,
    tag_input: String,
    dirty: Option<Instant>,
    notice: Option<(String, bool)>,
    updater: updater::State,
    textures: HashMap<String, TextureHandle>,
    watcher: Option<RecommendedWatcher>,
    watch_events: mpsc::Receiver<notify::Result<notify::Event>>,
    watch_sender: mpsc::Sender<notify::Result<notify::Event>>,
    watch_changed: Option<Instant>,
    list_collapsed: bool,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        startup: Option<PathBuf>,
        restoring_update: bool,
    ) -> Self {
        theme::install(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        let worker = Worker::start(move || ctx.request_repaint());
        let opened = store::data_dir().and_then(Store::open);
        let (store, notice) = match opened {
            Ok(store) => (Some(store), None),
            Err(e) => (
                None,
                Some((
                    format!(
                        "Cannot open library: {e:#}. No data was overwritten. Close the app and repair the data file or close the other instance."
                    ),
                    true,
                )),
            ),
        };
        let settings = store
            .as_ref()
            .map(|s| s.library.settings.clone())
            .unwrap_or_default();
        let startup = if restoring_update {
            startup.and_then(|path| match &store {
                Some(store) => store.restore_update_demo(path),
                None => Some(path),
            })
        } else {
            startup
        };
        let (watch_sender, watch_events) = mpsc::channel();
        let auto_updates = settings.check_for_updates;
        let mut app = Self {
            store,
            worker,
            active: None,
            rescan_pending: true,
            query: String::new(),
            view: View::All,
            sort: Sort::Newest,
            map_filter: String::new(),
            tag_filter: BTreeSet::new(),
            selected: None,
            startup,
            selected_players: BTreeSet::new(),
            voice_open: false,
            selected_speaker: None,
            clip_query: String::new(),
            clips: vec![],
            playback: None,
            dialog: None,
            cs2_input: settings.cs2_path.to_string_lossy().into_owned(),
            settings_draft: settings,
            tag_input: String::new(),
            dirty: None,
            notice,
            updater: updater::State {
                auto_pending: auto_updates,
                ..Default::default()
            },
            textures: HashMap::new(),
            watcher: None,
            watch_sender,
            watch_events,
            watch_changed: None,
            list_collapsed: false,
        };
        if let Some(path) = app.startup.clone().filter(|p| files::is_demo(p)) {
            match path.canonicalize() {
                Ok(path) => {
                    app.startup = Some(path.clone());
                    if let Some(parent) = path.parent()
                        && let Some(store) = &mut app.store
                        && let Err(e) = store.add_folder(parent.to_owned())
                    {
                        app.error(e);
                    }
                }
                Err(e) => app.error(e),
            }
        }
        app.refresh_watcher();
        app
    }

    fn error(&mut self, error: impl std::fmt::Display) {
        self.notice = Some((error.to_string(), true));
    }
    fn info(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), false));
    }
    fn mark_dirty(&mut self) {
        self.dirty.get_or_insert_with(Instant::now);
    }

    fn persist(&mut self) {
        if let Some(store) = &self.store {
            match store.save() {
                Ok(()) => self.dirty = None,
                Err(e) => {
                    self.error(format!("Could not save library: {e:#}"));
                    self.dirty = Some(Instant::now());
                }
            }
        }
    }

    fn send(&mut self, job: Job, label: impl Into<String>, cancel: Option<Arc<AtomicBool>>) {
        if self.active.is_some() {
            self.info("Wait for the current operation or cancel parsing first");
            return;
        }
        let parsing = matches!(&job, Job::Parse { output: None, .. });
        if let Err(e) = self.worker.sender.send(job) {
            self.error(e);
            return;
        }
        self.active = Some(Active {
            label: label.into(),
            progress: None,
            cancel,
            parsing,
        });
    }

    fn cancel_active(&mut self) {
        let Some(active) = &self.active else { return };
        if let Some(cancel) = &active.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        if active.parsing {
            // Cancel the queue as well as this file, rather than immediately
            // starting another one of thousands of pending background parses.
            if let Some(store) = &mut self.store {
                store.library.settings.background_parse = false;
            }
            self.settings_draft.background_parse = false;
            self.mark_dirty();
        }
    }

    fn refresh_watcher(&mut self) {
        self.watcher = None;
        let tx = self.watch_sender.clone();
        match notify::recommended_watcher(move |result| {
            let _ = tx.send(result);
        }) {
            Ok(mut watcher) => {
                let folders = self
                    .store
                    .as_ref()
                    .map(|s| s.library.folders.clone())
                    .unwrap_or_default();
                for folder in folders {
                    if let Err(e) = watcher.watch(&folder.path, RecursiveMode::Recursive) {
                        self.error(format!(
                            "Cannot watch {}: {e}. Use Rescan after restoring this folder",
                            folder.path.display()
                        ));
                    }
                }
                self.watcher = Some(watcher);
            }
            Err(e) => self.error(format!(
                "Filesystem watcher unavailable: {e}. Manual rescan still works"
            )),
        }
    }

    fn consume_events(&mut self) {
        while let Ok(event) = self.watch_events.try_recv() {
            match event {
                Ok(event)
                    if !matches!(event.kind, notify::EventKind::Access(_))
                        && event
                            .paths
                            .iter()
                            .any(|p| files::is_demo(p) || p.extension().is_none()) =>
                {
                    self.watch_changed = Some(Instant::now());
                }
                Err(e) => self.error(format!("Folder watch error: {e}")),
                _ => {}
            }
        }
        if self
            .watch_changed
            .is_some_and(|t| t.elapsed() >= Duration::from_millis(800))
        {
            self.watch_changed = None;
            self.rescan_pending = true;
        }
        while let Ok(event) = self.worker.events.try_recv() {
            if let Event::Progress(p) = event {
                if let Some(active) = &mut self.active {
                    active.progress = (p >= 0.0).then_some(p);
                }
                continue;
            }
            let cancelled = self
                .active
                .as_ref()
                .and_then(|a| a.cancel.as_ref())
                .is_some_and(|c| c.load(Ordering::Relaxed));
            self.active = None;
            match event {
                Event::Scanned(scan) => {
                    let warnings = scan.warnings.clone();
                    if let Some(store) = &mut self.store {
                        store.apply_scan(scan);
                    }
                    self.mark_dirty();
                    if !warnings.is_empty() {
                        self.error(warnings.join("\n"));
                    }
                    if let Some(startup) = self.startup.take() {
                        self.selected = self.store.as_ref().and_then(|s| {
                            s.library
                                .demos
                                .values()
                                .find(|d| files::path_key(&d.path) == files::path_key(&startup))
                                .map(|d| d.id.clone())
                        });
                    }
                    if self.selected.as_ref().is_none_or(|id| {
                        self.store
                            .as_ref()
                            .and_then(|s| s.library.demos.get(id))
                            .is_none_or(|d| !d.available)
                    }) {
                        self.selected = self.filtered_ids().first().cloned();
                    }
                }
                Event::Parsed {
                    id,
                    stamp,
                    extracted,
                    result,
                } => match result {
                    Ok(parsed) => {
                        if let Some(demo) = self
                            .store
                            .as_mut()
                            .and_then(|s| s.library.demos.get_mut(&id))
                            && demo.stamp == stamp
                        {
                            demo.parsed = Some(parsed.game);
                            demo.parse_error = None;
                            self.mark_dirty();
                        }
                        if extracted {
                            if self.selected.as_ref() == Some(&id) {
                                self.load_clips();
                            }
                            self.info(format!(
                                "Saved {} WAV clips. Source demo was not changed",
                                parsed.clips.len()
                            ));
                        }
                    }
                    Err(error) => {
                        if !extracted
                            && let Some(demo) = self
                                .store
                                .as_mut()
                                .and_then(|s| s.library.demos.get_mut(&id))
                        {
                            demo.parse_error = Some(if cancelled {
                                "Parsing cancelled. Use Retry to parse this demo.".into()
                            } else {
                                error.clone()
                            });
                            self.mark_dirty();
                        }
                        if cancelled {
                            self.info("Operation cancelled. Previously saved clips are unchanged");
                        } else if extracted || self.selected.as_ref() == Some(&id) {
                            self.error(error);
                        }
                    }
                },
                Event::Imported(result) => match result {
                    Ok(paths) => {
                        self.startup = paths.first().cloned();
                        self.rescan_pending = true;
                        self.info(format!("Imported {} demos; originals kept", paths.len()));
                    }
                    Err(e) => {
                        self.error(e);
                        self.rescan_pending = true;
                    }
                },
                Event::Transferred {
                    id,
                    destination,
                    copying,
                    result,
                } => match result {
                    Ok(()) => {
                        let relocated = self
                            .store
                            .as_mut()
                            .map(|store| store.relocate(&id, destination.clone(), copying));
                        if let Some(Err(e)) = relocated {
                            self.error(format!(
                                "File transfer succeeded, but metadata save failed: {e:#}"
                            ));
                            self.rescan_pending = true;
                            continue;
                        }
                        if let Some(Ok(new_id)) = relocated {
                            self.select(new_id);
                        }
                        self.view = View::All;
                        self.refresh_watcher();
                        self.rescan_pending = true;
                        self.info(format!(
                            "{} and SHA-256 verified: {}",
                            if copying { "Copied" } else { "Moved" },
                            destination.display()
                        ));
                    }
                    Err(e) => {
                        self.error(e);
                        self.rescan_pending = true;
                    }
                },
                Event::Trashed { id, result } => match result {
                    Ok(()) => {
                        if let Some(demo) = self
                            .store
                            .as_mut()
                            .and_then(|s| s.library.demos.get_mut(&id))
                        {
                            demo.available = false;
                        }
                        self.selected = None;
                        self.rescan_pending = true;
                        self.info(
                            "Demo moved to the Recycle Bin; saved audio and annotations were kept",
                        );
                    }
                    Err(e) => self.error(format!("Could not move file to the Recycle Bin: {e}")),
                },
                Event::Detected(result) => match result {
                    Ok(path) => {
                        self.cs2_input = path.to_string_lossy().into_owned();
                        self.info("CS2 install found. Save settings to use it");
                    }
                    Err(e) => self.error(e),
                },
                Event::Progress(_) => {}
            }
        }
    }

    fn schedule(&mut self) {
        if self.active.is_some() || self.store.is_none() {
            return;
        }
        if self.rescan_pending {
            self.rescan_pending = false;
            let folders = self
                .store
                .as_ref()
                .map(|s| s.library.folders.clone())
                .unwrap_or_default();
            self.send(Job::Scan(folders), "Scanning watched folders…", None);
            return;
        }
        let selected = self
            .selected
            .as_ref()
            .and_then(|id| self.store.as_ref()?.library.demos.get(id));
        let pending = selected
            .filter(|d| d.available && !d.cache_valid() && d.parse_error.is_none())
            .cloned()
            .or_else(|| {
                let store = self.store.as_ref()?;
                if !store.library.settings.background_parse {
                    return None;
                }
                store
                    .library
                    .demos
                    .values()
                    .find(|d| d.available && !d.cache_valid() && d.parse_error.is_none())
                    .cloned()
            });
        if let Some(demo) = pending {
            self.parse_demo(&demo, None);
        }
    }

    fn parse_demo(&mut self, demo: &Demo, extract_player: Option<Option<String>>) {
        let cancel = Arc::new(AtomicBool::new(false));
        let output = extract_player.as_ref().map(|_| {
            let root = self
                .store
                .as_ref()
                .map(|s| s.root.clone())
                .unwrap_or_default();
            let run = chrono::Utc::now().format("%Y%m%dT%H%M%S%.9f").to_string();
            root.join("Audio.native").join(&demo.id).join(run)
        });
        let label = format!(
            "{} {}",
            if output.is_some() {
                "Extracting voice from"
            } else {
                "Parsing"
            },
            demo.filename()
        );
        self.send(
            Job::Parse {
                id: demo.id.clone(),
                path: demo.path.clone(),
                stamp: demo.stamp.clone(),
                output,
                player: extract_player.flatten(),
                cancel: cancel.clone(),
            },
            label,
            Some(cancel),
        );
    }

    fn select(&mut self, id: String) {
        if self.selected.as_ref() == Some(&id) {
            return;
        }
        self.persist();
        self.selected = Some(id);
        self.selected_players.clear();
        self.selected_speaker = None;
        self.voice_open = false;
        self.playback = None;
        self.tag_input.clear();
        self.clips.clear();
    }

    fn load_clips(&mut self) {
        let Some(store) = &self.store else {
            return;
        };
        let Some(id) = &self.selected else {
            return;
        };
        match audio::list_clips(&store.root.join("Audio.native").join(id)) {
            Ok(clips) => self.clips = clips,
            Err(e) => self.error(format!("Cannot load saved voice clips: {e:#}")),
        }
    }

    fn pick_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Watch a demo folder (includes subfolders)")
            .pick_folder()
            && let Some(store) = &mut self.store
        {
            match store.add_folder(path) {
                Ok(id) => {
                    self.view = View::Folder(id);
                    self.rescan_pending = true;
                    self.refresh_watcher();
                }
                Err(e) => self.error(e),
            }
        }
    }

    fn import(&mut self, paths: Vec<PathBuf>) {
        let destination = self.store.as_ref().and_then(|store| {
            match &self.view {
                View::Folder(id) => store.library.folders.iter().find(|f| &f.id == id),
                _ => store.library.folders.first(),
            }
            .map(|f| f.path.clone())
        });
        match destination {
            Some(destination) => self.send(Job::Import { paths, destination }, "Importing demos (originals are kept)…", None),
            None => self.error("Add a watched folder before importing. Drops accept .dem files and ZIP archives; extract RAR/7z files first"),
        }
    }

    fn filtered_ids(&self) -> Vec<String> {
        let Some(store) = &self.store else {
            return vec![];
        };
        let cutoff =
            chrono::Utc::now().timestamp().saturating_sub(7 * 86400) as u128 * 1_000_000_000;
        let mut demos: Vec<_> = store
            .library
            .demos
            .values()
            .filter(|d| {
                d.available
                    && match &self.view {
                        View::All => true,
                        View::Favorites => d.favorite,
                        View::Recent => d.stamp.modified_ns >= cutoff,
                        View::Folder(id) => &d.folder_id == id,
                    }
            })
            .filter(|d| {
                d.matches(self.query.trim())
                    && (self.map_filter.is_empty()
                        || d.parsed.as_ref().is_some_and(|m| m.map == self.map_filter))
                    && self
                        .tag_filter
                        .iter()
                        .all(|t| d.tags.iter().any(|tag| tag.eq_ignore_ascii_case(t)))
            })
            .collect();
        demos.sort_by(|a, b| {
            match self.sort {
                Sort::Newest => b.stamp.modified_ns.cmp(&a.stamp.modified_ns),
                Sort::Oldest => a.stamp.modified_ns.cmp(&b.stamp.modified_ns),
                Sort::Largest => b.stamp.size.cmp(&a.stamp.size),
                Sort::Longest => b
                    .parsed
                    .as_ref()
                    .and_then(|m| m.duration)
                    .partial_cmp(&a.parsed.as_ref().and_then(|m| m.duration))
                    .unwrap_or(std::cmp::Ordering::Equal),
                Sort::Map => a
                    .parsed
                    .as_ref()
                    .map(|m| &m.map)
                    .cmp(&b.parsed.as_ref().map(|m| &m.map)),
            }
            .then(a.id.cmp(&b.id))
        });
        demos.iter().map(|d| d.id.clone()).collect()
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.consume_events();
        self.consume_update_events(&ctx);
        if self.store.is_some() && self.updater.auto_pending {
            self.check_updates(&ctx, false);
        }
        if self
            .dirty
            .is_some_and(|t| t.elapsed() >= Duration::from_millis(600))
        {
            self.persist();
        }
        if self.playback.as_ref().is_some_and(Playback::finished) {
            self.playback = None;
        }
        let paths = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_owned())
                .collect::<Vec<_>>()
        });
        if !paths.is_empty() {
            self.import(paths);
        }
        self.chrome(ui);
        self.status(ui);
        if self.updater.installing {
            CentralPanel::default().frame(card()).show(ui, |ui| {
                ui.heading("Installing update");
                ui.spinner();
                ui.label(&self.updater.message);
                small(ui, "Your library has been saved. The update helper will wait for this app to close, keep a backup, and restart it.");
            });
            ctx.request_repaint_after(Duration::from_millis(60));
            return;
        }
        if self.store.is_none() {
            CentralPanel::default().frame(card()).show(ui, |ui| {
                ui.heading("Library unavailable — your data was kept intact");
                ui.label("Close any other native instance. If Library.native.json is corrupt, make a backup before repairing it. Restart after resolving the error shown below.");
                if let Some((message, _)) = &self.notice { ui.colored_label(RED, message); }
                if ui.button("Open application data folder").clicked()
                    && let Ok(path) = store::data_dir()
                    && let Err(e) = open::that(path) { self.error(e); }
                if ui.button("Close").clicked() { ctx.send_viewport_cmd(ViewportCommand::Close); }
            });
            return;
        }
        self.sidebar(ui);
        self.toolbar(ui);
        self.update_banner(ui);
        let layout = self
            .store
            .as_ref()
            .map(|s| s.library.settings.layout)
            .unwrap_or_default();
        if !self.voice_open && !self.list_collapsed {
            match layout {
                LibraryLayout::Hybrid => {
                    egui::Panel::left("demo_list")
                        .default_size(300.0)
                        .min_size(230.0)
                        .max_size(500.0)
                        .frame(Frame::new().fill(BG).inner_margin(0))
                        .show(ui, |ui| self.library(ui, false));
                }
                LibraryLayout::Table | LibraryLayout::Cards => {
                    egui::Panel::top("library_above")
                        .default_size(250.0)
                        .min_size(120.0)
                        .max_size(450.0)
                        .resizable(true)
                        .frame(Frame::new().fill(BG).inner_margin(8))
                        .show(ui, |ui| self.library(ui, layout == LibraryLayout::Cards));
                }
            }
        }
        egui::CentralPanel::default()
            .frame(Frame::new().fill(SURFACE).inner_margin(18))
            .show(ui, |ui| self.details(ui));
        self.dialogs(&ctx);
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::unique("drop_hint")));
            let rect = ctx.viewport_rect().shrink(20.0);
            painter.rect_filled(rect, 10, Color32::from_black_alpha(225));
            painter.rect_stroke(rect, 10, Stroke::new(2.0, ACCENT), StrokeKind::Inside);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Drop .dem files or ZIP archives to import\nOriginal files are kept",
                FontId::proportional(22.0),
                TEXT,
            );
        }
        self.schedule();
        if self.active.is_some() || self.playback.is_some() || self.updater.task.is_some() {
            ctx.request_repaint_after(Duration::from_millis(60));
        } else {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(cancel) = self.active.as_ref().and_then(|a| a.cancel.as_ref()) {
            cancel.store(true, Ordering::Relaxed);
        }
        self.persist();
    }
}
