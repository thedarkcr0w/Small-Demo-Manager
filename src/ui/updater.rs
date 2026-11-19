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
        if self.active.is_some() || !self.pending_imports.is_empty() || self.updater.task.is_some()
        {
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
            spinner(ui);
            if let Some(progress) = self.updater.progress {
                progress_bar(ui, progress, 130.0);
            }
            if task.cancel.load(Ordering::Relaxed) {
                hint(ui, "Cancelling…");
            } else if !self.updater.installing && small_link(ui, "Cancel").clicked() {
                task.cancel.store(true, Ordering::Relaxed);
            }
        } else if self.updater.prepared.is_some() {
            if button(
                ui,
                "Restart & install…",
                Tone::Primary,
                self.active.is_none() && self.pending_imports.is_empty() && cfg!(windows),
            )
            .clicked()
            {
                self.dialog = Some(Dialog::InstallUpdate);
            }
            if self.active.is_some() {
                hint(
                    ui,
                    "Finish or cancel the current operation before restarting.",
                );
            }
        } else if self.updater.available.as_ref().is_some_and(|u| u.newer)
            && button(ui, "Download update", Tone::Primary, cfg!(windows)).clicked()
        {
            self.download_update(ui.ctx());
        }
    }

    pub(super) fn update_banner(&mut self, ui: &mut Ui) {
        if self.updater.dismissed || !self.updater.available.as_ref().is_some_and(|u| u.newer) {
            return;
        }
        Panel::top("app_update_banner")
            .frame(
                Frame::new()
                    .fill(KEY)
                    .inner_margin(Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().hline(
                    r.x_range().expand(16.0),
                    r.bottom() + 8.5,
                    Stroke::new(1.0, EDGE),
                );
                ui.horizontal_wrapped(|ui| {
                    let (dot, _) = ui.allocate_exact_size(vec2(12.0, 20.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        4.5,
                        if self.updater.error { SIGNAL } else { OK },
                    );
                    ui.label(
                        RichText::new(&self.updater.message)
                            .font(medium(14.0))
                            .color(if self.updater.error { SIGNAL_TEXT } else { INK }),
                    );
                    ui.add_space(space::S);
                    self.update_actions(ui);
                    ui.add_space(space::S);
                    if small_link(ui, "Later").clicked() {
                        self.updater.dismissed = true;
                    }
                });
            });
    }

    pub(super) fn update_settings(&mut self, ui: &mut Ui) {
        section(ui, "App updates");
        ui.add_space(4.0);
        group_with(ui, Margin::ZERO, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            control_row(
                ui,
                true,
                "Check for updates at startup",
                &format!(
                    "Installed version {} · GitHub releases. Downloads are verified against the release SHA-256; installing asks for a restart and keeps a rollback backup. Older .NET packages are excluded.",
                    env!("CARGO_PKG_VERSION")
                ),
                |ui| {
                    switch(ui, &mut self.settings_draft.check_for_updates);
                },
            );
            divider(ui);
            Frame::new()
                .inner_margin(Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 6.0;
                    if !self.updater.message.is_empty() {
                        if self.updater.error {
                            error_text(ui, &self.updater.message);
                        } else {
                            ui.add(
                                Label::new(
                                    RichText::new(&self.updater.message)
                                        .font(medium(13.5))
                                        .color(INK),
                                )
                                .wrap(),
                            );
                        }
                    }
                    ui.horizontal_wrapped(|ui| {
                        if button(
                            ui,
                            "Check for updates",
                            Tone::Secondary,
                            self.updater.task.is_none() && self.updater.prepared.is_none(),
                        )
                        .clicked()
                        {
                            self.check_updates(ui.ctx(), true);
                        }
                        self.update_actions(ui);
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = space::L;
                        if self.updater.prepared.is_some()
                            && link_colored(ui, "Discard downloaded update", SIGNAL_TEXT).clicked()
                        {
                            self.updater.prepared = None;
                            self.updater.message =
                                "Downloaded update discarded. The installed app was not changed."
                                    .into();
                        }
                        if let Some(update) = &self.updater.available
                            && small_link(ui, &format!("Release notes · {}", update.version))
                                .clicked()
                            && let Err(error) = open::that(&update.url)
                        {
                            self.error(error);
                        }
                        if small_link(ui, "Open GitHub releases").clicked()
                            && let Err(error) = open::that(updates::RELEASES)
                        {
                            self.error(error);
                        }
                    });
                });
        });
    }
}
