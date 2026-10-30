use super::*;

#[derive(Default)]
pub(super) struct State {
    pub task: Option<updates::Task>,
    pub available: Option<updates::Update>,
    pub prepared: Option<updates::PreparedUpdate>,
    pub message: String,
    pub error: bool,
    pub progress: Option<f32>,
    pub auto_pending: bool,
    pub dismissed: bool,
    pub installing: bool,
    pub manual: bool,
}

impl App {
    pub(crate) fn library_is_open(&self) -> bool {
        self.store.is_some()
    }

    pub(super) fn check_updates(&mut self, ctx: &Context, manual: bool) {
        if self.updater.task.is_some() {
            return;
        }
        self.updater.manual = manual;
        self.updater.auto_pending = false;
        self.updater.error = false;
        self.updater.message = "Checking GitHub releases…".into();
        self.updater.progress = None;
        let ctx = ctx.clone();
        self.updater.task = Some(updates::Task::start(
            move || ctx.request_repaint(),
            |_, _| updates::Event::Checked(updates::check().map_err(|e| format!("{e:#}"))),
        ));
    }

    fn download_update(&mut self, ctx: &Context) {
        if self.updater.task.is_some() {
            return;
        }
        let Some(update) = self.updater.available.clone().filter(|u| u.newer) else {
            return;
        };
        let Some(root) = self.store.as_ref().map(|s| s.root.clone()) else {
            return;
        };
        self.updater.error = false;
        self.updater.message = "Starting update download…".into();
        self.updater.progress = None;
        self.updater.dismissed = false;
        let ctx = ctx.clone();
        self.updater.task = Some(updates::Task::start(
            move || ctx.request_repaint(),
            move |cancel, progress| {
                updates::Event::Prepared(
                    updates::download(&update, &root, cancel, progress)
                        .map_err(|e| format!("{e:#}")),
                )
            },
        ));
    }

    pub(super) fn consume_update_events(&mut self, ctx: &Context) {
        loop {
            let event = self.updater.task.as_ref().map(|t| t.events.try_recv());
            let event = match event {
                Some(Ok(event)) => event,
                Some(Err(mpsc::TryRecvError::Disconnected)) => {
                    self.updater.task = None;
                    self.updater.installing = false;
                    self.updater.error = true;
                    self.updater.message = "The updater stopped unexpectedly. The installed app has not been replaced; check again to retry.".into();
                    break;
                }
                _ => break,
            };
            if let updates::Event::Progress { message, fraction } = event {
                self.updater.message = message;
                self.updater.progress = fraction;
                continue;
            }
            let cancelled = self
                .updater
                .task
                .as_ref()
                .is_some_and(|t| t.cancel.load(Ordering::Relaxed));
            self.updater.task = None;
            self.updater.progress = None;
            match event {
                updates::Event::Checked(_) if cancelled => {
                    self.updater.message = "Update check cancelled.".into();
                }
                updates::Event::Checked(result) => match result {
                    Ok(update) => {
                        self.updater.message = if update.newer {
                            format!("Update {} is available", update.version)
                        } else {
                            "You are up to date".into()
                        };
                        self.updater.available = Some(update);
                        self.updater.dismissed = false;
                    }
                    Err(error) => {
                        self.updater.error = true;
                        self.updater.message = error.clone();
                        if self.updater.manual {
                            self.error(error);
                        }
                    }
                },
                updates::Event::Prepared(result) => match result {
                    Ok(prepared) if !cancelled => {
                        self.updater.message =
                            format!("Update {} verified and ready to install", prepared.version);
                        self.updater.prepared = Some(prepared);
                    }
                    Ok(_) => {
                        self.updater.message =
                            "Download cancelled. The installed app was not changed.".into();
                    }
                    Err(error) => {
                        self.updater.error = !cancelled;
                        self.updater.message = if cancelled {
                            "Download cancelled. The installed app was not changed.".into()
                        } else {
                            error
                        };
                    }
                },
                updates::Event::Handoff(result) => match result {
                    Ok(()) => {
                        self.updater.message = "Restarting to install the verified update…".into();
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    Err(error) => {
                        self.updater.installing = false;
                        self.updater.error = true;
                        self.updater.message = error.clone();
                        self.error(error);
                    }
                },
                updates::Event::Progress { .. } => {}
            }
        }
    }

    pub(super) fn begin_update_install(&mut self, ctx: &Context) -> bool {
        if self.active.is_some() || self.updater.task.is_some() {
            return false;
        }
        let Some(store) = &self.store else {
            return false;
        };
        if let Err(error) = store.save() {
            self.error(format!(
                "The update was not started because the library could not be saved: {error:#}"
            ));
            return false;
        }
        let data_dir = store.root.clone();
        let demo = self
            .selected
            .as_ref()
            .and_then(|id| store.library.demos.get(id))
            .map(|d| d.path.clone());
        let Some(prepared) = self.updater.prepared.take() else {
            return false;
        };
        self.dirty = None;
        self.playback = None;
        self.updater.installing = true;
        self.updater.message = "Preparing a safe restart…".into();
        let ctx = ctx.clone();
        self.updater.task = Some(updates::Task::start(
            move || ctx.request_repaint(),
            move |_, _| {
                updates::Event::Handoff(
                    updates::installer::handoff(prepared, data_dir, demo)
                        .map_err(|e| format!("{e:#}")),
                )
            },
        ));
        true
    }

    fn update_actions(&mut self, ui: &mut Ui) {
        if let Some(task) = &self.updater.task {
            ui.spinner();
            if let Some(progress) = self.updater.progress {
                ui.add(
                    ProgressBar::new(progress)
                        .desired_width(130.0)
                        .show_percentage(),
                );
            }
            if task.cancel.load(Ordering::Relaxed) {
                small(ui, "Cancelling…");
            } else if !self.updater.installing && ui.button("Cancel").clicked() {
                task.cancel.store(true, Ordering::Relaxed);
            }
        } else if self.updater.prepared.is_some() {
            if ui
                .add_enabled(
                    self.active.is_none() && cfg!(windows),
                    primary("Restart & install…"),
                )
                .clicked()
            {
                self.dialog = Some(Dialog::InstallUpdate);
            }
            if self.active.is_some() {
                small(
                    ui,
                    "Finish or cancel the current operation before restarting.",
                );
            }
        } else if self.updater.available.as_ref().is_some_and(|u| u.newer)
            && ui
                .add_enabled(cfg!(windows), primary("Download update"))
                .clicked()
        {
            self.download_update(ui.ctx());
        }
    }

    pub(super) fn update_banner(&mut self, ui: &mut Ui) {
        if self.updater.dismissed || !self.updater.available.as_ref().is_some_and(|u| u.newer) {
            return;
        }
        Panel::top("app_update_banner")
            .frame(Frame::new().fill(SOFT).inner_margin(10))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(&self.updater.message).color(if self.updater.error {
                            RED
                        } else {
                            ACCENT
                        }),
                    );
                    self.update_actions(ui);
                    if ui.small_button("Later").clicked() {
                        self.updater.dismissed = true;
                    }
                });
            });
    }

    pub(super) fn update_settings(&mut self, ui: &mut Ui) {
        eyebrow(ui, "App updates");
        small(
            ui,
            format!(
                "Installed version {} · GitHub releases",
                env!("CARGO_PKG_VERSION")
            ),
        );
        ui.checkbox(
            &mut self.settings_draft.check_for_updates,
            "Automatically check for updates when the app starts",
        );
        small(
            ui,
            "Downloads are verified against the release SHA-256. Installation asks for a restart and keeps a rollback backup. Older .NET packages are excluded.",
        );
        if !self.updater.message.is_empty() {
            ui.add(
                Label::new(
                    RichText::new(&self.updater.message)
                        .size(11.0)
                        .color(if self.updater.error { RED } else { SECONDARY }),
                )
                .wrap(),
            );
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.updater.task.is_none() && self.updater.prepared.is_none(),
                    Button::new("Check for updates"),
                )
                .clicked()
            {
                self.check_updates(ui.ctx(), true);
            }
            self.update_actions(ui);
        });
        if self.updater.prepared.is_some() && ui.small_button("Discard downloaded update").clicked()
        {
            self.updater.prepared = None;
            self.updater.message =
                "Downloaded update discarded. The installed app was not changed.".into();
        }
        if let Some(update) = &self.updater.available
            && ui
                .button(format!("Release notes · {}", update.version))
                .clicked()
            && let Err(error) = open::that(&update.url)
        {
            self.error(error);
        }
        if ui.small_button("Open GitHub releases").clicked()
            && let Err(error) = open::that(updates::RELEASES)
        {
            self.error(error);
        }
    }
}
