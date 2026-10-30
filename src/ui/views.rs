use super::*;

impl App {
    pub(super) fn chrome(&mut self, ui: &mut Ui) {
        Panel::top("titlebar")
            .exact_size(36.0)
            .frame(
                Frame::new()
                    .fill(SURFACE)
                    .inner_margin(Margin::symmetric(10, 2)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("SDM").strong().color(ACCENT));
                    let (rect, response) = ui.allocate_exact_size(
                        vec2((ui.available_width() - 150.0).max(20.0), 30.0),
                        Sense::click_and_drag(),
                    );
                    ui.painter().text(
                        rect.left_center(),
                        Align2::LEFT_CENTER,
                        format!("SmallDemoManager   · v{}", env!("CARGO_PKG_VERSION")),
                        FontId::proportional(12.0),
                        SECONDARY,
                    );
                    if response.drag_started() {
                        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    if response.double_clicked() {
                        let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                        ui.ctx()
                            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                    }
                    if ui
                        .add_sized([36.0, 28.0], Button::new("—").frame(false))
                        .clicked()
                    {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                    if ui
                        .add_sized([36.0, 28.0], Button::new("□").frame(false))
                        .clicked()
                    {
                        let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                        ui.ctx()
                            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                    }
                    if ui
                        .add_sized(
                            [36.0, 28.0],
                            Button::new(RichText::new("×").size(18.0)).frame(false),
                        )
                        .clicked()
                    {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
    }

    pub(super) fn status(&mut self, ui: &mut Ui) {
        Panel::bottom("status")
            .frame(Frame::new().fill(BG).inner_margin(Margin::symmetric(12, 6)))
            .show(ui, |ui| {
                if let Some((text, is_error)) = self.notice.clone() {
                    ui.horizontal(|ui| {
                        ui.add(
                            Label::new(
                                RichText::new(&text)
                                    .color(if is_error { RED } else { SECONDARY })
                                    .size(12.0),
                            )
                            .wrap(),
                        );
                        if ui
                            .small_button("×")
                            .on_hover_text("Dismiss message")
                            .clicked()
                        {
                            self.notice = None;
                        }
                    });
                    ui.separator();
                }
                ui.horizontal(|ui| {
                    if let Some(active) = &self.active {
                        ui.spinner();
                        ui.label(RichText::new(&active.label).size(11.0).color(SECONDARY));
                        if let Some(p) = active.progress {
                            ui.add(ProgressBar::new(p).desired_width(120.0).show_percentage());
                        }
                        if let Some(cancel) = &active.cancel
                            && ui.small_button("Cancel").clicked()
                        {
                            cancel.store(true, Ordering::Relaxed);
                        }
                    } else {
                        small(
                            ui,
                            if self.watcher.is_some() {
                                "●  Watching folders · ready"
                            } else {
                                "Manual rescan available"
                            },
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        small(
                            ui,
                            if self.dirty.is_some() {
                                "Saving annotations…"
                            } else {
                                "Rust · Source 2 parser 0.5.10 · WAV / Opus"
                            },
                        );
                    });
                });
            });
    }

    pub(super) fn sidebar(&mut self, ui: &mut Ui) {
        Panel::left("sidebar")
            .exact_size(210.0)
            .resizable(false)
            .frame(Frame::new().fill(SURFACE).inner_margin(10))
            .show(ui, |ui| {
                let demos = self
                    .store
                    .as_ref()
                    .map(|s| {
                        s.library
                            .demos
                            .values()
                            .filter(|d| d.available)
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let cutoff = chrono::Utc::now().timestamp().saturating_sub(7 * 86400) as u128
                    * 1_000_000_000;
                for (view, title, count) in [
                    (View::All, "≡   All demos", demos.len()),
                    (
                        View::Recent,
                        "◷   Recent",
                        demos
                            .iter()
                            .filter(|d| d.stamp.modified_ns >= cutoff)
                            .count(),
                    ),
                    (
                        View::Favorites,
                        "☆   Favorites",
                        demos.iter().filter(|d| d.favorite).count(),
                    ),
                ] {
                    let selected = self.view == view;
                    let label = format!("{title}                 {count}");
                    if ui
                        .add_sized(
                            [ui.available_width(), 34.0],
                            Button::new(RichText::new(label).color(if selected {
                                ACCENT
                            } else {
                                SECONDARY
                            }))
                            .fill(if selected { SOFT } else { Color32::TRANSPARENT })
                            .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        self.view = view;
                    }
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    eyebrow(ui, "Watched folders");
                    if ui
                        .small_button("+")
                        .on_hover_text("Watch a folder")
                        .clicked()
                    {
                        self.pick_folder();
                    }
                });
                let folders = self
                    .store
                    .as_ref()
                    .map(|s| s.library.folders.clone())
                    .unwrap_or_default();
                ScrollArea::vertical()
                    .id_salt("folder_sidebar")
                    .max_height((ui.available_height() - 260.0).max(100.0))
                    .show(ui, |ui| {
                        for folder in &folders {
                            let selected = self.view == View::Folder(folder.id.clone());
                            let count = demos.iter().filter(|d| d.folder_id == folder.id).count();
                            let response = Frame::new()
                                .fill(if selected { SOFT } else { Color32::TRANSPARENT })
                                .corner_radius(6)
                                .inner_margin(7)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("▱").color(MUTED));
                                        ui.vertical(|ui| {
                                            ui.add(
                                                Label::new(
                                                    RichText::new(format!(
                                                        "{}  {count}",
                                                        folder.label
                                                    ))
                                                    .size(12.0)
                                                    .strong()
                                                    .color(if selected { ACCENT } else { TEXT }),
                                                )
                                                .truncate(),
                                            );
                                            ui.add(
                                                Label::new(
                                                    RichText::new(folder.path.to_string_lossy())
                                                        .monospace()
                                                        .size(9.5)
                                                        .color(MUTED),
                                                )
                                                .truncate(),
                                            );
                                        });
                                    });
                                })
                                .response
                                .interact(Sense::click());
                            if response.clicked() {
                                self.view = View::Folder(folder.id.clone());
                            }
                            response
                                .on_hover_text(folder.path.display().to_string())
                                .context_menu(|ui| {
                                    if ui.button("Stop watching (keep files)").clicked() {
                                        self.dialog = Some(Dialog::RemoveFolder(folder.id.clone()));
                                        ui.close();
                                    }
                                });
                        }
                        if folders.is_empty() {
                            small(ui, "Add your CS2, FACEIT, or downloaded demo folder.");
                        }
                    });
                ui.add_space(12.0);
                eyebrow(ui, "Filter by tag");
                let tags: BTreeSet<_> = demos.iter().flat_map(|d| d.tags.iter().cloned()).collect();
                ui.horizontal_wrapped(|ui| {
                    for tag in &tags {
                        if ui
                            .selectable_label(self.tag_filter.contains(tag), tag)
                            .clicked()
                            && !self.tag_filter.remove(tag)
                        {
                            self.tag_filter.insert(tag.clone());
                        }
                    }
                });
                if tags.is_empty() {
                    small(ui, "No tags yet — add them in the details panel.");
                }
                ui.with_layout(egui::Layout::bottom_up(Align::Min), |ui| {
                    if ui
                        .add_sized(
                            [ui.available_width(), 34.0],
                            Button::new("↻   Rescan all folders"),
                        )
                        .clicked()
                    {
                        self.rescan_pending = true;
                    }
                    ui.separator();
                    if ui
                        .add_sized(
                            [ui.available_width(), 40.0],
                            Button::new("SDM   About the project"),
                        )
                        .clicked()
                    {
                        self.dialog = Some(Dialog::About);
                    }
                    if ui
                        .add_sized([ui.available_width(), 40.0], Button::new("⚙   Settings"))
                        .clicked()
                    {
                        self.settings_draft = self
                            .store
                            .as_ref()
                            .map(|s| s.library.settings.clone())
                            .unwrap_or_default();
                        self.cs2_input =
                            self.settings_draft.cs2_path.to_string_lossy().into_owned();
                        self.dialog = Some(Dialog::Settings);
                    }
                });
            });
    }

    pub(super) fn toolbar(&mut self, ui: &mut Ui) {
        Panel::top("toolbar")
            .exact_size(56.0)
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let search = ui.add(
                        TextEdit::singleline(&mut self.query)
                            .id(Id::unique("library_search"))
                            .hint_text("Search filename, player, SteamID, server, tag…")
                            .desired_width((ui.available_width() - 620.0).max(130.0)),
                    );
                    if ui.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::K)) {
                        search.request_focus();
                    }
                    let maps: BTreeSet<_> = self
                        .store
                        .as_ref()
                        .map(|s| {
                            s.library
                                .demos
                                .values()
                                .filter(|d| d.available)
                                .filter_map(|d| d.parsed.as_ref().map(|m| m.map.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    ComboBox::from_id_salt("map_filter")
                        .width(110.0)
                        .selected_text(if self.map_filter.is_empty() {
                            "Map: All".into()
                        } else {
                            map_name(&self.map_filter)
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.map_filter, String::new(), "All maps");
                            for map in maps {
                                ui.selectable_value(
                                    &mut self.map_filter,
                                    map.clone(),
                                    map_name(&map),
                                );
                            }
                        });
                    ComboBox::from_id_salt("sort")
                        .width(110.0)
                        .selected_text(match self.sort {
                            Sort::Newest => "Newest first",
                            Sort::Oldest => "Oldest first",
                            Sort::Largest => "Largest",
                            Sort::Longest => "Longest",
                            Sort::Map => "Map A–Z",
                        })
                        .show_ui(ui, |ui| {
                            for (sort, name) in [
                                (Sort::Newest, "Newest first"),
                                (Sort::Oldest, "Oldest first"),
                                (Sort::Largest, "Largest"),
                                (Sort::Longest, "Longest"),
                                (Sort::Map, "Map A–Z"),
                            ] {
                                ui.selectable_value(&mut self.sort, sort, name);
                            }
                        });
                    let mut layout = self
                        .store
                        .as_ref()
                        .map(|s| s.library.settings.layout)
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 1.0;
                        for (value, label, hint) in [
                            (LibraryLayout::Table, "≡", "Table"),
                            (LibraryLayout::Cards, "▦", "Cards"),
                            (LibraryLayout::Hybrid, "◧", "Hybrid"),
                        ] {
                            if ui
                                .selectable_value(&mut layout, value, label)
                                .on_hover_text(hint)
                                .clicked()
                            {
                                if let Some(store) = &mut self.store {
                                    store.library.settings.layout = layout;
                                }
                                self.list_collapsed = false;
                                self.mark_dirty();
                            }
                        }
                    });
                    if ui
                        .small_button(if self.list_collapsed {
                            "Show list"
                        } else {
                            "Hide list"
                        })
                        .clicked()
                    {
                        self.list_collapsed = !self.list_collapsed;
                    }
                    if ui
                        .add_enabled(self.active.is_none(), Button::new("Import…"))
                        .clicked()
                        && let Some(paths) = rfd::FileDialog::new()
                            .add_filter("CS2 demos and ZIP", &["dem", "zip"])
                            .pick_files()
                    {
                        self.import(paths);
                    }
                    if ui.add(primary("+ Add folder")).clicked() {
                        self.pick_folder();
                    }
                });
            });
    }

    pub(super) fn library(&mut self, ui: &mut Ui, cards: bool) {
        let ids = self.filtered_ids();
        let title = match &self.view {
            View::All => "All demos".into(),
            View::Recent => "Recent · last 7 days".into(),
            View::Favorites => "Favorites".into(),
            View::Folder(id) => self
                .store
                .as_ref()
                .and_then(|s| s.library.folders.iter().find(|f| &f.id == id))
                .map(|f| f.label.clone())
                .unwrap_or_default(),
        };
        Frame::new().fill(SURFACE).inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong());
                ui.label(
                    RichText::new(format!("{} demos", ids.len()))
                        .size(11.0)
                        .color(MUTED),
                );
            });
        });
        ScrollArea::vertical()
            .id_salt("demo_library")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if cards {
                    ui.horizontal_wrapped(|ui| {
                        for id in &ids {
                            if let Some(demo) = self
                                .store
                                .as_ref()
                                .and_then(|s| s.library.demos.get(id))
                                .cloned()
                            {
                                self.demo_card(ui, &demo);
                            }
                        }
                    });
                } else {
                    for id in &ids {
                        if let Some(demo) = self
                            .store
                            .as_ref()
                            .and_then(|s| s.library.demos.get(id))
                            .cloned()
                        {
                            self.demo_row(ui, &demo);
                        }
                    }
                }
                if ids.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No demos to show").color(SECONDARY));
                        small(ui, "Add a folder, import a demo, or clear your filters.");
                        if ui.button("Clear filters").clicked() {
                            self.query.clear();
                            self.map_filter.clear();
                            self.tag_filter.clear();
                            self.view = View::All;
                        }
                    });
                }
            });
    }

    fn demo_row(&mut self, ui: &mut Ui, demo: &Demo) {
        let selected = self.selected.as_ref() == Some(&demo.id);
        let wide = ui.available_width() > 550.0;
        let (rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), if wide { 58.0 } else { 76.0 }),
            Sense::click(),
        );
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            0,
            if selected {
                SOFT
            } else if response.hovered() {
                HOVER
            } else {
                BG
            },
        );
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, BORDER),
        );
        if selected {
            painter.rect_filled(
                Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
                0,
                ACCENT,
            );
        }
        let star_rect = Rect::from_min_size(rect.min + vec2(6.0, 14.0), vec2(24.0, 30.0));
        painter.text(
            star_rect.center(),
            Align2::CENTER_CENTER,
            if demo.favorite { "★" } else { "☆" },
            FontId::proportional(15.0),
            if demo.favorite { ACCENT } else { MUTED },
        );
        let star = ui.interact(
            star_rect,
            Id::unique(("favorite", &demo.id)),
            Sense::click(),
        );
        if star.clicked() {
            if let Some(d) = self
                .store
                .as_mut()
                .and_then(|s| s.library.demos.get_mut(&demo.id))
            {
                d.favorite = !d.favorite;
            }
            self.mark_dirty();
        } else if response.clicked() {
            self.select(demo.id.clone());
        }
        let game = demo.parsed.as_ref();
        let name_width = if wide {
            rect.width() - 460.0
        } else {
            rect.width() - 46.0
        };
        let filename = ellipsize(&demo.filename(), (name_width / 6.8).max(12.0) as usize);
        let x = rect.left() + 36.0;
        ui.painter().text(
            pos2(x, rect.top() + 15.0),
            Align2::LEFT_CENTER,
            filename,
            FontId::monospace(11.0),
            TEXT,
        );
        let description = game
            .map(|g| format!("{} · {}", map_name(&g.map), g.server))
            .unwrap_or_else(|| {
                if demo.parse_error.is_some() {
                    "Parse failed — select for details".into()
                } else {
                    "Waiting for parser…".into()
                }
            });
        ui.painter().text(
            pos2(x, rect.top() + 34.0),
            Align2::LEFT_CENTER,
            ellipsize(&description, if wide { 65 } else { 32 }),
            FontId::proportional(10.0),
            if demo.parse_error.is_some() {
                RED
            } else {
                MUTED
            },
        );
        if wide {
            let score = game
                .map(|g| {
                    format!(
                        "{} : {}",
                        number(g.teams[0].score),
                        number(g.teams[1].score)
                    )
                })
                .unwrap_or_else(|| "— : —".into());
            ui.painter().text(
                pos2(rect.right() - 300.0, rect.center().y),
                Align2::RIGHT_CENTER,
                score,
                FontId::monospace(13.0),
                ACCENT,
            );
            ui.painter().text(
                pos2(rect.right() - 190.0, rect.center().y),
                Align2::RIGHT_CENTER,
                duration(game.and_then(|g| g.duration)),
                FontId::monospace(11.0),
                SECONDARY,
            );
            ui.painter().text(
                pos2(rect.right() - 90.0, rect.center().y),
                Align2::RIGHT_CENTER,
                size(demo.stamp.size),
                FontId::monospace(11.0),
                MUTED,
            );
            let date = chrono::DateTime::from_timestamp(
                (demo.stamp.modified_ns / 1_000_000_000) as i64,
                0,
            )
            .map(|d| d.format("%b %d").to_string())
            .unwrap_or_default();
            ui.painter().text(
                pos2(rect.right() - 12.0, rect.center().y),
                Align2::RIGHT_CENTER,
                date,
                FontId::monospace(10.0),
                MUTED,
            );
        } else if let Some(game) = game {
            let text = format!(
                "{}  {} : {}  {}",
                ellipsize(&game.teams[0].name, 15),
                number(game.teams[0].score),
                number(game.teams[1].score),
                ellipsize(&game.teams[1].name, 15)
            );
            ui.painter().text(
                pos2(x, rect.top() + 55.0),
                Align2::LEFT_CENTER,
                text,
                FontId::proportional(10.0),
                SECONDARY,
            );
        }
        response.on_hover_text(demo.path.display().to_string());
    }

    fn demo_card(&mut self, ui: &mut Ui, demo: &Demo) {
        let selected = self.selected.as_ref() == Some(&demo.id);
        let response = Frame::new()
            .fill(if selected { SOFT } else { CARD })
            .stroke(Stroke::new(1.0, if selected { ACCENT } else { BORDER }))
            .corner_radius(8)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(210.0);
                    ui.set_min_height(105.0);
                    ui.label(
                        RichText::new(
                            demo.parsed
                                .as_ref()
                                .map(|g| map_name(&g.map))
                                .unwrap_or_else(|| "Unparsed demo".into()),
                        )
                        .size(18.0)
                        .strong(),
                    );
                    ui.add(Label::new(RichText::new(demo.filename()).monospace()).truncate());
                    if let Some(game) = &demo.parsed {
                        ui.label(
                            RichText::new(format!(
                                "{} : {}",
                                number(game.teams[0].score),
                                number(game.teams[1].score)
                            ))
                            .color(ACCENT)
                            .size(18.0),
                        );
                    }
                    small(
                        ui,
                        format!(
                            "{} · {}",
                            duration(demo.parsed.as_ref().and_then(|g| g.duration)),
                            size(demo.stamp.size)
                        ),
                    );
                });
            })
            .response
            .interact(Sense::click());
        if response.clicked() {
            self.select(demo.id.clone());
        }
    }

    pub(super) fn details(&mut self, ui: &mut Ui) {
        let demo = self
            .selected
            .as_ref()
            .and_then(|id| self.store.as_ref()?.library.demos.get(id))
            .cloned();
        let Some(demo) = demo.filter(|d| d.available) else {
            ui.add_space(70.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Your demos. Nothing in the way.")
                        .size(24.0)
                        .strong(),
                );
                small(ui, "A native CS2 library — no browser, no sample matches.");
                ui.add_space(20.0);
                if ui.add(primary("+ Add your demo folder")).clicked() {
                    self.pick_folder();
                }
                small(
                    ui,
                    "Or drag a .dem file / ZIP archive into the window after adding a folder.",
                );
            });
            return;
        };
        if self.voice_open {
            self.voice(ui, &demo);
            return;
        }
        ScrollArea::vertical()
            .id_salt(("details", &demo.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    card().show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 180.0).max(100.0));
                            eyebrow(ui, "Loaded demo · drop files to import");
                            ui.add(
                                Label::new(
                                    RichText::new(files::display_path(&demo.path))
                                        .monospace()
                                        .size(11.0),
                                )
                                .truncate(),
                            )
                            .on_hover_text(demo.path.display().to_string());
                        });
                    });
                    let moving = self
                        .store
                        .as_ref()
                        .is_some_and(|s| s.library.settings.move_files);
                    if ui
                        .add_enabled(
                            self.active.is_none(),
                            primary(if moving {
                                "▶  Move to CS2"
                            } else {
                                "▶  Copy to CS2"
                            }),
                        )
                        .clicked()
                    {
                        self.dialog = Some(Dialog::Rename {
                            id: demo.id.clone(),
                            name: demo
                                .path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            to_cs2: true,
                        });
                    }
                });
                ui.add_space(8.0);
                if let Some(error) = &demo.parse_error {
                    card().show(ui, |ui| {
                        ui.colored_label(RED, error);
                        if ui
                            .add_enabled(self.active.is_none(), Button::new("Retry parsing"))
                            .clicked()
                        {
                            if let Some(d) = self
                                .store
                                .as_mut()
                                .and_then(|s| s.library.demos.get_mut(&demo.id))
                            {
                                d.parse_error = None;
                            }
                            self.parse_demo(&demo, None);
                        }
                    });
                }
                if let Some(game) = &demo.parsed {
                    self.match_view(ui, &demo, game);
                    self.command_bar(ui, game);
                    self.timeline(ui, game);
                    for warning in &game.warnings {
                        small(ui, warning);
                    }
                } else if demo.parse_error.is_none() {
                    card().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Reading real match data…");
                        });
                        small(
                            ui,
                            "Map, scores, players and rounds will appear after parsing.",
                        );
                    });
                }
                self.annotations(ui, &demo);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(if demo.favorite {
                            "★ Favorited"
                        } else {
                            "☆ Favorite"
                        })
                        .clicked()
                    {
                        if let Some(d) = self
                            .store
                            .as_mut()
                            .and_then(|s| s.library.demos.get_mut(&demo.id))
                        {
                            d.favorite = !d.favorite;
                        }
                        self.mark_dirty();
                    }
                    if ui.button("Reveal in folder").clicked()
                        && let Err(e) = files::reveal(&demo.path)
                    {
                        self.error(e);
                    }
                    if ui.button("Copy playdemo command").clicked() {
                        let cs2 = self
                            .store
                            .as_ref()
                            .map(|s| s.library.settings.cs2_path.clone())
                            .unwrap_or_default();
                        ui.ctx().copy_text(steam::play_command(&demo.path, &cs2));
                        self.info("Copied playdemo command");
                    }
                    if ui
                        .add_enabled(self.active.is_none(), Button::new("Rename"))
                        .clicked()
                    {
                        self.dialog = Some(Dialog::Rename {
                            id: demo.id.clone(),
                            name: demo
                                .path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            to_cs2: false,
                        });
                    }
                    if ui
                        .add_enabled(
                            self.active.is_none(),
                            Button::new(RichText::new("Recycle Bin").color(RED)),
                        )
                        .clicked()
                    {
                        self.dialog = Some(Dialog::Trash(demo.id.clone()));
                    }
                });
                ui.add_space(10.0);
                if let Some(game) = &demo.parsed {
                    small(
                        ui,
                        format!(
                            "Game build {} · {} ticks · {} · {}",
                            game.game_build,
                            game.ticks,
                            game.tick_interval
                                .map(|t| format!("{:.2} ticks/s", 1.0 / t))
                                .unwrap_or_else(|| "tick rate unavailable".into()),
                            size(demo.stamp.size)
                        ),
                    );
                }
            });
    }

    fn match_view(&mut self, ui: &mut Ui, demo: &Demo, game: &MatchData) {
        let center_width = 118.0;
        let team_width = ((ui.available_width() - center_width - 16.0) / 2.0).max(150.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                vec2(team_width, 320.0),
                egui::Layout::top_down(Align::Min),
                |ui| self.team(ui, game, 0, team_width),
            );
            ui.allocate_ui_with_layout(
                vec2(center_width, 320.0),
                egui::Layout::top_down(Align::Center),
                |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(center_width, 320.0), Sense::hover());
                    ui.painter().rect_filled(rect, 8, BG);
                    if !self.textures.contains_key(&game.map)
                        && let Some(bytes) = map_art(&game.map)
                        && let Ok(image) = image::load_from_memory(bytes)
                    {
                        let image = image.to_rgba8();
                        self.textures.insert(
                            game.map.clone(),
                            ui.ctx().load_texture(
                                &game.map,
                                ColorImage::from_rgba_unmultiplied(
                                    [image.width() as usize, image.height() as usize],
                                    image.as_raw(),
                                ),
                                TextureOptions::LINEAR,
                            ),
                        );
                    }
                    if let Some(texture) = self.textures.get(&game.map) {
                        ui.painter().image(
                            texture.id(),
                            rect,
                            Rect::from_min_max(pos2(0.2, 0.0), pos2(0.8, 1.0)),
                            Color32::from_rgb(95, 95, 95),
                        );
                        ui.painter()
                            .rect_filled(rect, 8, Color32::from_black_alpha(95));
                    }
                    ui.painter()
                        .rect_stroke(rect, 8, Stroke::new(1.0, BORDER), StrokeKind::Inside);
                    let button = Rect::from_min_size(
                        rect.min + vec2(4.0, 12.0),
                        vec2(center_width - 8.0, 30.0),
                    );
                    if ui
                        .put(
                            button,
                            Button::new(RichText::new("Voice extractor").size(10.5).color(ACCENT))
                                .fill(BG),
                        )
                        .clicked()
                    {
                        self.voice_open = true;
                        self.load_clips();
                        self.selected_speaker = game.players.first().map(Player::key);
                    }
                    ui.painter().text(
                        pos2(rect.center().x, rect.top() + 108.0),
                        Align2::CENTER_CENTER,
                        &game.map,
                        FontId::monospace(10.0),
                        SECONDARY,
                    );
                    ui.painter().text(
                        pos2(rect.center().x, rect.top() + 132.0),
                        Align2::CENTER_CENTER,
                        map_name(&game.map),
                        FontId::proportional(19.0),
                        TEXT,
                    );
                    ui.painter().text(
                        pos2(rect.center().x, rect.top() + 186.0),
                        Align2::CENTER_CENTER,
                        "VS",
                        FontId::proportional(12.0),
                        MUTED,
                    );
                    ui.painter().text(
                        pos2(rect.center().x, rect.top() + 220.0),
                        Align2::CENTER_CENTER,
                        format!(
                            "{} : {}",
                            number(game.teams[0].score),
                            number(game.teams[1].score)
                        ),
                        FontId::monospace(23.0),
                        ACCENT,
                    );
                    ui.painter().text(
                        pos2(rect.center().x, rect.bottom() - 30.0),
                        Align2::CENTER_CENTER,
                        duration(game.duration),
                        FontId::monospace(12.0),
                        SECONDARY,
                    );
                },
            );
            ui.allocate_ui_with_layout(
                vec2(team_width, 320.0),
                egui::Layout::top_down(Align::Min),
                |ui| self.team(ui, game, 1, team_width),
            );
        });
        let _ = demo;
    }

    fn team(&mut self, ui: &mut Ui, game: &MatchData, index: usize, width: f32) {
        Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.set_width((width - 26.0).max(120.0));
                ui.set_min_height(294.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        eyebrow(ui, if index == 0 { "Team A" } else { "Team B" });
                        ui.add(
                            Label::new(
                                RichText::new(if game.teams[index].name.is_empty() {
                                    if index == 0 { "Team A" } else { "Team B" }
                                } else {
                                    &game.teams[index].name
                                })
                                .strong(),
                            )
                            .truncate(),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(number(game.teams[index].score))
                                .monospace()
                                .size(24.0)
                                .color(if game.teams[index].score > game.teams[1 - index].score {
                                    ACCENT
                                } else {
                                    SECONDARY
                                }),
                        );
                    });
                });
                ui.separator();
                let players: Vec<_> = game
                    .players
                    .iter()
                    .filter(|p| p.team == Some(index))
                    .collect();
                for player in &players {
                    ui.push_id(player.key(), |ui| {
                        ui.horizontal(|ui| {
                            let key = player.key();
                            let mut selected = self.selected_players.contains(&key);
                            if ui.checkbox(&mut selected, "").changed() {
                                if selected {
                                    self.selected_players.insert(key);
                                } else {
                                    self.selected_players.remove(&key);
                                }
                            }
                            let response = ui
                                .vertical(|ui| {
                                    ui.add(
                                        Label::new(RichText::new(&player.name).size(12.5).strong())
                                            .truncate(),
                                    );
                                    let stats = format!(
                                        "{}/{}/{}  · {} ADR",
                                        number(player.kills),
                                        number(player.deaths),
                                        number(player.assists),
                                        player
                                            .adr(game.completed_rounds)
                                            .map(|v| format!("{v:.0}"))
                                            .unwrap_or_else(|| "—".into())
                                    );
                                    ui.label(
                                        RichText::new(stats).monospace().size(10.0).color(MUTED),
                                    );
                                })
                                .response
                                .interact(Sense::click());
                            response.context_menu(|ui| self.player_menu(ui, player));
                            ui.menu_button("⌄", |ui| self.player_menu(ui, player));
                        });
                    });
                }
                if players.is_empty() {
                    small(ui, "No recorded players on this team.");
                }
                ui.separator();
                let all = !players.is_empty()
                    && players
                        .iter()
                        .all(|p| self.selected_players.contains(&p.key()));
                if ui
                    .button(if all {
                        "Clear team selection"
                    } else {
                        "Select all"
                    })
                    .clicked()
                {
                    for player in players {
                        if all {
                            self.selected_players.remove(&player.key());
                        } else {
                            self.selected_players.insert(player.key());
                        }
                    }
                }
            });
    }

    fn player_menu(&mut self, ui: &mut Ui, player: &Player) {
        ui.label(RichText::new(&player.name).strong());
        let Some(steam) = &player.steam_id else {
            small(ui, "SteamID64 not recorded. No profile is invented.");
            return;
        };
        if ui.button("Copy SteamID64").clicked() {
            ui.ctx().copy_text(steam.clone());
            ui.close();
        }
        for (name, prefix) in [
            ("Open Steam profile", "https://steamcommunity.com/profiles/"),
            ("Open cswatch.in profile", "https://cswatch.in/player/"),
            ("Open Leetify profile", "https://leetify.com/app/profile/"),
            ("Open csstats.gg profile", "https://csstats.gg/player/"),
        ] {
            if ui.button(name).clicked() {
                if let Err(e) = open::that(format!("{prefix}{steam}")) {
                    self.error(e);
                }
                ui.close();
            }
        }
    }

    fn command_bar(&mut self, ui: &mut Ui, game: &MatchData) {
        ui.add_space(6.0);
        card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                if self.selected_players.is_empty() {
                    ui.label(
                        RichText::new("Select players you would like to hear in the demo…")
                            .color(MUTED)
                            .size(12.0),
                    );
                } else {
                    let selected: Vec<_> = game
                        .players
                        .iter()
                        .filter(|p| self.selected_players.contains(&p.key()))
                        .collect();
                    for player in selected {
                        if ui
                            .add(
                                Button::new(
                                    RichText::new(format!("{}  ×", player.name))
                                        .size(11.0)
                                        .color(ACCENT),
                                )
                                .fill(SOFT),
                            )
                            .clicked()
                        {
                            self.selected_players.remove(&player.key());
                        }
                    }
                }
                if ui
                    .add_enabled(!self.selected_players.is_empty(), primary("Copy Cmd."))
                    .clicked()
                {
                    let selected: Vec<_> = game
                        .players
                        .iter()
                        .filter(|p| self.selected_players.contains(&p.key()))
                        .collect();
                    if selected.iter().any(|p| p.entity_index.is_none()) {
                        self.error("A selected player has no recorded controller index");
                    } else {
                        match voice_command(selected.iter().filter_map(|p| p.entity_index)) {
                            Ok(command) => {
                                ui.ctx().copy_text(command.clone());
                                self.info(format!("Copied: {command}"));
                            }
                            Err(e) => self.error(e),
                        }
                    }
                }
                if !self.selected_players.is_empty() && ui.small_button("Clear").clicked() {
                    self.selected_players.clear();
                }
            });
        });
    }

    fn timeline(&mut self, ui: &mut Ui, game: &MatchData) {
        ui.add_space(6.0);
        card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                eyebrow(ui, "Round timeline");
                small(ui, format!("{} observed rounds · A / B", game.rounds.len()));
            });
            if game.rounds.is_empty() {
                small(ui, "Unavailable — no completed-round transitions recorded.");
                return;
            }
            let width = ((ui.available_width() - (game.rounds.len() - 1) as f32 * 3.0)
                / game.rounds.len() as f32)
                .max(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for round in &game.rounds {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(width, 24.0), Sense::click());
                    let color = match round.winner {
                        Some(0) => ACCENT,
                        Some(1) => BLUE,
                        _ => MUTED,
                    };
                    ui.painter()
                        .rect_filled(rect, 2, color.gamma_multiply(0.25));
                    ui.painter()
                        .line_segment([rect.left_top(), rect.right_top()], Stroke::new(2.0, color));
                    if width > 14.0 {
                        ui.painter().text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            round.number,
                            FontId::monospace(9.0),
                            SECONDARY,
                        );
                    }
                    if response.clicked() {
                        ui.ctx()
                            .copy_text(format!("demo_gototick {}", round.start_tick));
                        self.info(format!("Copied jump command for round {}", round.number));
                    }
                    response.on_hover_text(format!(
                        "Round {} · tick {} → {}\nClick to copy demo_gototick",
                        round.number, round.start_tick, round.end_tick
                    ));
                }
            });
        });
    }

    fn annotations(&mut self, ui: &mut Ui, demo: &Demo) {
        ui.add_space(6.0);
        card().show(ui, |ui| {
            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                eyebrow(ui, "Tags");
                ui.horizontal_wrapped(|ui| {
                    for tag in &demo.tags {
                        if ui
                            .button(RichText::new(format!("{tag} ×")).color(ACCENT).size(11.0))
                            .clicked()
                        {
                            if let Some(d) = self
                                .store
                                .as_mut()
                                .and_then(|s| s.library.demos.get_mut(&demo.id))
                            {
                                d.tags.retain(|t| t != tag);
                            }
                            self.mark_dirty();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    let response = ui.add(
                        TextEdit::singleline(&mut self.tag_input)
                            .hint_text("+ add tag")
                            .desired_width(120.0),
                    );
                    if ui.small_button("Add").clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    {
                        let tag = self.tag_input.trim().chars().take(60).collect::<String>();
                        if !tag.is_empty() {
                            if let Some(d) = self
                                .store
                                .as_mut()
                                .and_then(|s| s.library.demos.get_mut(&demo.id))
                                && !d.tags.iter().any(|t| t.eq_ignore_ascii_case(&tag))
                            {
                                d.tags.push(tag);
                            }
                            self.tag_input.clear();
                            self.mark_dirty();
                        }
                    }
                });
                let ui = &mut columns[1];
                eyebrow(ui, "Notes");
                if let Some(d) = self
                    .store
                    .as_mut()
                    .and_then(|s| s.library.demos.get_mut(&demo.id))
                {
                    let changed = ui
                        .add(
                            TextEdit::multiline(&mut d.note)
                                .hint_text("Clutches, suspicious moments, tactics…")
                                .desired_width(f32::INFINITY)
                                .desired_rows(3),
                        )
                        .changed();
                    if changed {
                        self.mark_dirty();
                    }
                }
            });
        });
    }

    fn voice(&mut self, ui: &mut Ui, demo: &Demo) {
        ui.horizontal(|ui| {
            if ui.button("‹  Back to match").clicked() {
                self.voice_open = false;
                self.playback = None;
            }
            ui.vertical(|ui| {
                eyebrow(ui, "Voice extractor");
                ui.label(RichText::new(demo.filename()).monospace().size(11.0));
            });
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                small(ui, "WAV · 48 kHz · mono · 16-bit PCM");
            });
        });
        ui.add_space(12.0);
        let players = demo
            .parsed
            .as_ref()
            .map(|g| g.players.clone())
            .unwrap_or_default();
        let height = ui.available_height() - 50.0;
        let left_width = 195.0;
        Panel::left("voice_players")
            .exact_size(left_width)
            .frame(card())
            .show(ui, |ui| {
                eyebrow(ui, "Playerlist");
                ui.separator();
                ScrollArea::vertical()
                    .id_salt("voice_players_scroll")
                    .max_height((height - 140.0).max(100.0))
                    .show(ui, |ui| {
                        for player in &players {
                            let key = player.key();
                            let selected = self.selected_speaker.as_ref() == Some(&key);
                            if ui
                                .selectable_label(selected, RichText::new(&player.name).strong())
                                .clicked()
                            {
                                self.selected_speaker = Some(key);
                            }
                            small(
                                ui,
                                player
                                    .steam_id
                                    .as_deref()
                                    .unwrap_or("SteamID64 not recorded"),
                            );
                            ui.add_space(4.0);
                        }
                    });
                ui.separator();
                let selected = players
                    .iter()
                    .find(|p| Some(p.key()) == self.selected_speaker);
                if ui
                    .add_enabled(
                        self.active.is_none()
                            && selected.and_then(|p| p.steam_id.as_ref()).is_some(),
                        primary("Extract player"),
                    )
                    .clicked()
                {
                    self.parse_demo(demo, Some(selected.and_then(|p| p.steam_id.clone())));
                }
                if ui
                    .add_enabled(self.active.is_none(), Button::new("Extract all players"))
                    .clicked()
                {
                    self.parse_demo(demo, Some(None));
                }
                small(
                    ui,
                    "Only voice actually recorded in the demo can be extracted.",
                );
            });
        Panel::left("voice_events").default_size(220.0).min_size(170.0).frame(card()).show(ui, |ui| {
            eyebrow(ui, "Selected player");
            ui.separator();
            let steam = players.iter().find(|p| Some(p.key()) == self.selected_speaker).and_then(|p| p.steam_id.as_deref());
            let clips: Vec<_> = self.clips.iter().filter(|c| steam.is_some() && c.steam_id.as_deref() == steam).cloned().collect();
            ScrollArea::vertical().id_salt("voice_events_scroll").max_height(height).show(ui, |ui| {
                if clips.is_empty() { small(ui, "No saved clips for this player. Extract their recorded voice first."); }
                for clip in clips {
                    ui.horizontal(|ui| { ui.label(RichText::new(clip.round.map(|n| format!("Round {n}")).unwrap_or_else(|| "Outside a recorded round".into())).strong()); if ui.small_button("▶").clicked() { self.play(&clip); } });
                    small(ui, format!("Demo {} · {:.2} s", duration(Some(clip.demo_seconds)), clip.duration));
                    ui.separator();
                }
            });
        });
        CentralPanel::default().frame(card()).show(ui, |ui| {
            ui.horizontal(|ui| { eyebrow(ui, "Saved voice audio"); small(ui, format!("{} files", self.clips.len())); });
            ui.add(TextEdit::singleline(&mut self.clip_query).hint_text("Search clips by player…").desired_width(f32::INFINITY));
            ui.separator();
            let clips: Vec<_> = self.clips.iter().filter(|c| c.player.to_lowercase().contains(&self.clip_query.to_lowercase())).cloned().collect();
            ScrollArea::vertical().id_salt("voice_saved_scroll").max_height(height).show(ui, |ui| {
                if clips.is_empty() {
                    ui.add_space(35.0); ui.vertical_centered(|ui| { ui.label(RichText::new("No clips to show").strong().color(SECONDARY)); small(ui, "Extract a player or the whole demo. No waveform is shown until audio exists."); });
                }
                for clip in &clips {
                    ui.push_id((&clip.id, &clip.path), |ui| {
                        ui.horizontal(|ui| {
                            let playing = self.playback.as_ref().is_some_and(|p| p.clip_id == clip.id);
                            if ui.button(if playing { "■" } else { "▶" }).clicked() { if playing { self.playback = None; } else { self.play(clip); } }
                            ui.vertical(|ui| {
                                ui.label(RichText::new(format!("{} · {} · {}", clip.player, clip.round.map(|n| format!("Round {n}")).unwrap_or_else(|| "Unnumbered".into()), duration(Some(clip.demo_seconds)))).size(11.0).strong());
                                let width = ui.available_width().clamp(80.0, 260.0);
                                let (rect, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
                                let progress = self.playback.as_ref().filter(|p| p.clip_id == clip.id).map(|p| (p.position() / clip.duration).clamp(0.0, 1.0) as f32).unwrap_or(0.0);
                                let n = clip.peaks.len().max(1);
                                for (i, peak) in clip.peaks.iter().enumerate() {
                                    let x = rect.left() + (i as f32 + 0.5) / n as f32 * rect.width();
                                    let h = peak.clamp(0.0, 1.0) * rect.height() * 0.5;
                                    ui.painter().line_segment([pos2(x, rect.center().y - h), pos2(x, rect.center().y + h)], Stroke::new(2.0, if i as f32 / n as f32 <= progress && playing { ACCENT } else { MUTED }));
                                }
                            });
                            ui.vertical(|ui| { small(ui, format!("{:.2}s", clip.duration)); small(ui, "WAV"); });
                            if ui.small_button("▱").on_hover_text("Reveal WAV in folder").clicked() && let Err(e) = files::reveal(&clip.path) { self.error(e); }
                        });
                        ui.separator();
                    });
                }
            });
        });
    }

    fn play(&mut self, clip: &Clip) {
        self.playback = None;
        match Playback::play(clip) {
            Ok(playback) => self.playback = Some(playback),
            Err(e) => self.error(format!("Playback failed: {e:#}")),
        }
    }

    pub(super) fn dialogs(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut open = true;
        let mut done = false;
        let title = match &dialog {
            Dialog::Settings => "Settings",
            Dialog::About => "About SmallDemoManager",
            Dialog::InstallUpdate => "Install update and restart?",
            Dialog::Rename { to_cs2: true, .. } => "Transfer demo to CS2",
            Dialog::Rename { .. } => "Rename demo",
            Dialog::RemoveFolder(_) => "Stop watching folder?",
            Dialog::Trash(_) => "Move demo to Recycle Bin?",
        };
        let initial_height = if matches!(dialog, Dialog::Settings) {
            (ctx.viewport_rect().height() - 160.0).clamp(300.0, 700.0)
        } else {
            340.0
        };
        Window::new(title).open(&mut open).collapsible(false).resizable(false).default_width(520.0).default_height(initial_height).max_height((ctx.viewport_rect().height() - 80.0).max(300.0)).vscroll(true).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            match &mut dialog {
                Dialog::About => {
                    ui.label(RichText::new("Small Demo Manager").size(22.0).strong().color(ACCENT));
                    small(ui, format!("v{} · native Rust · personal use", env!("CARGO_PKG_VERSION")));
                    ui.separator();
                    ui.label("Originally created by Pythaeus. Continued by darkcr0w in his absence, respecting his work and keeping his legacy alive.");
                    ui.label("Special thanks to KEROVSKI, HaiX, LobaCS2, neokCS, Throw, and everyone testing builds in the community.");
                    ui.separator();
                    for (label, url) in [("GitHub repository", "https://github.com/thedarkcr0w/Small-Demo-Manager"), ("Support darkcr0w", "https://ko-fi.com/darkcr0w"), ("Support Pythaeus", "https://ko-fi.com/pythaeus")] { if ui.button(label).clicked() && let Err(e) = open::that(url) { self.error(e); } }
                    small(ui, "Native UI: egui/eframe · Parser: source2-demo · Audio: libopus / rodio. See LICENSE and THIRD-PARTY-NOTICES.md.");
                    if ui.button("Close").clicked() { done = true; }
                }
                Dialog::InstallUpdate => {
                    let version = self.updater.prepared.as_ref().map(|p| p.version.as_str()).unwrap_or("—");
                    ui.label(format!("Install version {version} from the verified GitHub release?"));
                    ui.label("The app will save your library, close, install the update, and reopen this demo. Existing app files are backed up; failed installation or startup restores the previous files. Demos, notes, and saved audio are not replaced.");
                    small(ui, "The app folder must be writable. No administrator prompt or external updater script is used.");
                    ui.horizontal(|ui| {
                        if ui.button("Not now").clicked() { done = true; }
                        if ui.add_enabled(self.active.is_none() && self.updater.task.is_none() && self.updater.prepared.is_some(), primary("Restart & install")).clicked() {
                            done = self.begin_update_install(ctx);
                        }
                    });
                }
                Dialog::Settings => {
                    eyebrow(ui, "CS2 demo destination");
                    small(ui, "Choose the game's game/csgo directory (or its replays subfolder).");
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut self.cs2_input).desired_width(360.0).hint_text("…/Counter-Strike Global Offensive/game/csgo"));
                        if ui.button("Browse…").clicked() && let Some(path) = rfd::FileDialog::new().pick_folder() { self.cs2_input = path.to_string_lossy().into_owned(); }
                    });
                    if ui.add_enabled(self.active.is_none(), Button::new("Auto-detect Steam install")).clicked() { self.send(Job::Detect, "Locating CS2 in Steam's libraries…", None); }
                    if !self.cs2_input.trim().is_empty() {
                        match steam::validate_destination(&PathBuf::from(self.cs2_input.trim())) {
                            Ok(()) => { ui.colored_label(Color32::from_rgb(34, 197, 94), "CS2 game directory verified"); }
                            Err(e) => { ui.colored_label(RED, e.to_string()); }
                        }
                    }
                    ui.separator();
                    ui.checkbox(&mut self.settings_draft.move_files, "Move instead of copying to CS2 (removes original after verification)");
                    ui.add_enabled_ui(self.settings_draft.move_files, |ui| { ui.checkbox(&mut self.settings_draft.backup_before_move, "Keep a verified backup before moving"); });
                    small(ui, "Backups: %LocalAppData%/Small-Demo-Manager/Backups.native");
                    ui.checkbox(&mut self.settings_draft.background_parse, "Parse the library in the background for player / SteamID search");
                    ui.separator();
                    self.update_settings(ui);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() { done = true; }
                        if ui.add(primary("Save settings")).clicked() {
                            let path = PathBuf::from(self.cs2_input.trim());
                            if !path.as_os_str().is_empty() && let Err(e) = steam::validate_destination(&path) { self.error(e); return; }
                            self.settings_draft.cs2_path = path;
                            if let Some(store) = &mut self.store { store.library.settings = self.settings_draft.clone(); }
                            self.persist(); done = true;
                        }
                    });
                }
                Dialog::Rename { id, name, to_cs2 } => {
                    ui.label("Filename (without .dem)");
                    ui.add(TextEdit::singleline(name).desired_width(480.0));
                    if let Err(e) = files::demo_filename(name) { ui.colored_label(RED, e.to_string()); }
                    if *to_cs2 {
                        let settings = self.store.as_ref().map(|s| s.library.settings.clone()).unwrap_or_default();
                        small(ui, format!("Destination: {}", settings.cs2_path.display()));
                        small(ui, if settings.move_files { if settings.backup_before_move { "Move · verified backup enabled · SHA-256 checked before original is removed" } else { "Move · backup disabled · SHA-256 checked before original is removed" } } else { "Copy · original stays where it is · SHA-256 checked" });
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() { done = true; }
                        if ui.add_enabled(self.active.is_none() && files::demo_filename(name).is_ok(), primary("Confirm")).clicked() {
                            let Some(store) = &self.store else { return; };
                            let Some(demo) = store.library.demos.get(id).cloned() else { return; };
                            if *to_cs2 {
                                let settings = store.library.settings.clone();
                                if let Err(e) = steam::validate_destination(&settings.cs2_path) { self.error(format!("{e}. Configure this in Settings first")); return; }
                                let filename = match files::demo_filename(name) { Ok(name) => name, Err(e) => { self.error(e); return; } };
                                let backup = (settings.move_files && settings.backup_before_move).then(|| store.root.join("Backups.native"));
                                self.send(Job::Transfer { id: id.clone(), source: demo.path, destination: settings.cs2_path.join(filename), moving: settings.move_files, backup }, "Transferring and verifying SHA-256…", None);
                            } else { self.send(Job::Rename { id: id.clone(), source: demo.path, name: name.clone() }, "Renaming and verifying demo…", None); }
                            done = true;
                        }
                    });
                }
                Dialog::RemoveFolder(id) => {
                    ui.label("Demo files, saved audio, notes, and tags stay on disk. You can add this folder again later.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() { done = true; }
                        if ui.add_enabled(self.active.is_none(), Button::new("Stop watching")).clicked() {
                            if let Some(store) = &mut self.store {
                                store.library.folders.retain(|f| &f.id != id);
                                for demo in store.library.demos.values_mut().filter(|d| &d.folder_id == id) { demo.available = false; }
                            }
                            self.view = View::All; self.refresh_watcher(); self.persist(); self.rescan_pending = true; done = true;
                        }
                    });
                }
                Dialog::Trash(id) => {
                    ui.label("The demo will be moved to the Windows Recycle Bin, not permanently deleted. Saved voice clips and annotations stay intact.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() { done = true; }
                        if ui.add_enabled(self.active.is_none(), Button::new(RichText::new("Move to Recycle Bin").color(RED))).clicked() {
                            if let Some(demo) = self.store.as_ref().and_then(|s| s.library.demos.get(id)).cloned() { self.send(Job::Trash { id: id.clone(), path: demo.path }, "Moving demo to the Recycle Bin…", None); }
                            done = true;
                        }
                    });
                }
            }
        });
        if !done && open && !ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.dialog = Some(dialog);
        }
    }
}

fn ellipsize(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        text.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}
