use super::*;

/// Colour of team `index` (0 = A, 1 = B) for text and marks on the light page.
fn team_color(index: usize) -> Color32 {
    if index == 0 { SIGNAL_TEXT } else { TEAM_B }
}

/// Height of the title band (logo, version, window controls).
const TITLE_BAR: f32 = 54.0;

/// What the "Muted" badge means, for its tooltip and the player menu.
const MUTED_HINT: &str = "Community muted: you won't hear this player when you watch the demo in CS2. The voice extractor still gets their voice.";

/// A player's name in `font`, cut short to leave room for a "Muted" badge when the
/// demo recorded a communication-abuse mute for them.
fn player_name(ui: &mut Ui, player: &Player, font: FontId) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let room = if player.comm_muted {
            badge_width(ui, "Muted") + 6.0
        } else {
            0.0
        };
        let name = one_line(ui, &player.name, font, INK, ui.available_width() - room);
        ui.add(Label::new(name).selectable(false));
        if player.comm_muted {
            badge(ui, "Muted").on_hover_text(MUTED_HINT);
        }
    });
}

/// The full window width for edge-to-edge lines inside padded panels.
fn full_width(ui: &Ui) -> egui::Rangef {
    ui.ctx().content_rect().x_range()
}

impl App {
    pub(super) fn chrome(&mut self, ui: &mut Ui) {
        Panel::top("titlebar")
            .exact_size(TITLE_BAR)
            .frame(Frame::new().fill(GROUND).inner_margin(Margin {
                left: 16,
                right: 8,
                top: 0,
                bottom: 0,
            }))
            .show(ui, |ui| {
                let bar = ui.max_rect();
                ui.painter()
                    .hline(full_width(ui), bar.bottom() + 0.5, Stroke::new(1.0, EDGE));
                ui.horizontal_centered(|ui| {
                    logo(ui);
                    ui.add_space(4.0);
                    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
                    let g = ui.painter().layout_no_wrap(version, mono(11.5), LABEL);
                    let (r, _) = ui.allocate_exact_size(g.size() + vec2(14.0, 8.0), Sense::hover());
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(radius::TINY), IDLE);
                    ui.painter().galley(r.center() - g.size() / 2.0, g, LABEL);
                });
            });
        self.window_controls(ui.ctx());
    }

    /// Moving, maximising, minimising and closing the window. The whole title band drags
    /// the window and the three buttons sit on top of it. They live in a layer above the
    /// dialogs: a modal dialog must never leave a borderless window stuck in place.
    fn window_controls(&self, ctx: &Context) {
        let band = Rect::from_min_size(
            ctx.content_rect().min,
            vec2(ctx.content_rect().width(), TITLE_BAR),
        );
        Area::new(Id::unique("window_controls"))
            .order(Order::Tooltip)
            .fixed_pos(band.min)
            .fade_in(false)
            .show(ctx, |ui| {
                let (band, drag) = ui.allocate_exact_size(band.size(), Sense::click_and_drag());
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                if drag.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
                let mut ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(band.shrink2(vec2(8.0, 0.0)))
                        .layout(egui::Layout::right_to_left(Align::Center)),
                );
                ui.spacing_mut().item_spacing.x = 2.0;
                let size = vec2(42.0, 32.0);
                if ghost_icon(&mut ui, Glyph::Cross, size, INK, true)
                    .on_hover_text("Close")
                    .clicked()
                {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                if ghost_icon(&mut ui, Glyph::Maximize, size, INK, false)
                    .on_hover_text(if maximized { "Restore" } else { "Maximize" })
                    .clicked()
                {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
                if ghost_icon(&mut ui, Glyph::Minimize, size, INK, false)
                    .on_hover_text("Minimize")
                    .clicked()
                {
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                }
            });
    }

    pub(super) fn status(&mut self, ui: &mut Ui) {
        Panel::bottom("status")
            .exact_size(30.0)
            .frame(
                Frame::new()
                    .fill(GROUND)
                    .inner_margin(Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter()
                    .hline(full_width(ui), r.top() - 0.5, Stroke::new(1.0, EDGE));
                let active = self
                    .active
                    .as_ref()
                    .map(|a| (a.label.clone(), a.progress, a.cancel.is_some()));
                ui.horizontal_centered(|ui| {
                    if let Some((label, progress, cancellable)) = active {
                        spinner(ui);
                        ui.label(RichText::new(label).font(mono(12.0)).color(INK));
                        if let Some(p) = progress {
                            ui.add_space(6.0);
                            progress_bar(ui, p, 110.0);
                        }
                        if cancellable && small_link(ui, "Cancel").clicked() {
                            self.cancel_active();
                        }
                    } else {
                        let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                        ui.painter().circle_filled(
                            dot.center(),
                            3.5,
                            if self.watcher.is_some() { OK } else { MUTED },
                        );
                        ui.label(
                            RichText::new(if self.watcher.is_some() {
                                "Watching folders · ready"
                            } else {
                                "Manual rescan available"
                            })
                            .font(mono(12.0))
                            .color(INK),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(if self.dirty.is_some() {
                                "Saving annotations…"
                            } else {
                                "Rust · Source 2 parser 0.5.10 · WAV / Opus"
                            })
                            .font(mono(12.0))
                            .color(LABEL),
                        );
                    });
                });
            });
    }

    /// Messages float above the status bar, bottom right. Errors get a red edge; every
    /// message stays until it is dismissed, so nothing important disappears unread.
    pub(super) fn notice_card(&mut self, ctx: &Context) {
        let Some((text, is_error)) = self.notice.clone() else {
            return;
        };
        let screen = ctx.content_rect();
        let width = (screen.width() - 80.0).clamp(220.0, 430.0);
        Area::new(Id::unique("notice_card"))
            .order(Order::Tooltip)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-18.0, -44.0))
            .interactable(true)
            .show(ctx, |ui| {
                notice_frame(is_error).show(ui, |ui| {
                    ui.set_width(width);
                    ui.horizontal_top(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(18.0, 20.0), Sense::hover());
                        paint_glyph(
                            ui.painter(),
                            if is_error { Glyph::Info } else { Glyph::Check },
                            r.center(),
                            15.0,
                            if is_error { SIGNAL_TEXT } else { OK },
                        );
                        ui.vertical(|ui| {
                            ScrollArea::vertical()
                                .id_salt("notice_scroll")
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        Label::new(
                                            RichText::new(&text).font(medium(13.5)).color(INK),
                                        )
                                        .wrap(),
                                    );
                                });
                            ui.add_space(4.0);
                            if small_link(ui, "Dismiss").clicked() {
                                self.notice = None;
                            }
                        });
                    });
                });
            });
    }

    pub(super) fn sidebar(&mut self, ui: &mut Ui) {
        Panel::left("sidebar")
            .exact_size(236.0)
            .resizable(false)
            .frame(Frame::new().fill(PAGE).inner_margin(Margin {
                left: 12,
                right: 12,
                top: 16,
                bottom: 12,
            }))
            .show(ui, |ui| {
                let cutoff = chrono::Utc::now().timestamp().saturating_sub(7 * 86400) as u128
                    * 1_000_000_000;
                let mut total = 0;
                let mut recent = 0;
                let mut favorites = 0;
                let mut folder_counts = HashMap::new();
                let mut tags = BTreeSet::new();
                if let Some(store) = &self.store {
                    for demo in store.library.demos.values().filter(|d| d.available) {
                        total += 1;
                        recent += usize::from(demo.stamp.modified_ns >= cutoff);
                        favorites += usize::from(demo.favorite);
                        *folder_counts.entry(demo.folder_id.clone()).or_insert(0) += 1;
                        tags.extend(demo.tags.iter().cloned());
                    }
                }
                ui.spacing_mut().item_spacing.y = 2.0;
                section(ui, "Library");
                ui.add_space(4.0);
                for (view, glyph, title, count) in [
                    (View::All, Glyph::Rows, "All demos", total),
                    (View::Recent, Glyph::Clock, "Recent", recent),
                    (View::Favorites, Glyph::Star, "Favorites", favorites),
                ] {
                    let selected = self.view == view;
                    if nav_item(ui, title, Some(glyph), title, None, Some(count), selected)
                        .clicked()
                    {
                        self.view = view;
                    }
                }
                ui.add_space(space::L);
                ui.horizontal(|ui| {
                    section(ui, "Watched folders");
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if icon_key(ui, Glyph::Plus, true)
                            .on_hover_text("Watch a folder")
                            .clicked()
                        {
                            self.pick_folder();
                        }
                    });
                });
                ui.add_space(2.0);
                let folders = self
                    .store
                    .as_ref()
                    .map(|s| s.library.folders.clone())
                    .unwrap_or_default();
                ScrollArea::vertical()
                    .id_salt("folder_sidebar")
                    .max_height((ui.available_height() - 250.0).max(100.0))
                    .show(ui, |ui| {
                        for folder in &folders {
                            let selected = self.view == View::Folder(folder.id.clone());
                            let count = folder_counts.get(&folder.id).copied().unwrap_or(0);
                            let path = files::display_path(&folder.path);
                            let response = nav_item(
                                ui,
                                ("folder", &folder.id),
                                Some(Glyph::Folder),
                                &folder.label,
                                Some(path.as_ref()),
                                Some(count),
                                selected,
                            );
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
                            hint(ui, "Add your CS2, FACEIT, or downloaded demo folder.");
                        }
                    });
                ui.add_space(space::L);
                section(ui, "Filter by tag");
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    for tag in &tags {
                        if chip(ui, tag, self.tag_filter.contains(tag), false).clicked()
                            && !self.tag_filter.remove(tag)
                        {
                            self.tag_filter.insert(tag.clone());
                        }
                    }
                });
                if tags.is_empty() {
                    hint(ui, "No tags yet — add them to a demo.");
                }
                ui.with_layout(egui::Layout::bottom_up(Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(16.0, 18.0), Sense::hover());
                        paint_glyph(ui.painter(), Glyph::Refresh, r.center(), 12.0, INK);
                        if small_link(ui, "Rescan all folders").clicked() {
                            self.rescan_pending = true;
                        }
                    });
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        let half = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                        if icon_button_wide(
                            ui,
                            Glyph::Gear,
                            "Settings",
                            Tone::Secondary,
                            half,
                            true,
                        )
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
                        if icon_button_wide(ui, Glyph::Info, "About", Tone::Secondary, half, true)
                            .clicked()
                        {
                            self.dialog = Some(Dialog::About);
                        }
                    });
                });
            });
    }

    pub(super) fn toolbar(&mut self, ui: &mut Ui) {
        Panel::top("toolbar")
            .exact_size(62.0)
            .frame(
                Frame::new()
                    .fill(PAGE)
                    .inner_margin(Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().hline(
                    r.x_range().expand(16.0),
                    r.bottom() + 0.5,
                    Stroke::new(1.0, EDGE),
                );
                ui.horizontal_centered(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        // In a narrow window the main action shrinks to its icon, so the
                        // search box keeps a usable width.
                        let compact = ui.available_width() < 860.0;
                        let add = icon_button(
                            ui,
                            Glyph::Plus,
                            if compact { "" } else { "Add folder" },
                            Tone::Primary,
                            true,
                        );
                        if add.clicked() {
                            self.pick_folder();
                        }
                        if compact {
                            add.on_hover_text("Add folder");
                        }
                        if button(ui, "Import…", Tone::Secondary, self.active.is_none()).clicked()
                            && let Some(paths) = rfd::FileDialog::new()
                                .add_filter("CS2 demos and ZIP", &["dem", "zip"])
                                .pick_files()
                        {
                            self.import(paths);
                        }
                        ui.add_space(space::S);
                        if icon_key(ui, Glyph::Sidebar, true)
                            .on_hover_text(if self.list_collapsed {
                                "Show the demo list"
                            } else {
                                "Hide the demo list"
                            })
                            .clicked()
                        {
                            self.list_collapsed = !self.list_collapsed;
                        }
                        let layout = self
                            .store
                            .as_ref()
                            .map(|s| s.library.settings.layout)
                            .unwrap_or_default();
                        let layouts = [
                            LibraryLayout::Table,
                            LibraryLayout::Cards,
                            LibraryLayout::Hybrid,
                        ];
                        let current = layouts.iter().position(|l| *l == layout).unwrap_or(0);
                        if let Some(i) = icon_segmented(
                            ui,
                            "layout",
                            &[
                                (Glyph::Rows, "Table"),
                                (Glyph::Grid, "Cards"),
                                (Glyph::Split, "Split view"),
                            ],
                            current,
                        ) {
                            if let Some(store) = &mut self.store {
                                store.library.settings.layout = layouts[i];
                            }
                            self.list_collapsed = false;
                            self.mark_dirty();
                        }
                        ui.add_space(space::M);
                        let sorts = [
                            (Sort::Newest, "Newest first"),
                            (Sort::Oldest, "Oldest first"),
                            (Sort::Largest, "Largest"),
                            (Sort::Longest, "Longest"),
                            (Sort::Map, "Map A–Z"),
                        ];
                        let sort_name = sorts
                            .iter()
                            .find(|(s, _)| *s == self.sort)
                            .map_or("", |(_, n)| n);
                        let sort = inline_picker(ui, "", sort_name, 14.0);
                        Popup::menu(&sort).show(|ui| {
                            ui.set_min_width(160.0);
                            for (sort, name) in sorts {
                                ui.selectable_value(&mut self.sort, sort, name);
                            }
                        });
                        ui.add_space(space::S);
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
                        let map_label = if self.map_filter.is_empty() {
                            "All".into()
                        } else {
                            map_name(&self.map_filter)
                        };
                        let map = inline_picker(ui, "Map", &map_label, 14.0);
                        Popup::menu(&map).show(|ui| {
                            ui.set_min_width(170.0);
                            ui.selectable_value(&mut self.map_filter, String::new(), "All maps");
                            for map in maps {
                                ui.selectable_value(
                                    &mut self.map_filter,
                                    map.clone(),
                                    map_name(&map),
                                );
                            }
                        });
                        ui.add_space(space::M);
                        let search = search_field(
                            ui,
                            Id::unique("library_search"),
                            &mut self.query,
                            if ui.available_width() >= 400.0 {
                                "Search demos, players, SteamIDs, tags…   Ctrl+K"
                            } else if ui.available_width() >= 250.0 {
                                "Search demos, players, tags…"
                            } else {
                                "Search…"
                            },
                        );
                        if ui.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::K)) {
                            search.request_focus();
                        }
                    });
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
        Frame::new()
            .inner_margin(Margin {
                left: 16,
                right: 16,
                top: 14,
                bottom: 10,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.add(Label::new(RichText::new(title).font(bold(16.0)).color(INK)).truncate());
                    ui.label(
                        RichText::new(format!(
                            "{} demo{}",
                            ids.len(),
                            if ids.len() == 1 { "" } else { "s" }
                        ))
                        .font(mono(12.0))
                        .color(MUTED),
                    );
                });
            });
        divider(ui);
        let scroll = ScrollArea::vertical()
            .id_salt("demo_library")
            .auto_shrink([false, false]);
        // Rows borrow their demo and report what was clicked; it is applied afterwards.
        let mut action = None;
        let demos = self.store.as_ref().map(|s| &s.library.demos);
        let selected = self.selected.as_deref();
        if ids.is_empty() {
            scroll.show(ui, |ui| {
                ui.add_space(36.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("No demos to show")
                            .font(bold(16.0))
                            .color(INK),
                    );
                    ui.add_space(2.0);
                    hint(ui, "Add a folder, import a demo, or clear your filters.");
                    ui.add_space(space::S);
                    if small_link(ui, "Clear filters").clicked() {
                        self.query.clear();
                        self.map_filter.clear();
                        self.tag_filter.clear();
                        self.view = View::All;
                    }
                });
            });
        } else if cards {
            const CARD: Vec2 = vec2(228.0, 122.0);
            let gap = ui.spacing().item_spacing.x + 4.0;
            // Reserve scrollbar space before choosing the number of columns.
            let columns = ((ui.available_width() - 28.0 + gap) / (CARD.x + gap))
                .floor()
                .max(1.0) as usize;
            scroll.show_rows(ui, CARD.y, ids.len().div_ceil(columns), |ui, rows| {
                ui.add_space(4.0);
                for row in rows {
                    ui.horizontal_top(|ui| {
                        ui.add_space(12.0);
                        ui.spacing_mut().item_spacing.x = gap;
                        for id in ids.iter().skip(row * columns).take(columns) {
                            if let Some(demo) = demos.and_then(|d| d.get(id)) {
                                let is_selected = selected == Some(id.as_str());
                                action = demo_card(ui, demo, is_selected, CARD).or(action.take());
                            }
                        }
                    });
                }
            });
        } else {
            let height = if ui.available_width() > 560.0 {
                60.0
            } else {
                78.0
            };
            scroll.show_rows(
                ui,
                height - ui.spacing().item_spacing.y,
                ids.len(),
                |ui, rows| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for id in &ids[rows] {
                        if let Some(demo) = demos.and_then(|d| d.get(id)) {
                            let is_selected = selected == Some(id.as_str());
                            action = demo_row(ui, demo, is_selected, height).or(action.take());
                        }
                    }
                },
            );
        }
        match action {
            Some(LibraryAction::Select(id)) => self.select(id),
            Some(LibraryAction::ToggleFavorite(id)) => {
                if let Some(d) = self
                    .store
                    .as_mut()
                    .and_then(|s| s.library.demos.get_mut(&id))
                {
                    d.favorite = !d.favorite;
                }
                self.mark_dirty();
            }
            None => {}
        }
    }
}

/// What a click in the demo list asks for.
enum LibraryAction {
    Select(String),
    ToggleFavorite(String),
}

/// One line of the table / split list. Its ids come from the demo, not from its place in
/// the list, so hover and selection stay with the demo while the list scrolls.
fn demo_row(ui: &mut Ui, demo: &Demo, selected: bool, height: f32) -> Option<LibraryAction> {
    let wide = ui.available_width() > 560.0;
    let id = Id::unique(("demo-row", &demo.id));
    let (_, rect) = ui.allocate_space(vec2(ui.available_width(), height));
    let response = ui.interact(rect, id, Sense::click());
    let star_rect =
        Rect::from_min_size(pos2(rect.left() + 10.0, rect.top() + 9.0), vec2(24.0, 24.0));
    // Added after the row, so the star wins where the two overlap.
    let star = ui
        .interact(star_rect, id.with("favorite"), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    let action = if star.clicked() {
        Some(LibraryAction::ToggleFavorite(demo.id.clone()))
    } else if response.clicked() {
        Some(LibraryAction::Select(demo.id.clone()))
    } else {
        None
    };
    if !ui.is_rect_visible(rect) {
        return action;
    }
    let ctx = ui.ctx();
    let hover = motion::flag(
        ctx,
        id.with("hover"),
        response.hovered() || star.hovered(),
        motion::FAST,
    );
    let sel = motion::flag(ctx, id.with("sel"), selected, motion::BASE);
    let star_hover = motion::flag(ctx, id.with("star"), star.hovered(), motion::FAST);
    let painter = ui.painter();
    let row = rect.shrink2(vec2(8.0, 3.0));
    let strength = hover.max(sel);
    if strength > 0.0 {
        painter.rect_filled(
            row,
            CornerRadius::same(radius::CONTROL),
            mix(HOVER, IDLE, sel).gamma_multiply(strength),
        );
    }
    if sel > 0.0 {
        painter.rect_filled(
            Rect::from_min_size(
                pos2(row.left(), row.center().y - 12.0 * sel),
                vec2(3.0, 24.0 * sel),
            ),
            CornerRadius::same(2),
            SIGNAL,
        );
    }
    paint_glyph(
        painter,
        if demo.favorite {
            Glyph::StarFilled
        } else {
            Glyph::Star
        },
        star_rect.center(),
        14.0 + star_hover,
        if demo.favorite {
            SIGNAL
        } else {
            mix(mix(EDGE, MUTED, 0.4), INK, star_hover)
        },
    );
    let game = demo.parsed.as_ref();
    let x = rect.left() + 42.0;
    let text_w = if wide {
        rect.width() - 470.0
    } else {
        rect.right() - x - 14.0
    };
    let filename = one_line(ui, demo.filename(), mono(12.5), INK, text_w);
    painter.galley(pos2(x, rect.top() + 12.0), filename, INK);
    let description = game
        .map(|g| format!("{} · {}", map_name(&g.map), g.server))
        .unwrap_or_else(|| {
            if demo.parse_error.is_some() {
                "Parse failed — select for details".into()
            } else {
                "Click to load match data".into()
            }
        });
    let error = demo.parse_error.is_some();
    let desc = one_line(
        ui,
        description,
        if error { medium(12.5) } else { body(12.5) },
        if error { SIGNAL_TEXT } else { MUTED },
        text_w,
    );
    painter.galley(pos2(x, rect.top() + 32.0), desc, MUTED);
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
        let cy = rect.center().y;
        painter.text(
            pos2(rect.right() - 300.0, cy),
            Align2::RIGHT_CENTER,
            score,
            bold(15.0),
            if game.is_some() { INK } else { MUTED },
        );
        painter.text(
            pos2(rect.right() - 196.0, cy),
            Align2::RIGHT_CENTER,
            duration(game.and_then(|g| g.duration)),
            mono(12.0),
            LABEL,
        );
        painter.text(
            pos2(rect.right() - 92.0, cy),
            Align2::RIGHT_CENTER,
            size(demo.stamp.size),
            mono(12.0),
            MUTED,
        );
        let date =
            chrono::DateTime::from_timestamp((demo.stamp.modified_ns / 1_000_000_000) as i64, 0)
                .map(|d| d.format("%b %d").to_string())
                .unwrap_or_default();
        painter.text(
            pos2(rect.right() - 18.0, cy),
            Align2::RIGHT_CENTER,
            date,
            mono(12.0),
            MUTED,
        );
    } else if let Some(game) = game {
        let text = format!(
            "{}  {} : {}  {}",
            game.teams[0].name,
            number(game.teams[0].score),
            number(game.teams[1].score),
            game.teams[1].name
        );
        let g = one_line(ui, text, body(12.0), LABEL, text_w);
        painter.galley(pos2(x, rect.top() + 52.0), g, LABEL);
    }
    if strength < 1.0 {
        painter.hline(
            row.left() + 6.0..=row.right() - 6.0,
            rect.bottom() - 0.5,
            Stroke::new(1.0, EDGE.gamma_multiply(0.7 * (1.0 - strength))),
        );
    }
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(demo.path.display().to_string());
    action
}

/// One demo in the card grid; ids come from the demo, as in [`demo_row`].
fn demo_card(ui: &mut Ui, demo: &Demo, selected: bool, card: Vec2) -> Option<LibraryAction> {
    let (_, rect) = ui.allocate_space(card);
    let response = ui
        .interact(rect, Id::unique(("demo-card", &demo.id)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    let action = response
        .clicked()
        .then(|| LibraryAction::Select(demo.id.clone()));
    if !ui.is_rect_visible(rect) {
        return action;
    }
    let (hover, press) = hover_press(ui, &response, true);
    let sel = motion::flag(ui.ctx(), response.id.with("sel"), selected, motion::BASE);
    let face = Rect::from_min_size(
        rect.min + vec2(0.0, 2.0 * press - hover * (1.0 - press)),
        vec2(card.x, card.y - 4.0),
    );
    let painter = ui.painter();
    painter.rect_filled(
        Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(card.x, card.y - 4.0)),
        CornerRadius::same(radius::CARD),
        mix(EDGE, SIGNAL_EDGE, sel),
    );
    painter.rect(
        face,
        CornerRadius::same(radius::CARD),
        mix(KEY, HOVER, hover),
        Stroke::new(1.0 + 0.5 * sel, mix(EDGE, SIGNAL, sel)),
        StrokeKind::Inside,
    );
    let inner = face.shrink2(vec2(14.0, 12.0));
    let game = demo.parsed.as_ref();
    let map = one_line(
        ui,
        game.map(|g| map_name(&g.map))
            .unwrap_or_else(|| "Unparsed demo".into()),
        bold(17.0),
        INK,
        inner.width() - 24.0,
    );
    painter.galley(inner.min, map, INK);
    if demo.favorite {
        paint_glyph(
            painter,
            Glyph::StarFilled,
            pos2(inner.right() - 7.0, inner.top() + 10.0),
            14.0,
            SIGNAL,
        );
    }
    let file = one_line(ui, demo.filename(), mono(11.5), MUTED, inner.width());
    painter.galley(pos2(inner.left(), inner.top() + 26.0), file, MUTED);
    match game {
        Some(game) => {
            painter.text(
                pos2(inner.left(), inner.top() + 58.0),
                Align2::LEFT_CENTER,
                format!(
                    "{} : {}",
                    number(game.teams[0].score),
                    number(game.teams[1].score)
                ),
                display(18.0),
                INK,
            );
        }
        None => {
            painter.text(
                pos2(inner.left(), inner.top() + 58.0),
                Align2::LEFT_CENTER,
                if demo.parse_error.is_some() {
                    "Parse failed"
                } else {
                    "Click to load match data"
                },
                body(12.5),
                if demo.parse_error.is_some() {
                    SIGNAL_TEXT
                } else {
                    MUTED
                },
            );
        }
    }
    painter.text(
        pos2(inner.left(), inner.bottom() - 6.0),
        Align2::LEFT_CENTER,
        format!(
            "{} · {}",
            duration(game.and_then(|g| g.duration)),
            size(demo.stamp.size)
        ),
        mono(11.5),
        LABEL,
    );
    response.on_hover_text(demo.path.display().to_string());
    action
}

impl App {
    pub(super) fn details(&mut self, ui: &mut Ui) {
        let demo = self
            .selected
            .as_ref()
            .and_then(|id| self.store.as_ref()?.library.demos.get(id))
            .cloned();
        let Some(demo) = demo.filter(|d| d.available) else {
            let has_demos = self
                .store
                .as_ref()
                .is_some_and(|store| store.library.demos.values().any(|demo| demo.available));
            ui.add_space((ui.available_height() * 0.22).clamp(30.0, 160.0));
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(if has_demos {
                        "Choose a demo to open"
                    } else {
                        "Your demos. Nothing in the way."
                    })
                    .font(display(28.0))
                    .color(INK),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Demos are parsed only when clicked or dropped into the window.")
                        .font(body(15.0))
                        .color(MUTED),
                );
                ui.add_space(space::XL);
                if big_button(
                    ui,
                    Some(Glyph::Plus),
                    "Add your demo folder",
                    Tone::Primary,
                    0.0,
                    true,
                )
                .clicked()
                {
                    self.pick_folder();
                }
                ui.add_space(space::M);
                hint(
                    ui,
                    "Or drop a .dem file or ZIP archive here. Original files are kept.",
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
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), 0.0),
                    egui::Layout::right_to_left(Align::Center),
                    |ui| {
                        let moving = self
                            .store
                            .as_ref()
                            .is_some_and(|s| s.library.settings.move_files);
                        if big_button(
                            ui,
                            Some(Glyph::Play),
                            if moving { "Move to CS2" } else { "Copy to CS2" },
                            Tone::Primary,
                            0.0,
                            self.active.is_none(),
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
                        ui.add_space(space::L);
                        ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                section(ui, "Loaded demo · drop files to import");
                                ui.add(
                                    Label::new(
                                        RichText::new(demo.filename())
                                            .font(bold(20.0))
                                            .color(INK),
                                    )
                                    .truncate(),
                                );
                                ui.add(
                                    Label::new(
                                        RichText::new(files::display_path(&demo.path))
                                            .font(mono(12.0))
                                            .color(MUTED),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(demo.path.display().to_string());
                            });
                        });
                    },
                );
                ui.add_space(space::L);
                if let Some(error) = &demo.parse_error {
                    group(ui, |ui| {
                        section(ui, "Could not read this demo");
                        ui.add_space(4.0);
                        error_text(ui, error);
                        ui.add_space(space::S);
                        if button(ui, "Retry parsing", Tone::Secondary, self.active.is_none())
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
                        hint(ui, warning);
                    }
                } else if demo.parse_error.is_none() {
                    group(ui, |ui| {
                        if self.parse_requested
                            || self.active.as_ref().is_some_and(|active| {
                                active.demo_id.as_ref() == Some(&demo.id)
                            })
                        {
                            ui.horizontal(|ui| {
                                spinner(ui);
                                ui.label(
                                    RichText::new("Reading real match data…")
                                        .font(medium(14.5))
                                        .color(INK),
                                );
                            });
                        } else {
                            hint(
                                ui,
                                "This demo has not been parsed. Other library files are not read automatically.",
                            );
                            ui.add_space(space::S);
                            if button(ui, "Open demo", Tone::Secondary, true).clicked() {
                                self.select(demo.id.clone());
                            }
                        }
                    });
                }
                self.annotations(ui, &demo);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    if icon_button(
                        ui,
                        if demo.favorite {
                            Glyph::StarFilled
                        } else {
                            Glyph::Star
                        },
                        if demo.favorite {
                            "Favorited"
                        } else {
                            "Favorite"
                        },
                        Tone::Secondary,
                        true,
                    )
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
                    if icon_button(ui, Glyph::Folder, "Reveal in folder", Tone::Secondary, true)
                        .clicked()
                        && let Err(e) = files::reveal(&demo.path)
                    {
                        self.error(e);
                    }
                    if button(ui, "Copy playdemo command", Tone::Secondary, true).clicked() {
                        let cs2 = self
                            .store
                            .as_ref()
                            .map(|s| s.library.settings.cs2_path.clone())
                            .unwrap_or_default();
                        ui.ctx().copy_text(steam::play_command(&demo.path, &cs2));
                        self.info("Copied playdemo command");
                    }
                    if button(ui, "Rename", Tone::Secondary, self.active.is_none()).clicked() {
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
                    if button(ui, "Recycle Bin", Tone::Danger, self.active.is_none()).clicked() {
                        self.dialog = Some(Dialog::Trash(demo.id.clone()));
                    }
                });
                ui.add_space(space::M);
                if let Some(game) = &demo.parsed {
                    ui.label(
                        RichText::new(format!(
                            "Game build {} · {} ticks · {} · {}",
                            game.game_build,
                            game.ticks,
                            game.tick_interval
                                .map(|t| format!("{:.2} ticks/s", 1.0 / t))
                                .unwrap_or_else(|| "tick rate unavailable".into()),
                            size(demo.stamp.size)
                        ))
                        .font(mono(11.5))
                        .color(LABEL),
                    );
                }
                ui.add_space(space::L);
            });
    }

    fn match_view(&mut self, ui: &mut Ui, demo: &Demo, game: &MatchData) {
        let gap = 12.0;
        let center_width = 158.0;
        let team_width = (ui.available_width() - center_width - 2.0 * gap) / 2.0;
        if team_width < 210.0 {
            // Too narrow for three columns (split view in a small window): the map
            // becomes a banner and the teams stack under it.
            self.map_tile(ui, game, vec2(ui.available_width(), 200.0));
            ui.add_space(space::M + 3.0);
            self.team(ui, game, 0, 0.0);
            self.team(ui, game, 1, 0.0);
            return;
        }
        let rows = (0..2)
            .map(|t| game.players.iter().filter(|p| p.team == Some(t)).count())
            .max()
            .unwrap_or(0)
            .max(1);
        // Both team cards and the map tile share one height: the taller card's, as
        // measured on the previous frame (an estimate the first time).
        let key = Id::unique(("match-height", &demo.id));
        let estimate = 32.0 + 66.0 + rows as f32 * 52.0 + 46.0;
        let height = ui
            .data(|d| d.get_temp::<f32>(key))
            .unwrap_or(estimate)
            .max(300.0);
        let mut needed = height;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (index, width) in [(0, team_width), (2, center_width), (1, team_width)] {
                let card = ui
                    .allocate_ui_with_layout(
                        vec2(width, height),
                        egui::Layout::top_down(Align::Min),
                        |ui| {
                            if index == 2 {
                                self.map_tile(ui, game, vec2(center_width, height));
                                height
                            } else {
                                self.team(ui, game, index, height)
                            }
                        },
                    )
                    .inner;
                needed = needed.max(card);
            }
        });
        if needed > height + 0.5 {
            ui.data_mut(|d| d.insert_temp(key, needed));
            ui.ctx().request_repaint();
        }
    }

    fn map_tile(&mut self, ui: &mut Ui, game: &MatchData, size: Vec2) {
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let r = CornerRadius::same(radius::CARD);
        ui.painter().rect_filled(
            rect.translate(vec2(0.0, 3.0)),
            r,
            Color32::from_rgb(0x12, 0x12, 0x14),
        );
        ui.painter().rect_filled(rect, r, SCREEN);
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
            // Show the centre of the artwork cropped to the tile's own shape.
            let tex = texture.size_vec2();
            let scale = (size.x / tex.x).max(size.y / tex.y);
            let span = (size / (tex * scale)).min(Vec2::splat(1.0));
            let uv = Rect::from_center_size(pos2(0.5, 0.5), span);
            Image::new((texture.id(), size))
                .uv(uv)
                .tint(Color32::from_gray(120))
                .corner_radius(r)
                .paint_at(ui, rect);
            ui.painter()
                .rect_filled(rect, r, Color32::from_black_alpha(70));
        }
        let button_width = (size.x - 24.0).min(134.0);
        let button_rect =
            Rect::from_min_size(rect.min + vec2(12.0, 12.0), vec2(button_width, 40.0));
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(button_rect)
                .layout(egui::Layout::top_down(Align::Min)),
        );
        if icon_button_wide(
            &mut child,
            Glyph::Wave,
            "Voice",
            Tone::Secondary,
            button_width,
            true,
        )
        .on_hover_text("Open the voice extractor")
        .clicked()
        {
            self.voice_open = true;
            self.load_clips();
            self.selected_speaker = game.players.first().map(Player::key);
        }
        let p = ui.painter();
        let cx = rect.center().x;
        let light = |a: f32| KEY.gamma_multiply(a);
        p.text(
            pos2(cx, rect.top() + size.y * 0.36),
            Align2::CENTER_CENTER,
            &game.map,
            mono(11.0),
            light(0.7),
        );
        let name = one_line(ui, map_name(&game.map), display(17.0), KEY, size.x - 18.0);
        p.galley(
            pos2(cx - name.size().x / 2.0, rect.top() + size.y * 0.36 + 12.0),
            name,
            KEY,
        );
        p.text(
            pos2(cx, rect.top() + size.y * 0.58),
            Align2::CENTER_CENTER,
            "VS",
            mono(11.0),
            light(0.6),
        );
        p.text(
            pos2(cx, rect.top() + size.y * 0.58 + 30.0),
            Align2::CENTER_CENTER,
            format!(
                "{} : {}",
                number(game.teams[0].score),
                number(game.teams[1].score)
            ),
            display(25.0),
            KEY,
        );
        p.text(
            pos2(cx, rect.bottom() - 24.0),
            Align2::CENTER_CENTER,
            duration(game.duration),
            mono(12.0),
            light(0.8),
        );
    }

    /// One team's card; returns the height it needed (frame included).
    fn team(&mut self, ui: &mut Ui, game: &MatchData, index: usize, height: f32) -> f32 {
        let color = team_color(index);
        group(ui, |ui| {
            ui.set_min_height((height - 32.0).max(0.0));
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 0.0),
                egui::Layout::right_to_left(Align::Center),
                |ui| {
                    let winning = game.teams[index].score > game.teams[1 - index].score;
                    ui.label(
                        RichText::new(number(game.teams[index].score))
                            .font(display(30.0))
                            .color(if winning { color } else { MUTED }),
                    );
                    ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.horizontal(|ui| {
                                let (dot, _) =
                                    ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                                ui.painter().circle_filled(dot.center(), 4.0, color);
                                section(ui, if index == 0 { "Team A" } else { "Team B" });
                            });
                            ui.add(
                                Label::new(
                                    RichText::new(if game.teams[index].name.is_empty() {
                                        if index == 0 { "Team A" } else { "Team B" }
                                    } else {
                                        &game.teams[index].name
                                    })
                                    .font(bold(17.0))
                                    .color(INK),
                                )
                                .truncate(),
                            );
                        });
                    });
                },
            );
            ui.add_space(space::S);
            divider(ui);
            ui.add_space(space::XS);
            let players: Vec<_> = game
                .players
                .iter()
                .filter(|p| p.team == Some(index))
                .collect();
            for player in &players {
                ui.push_id(player.key(), |ui| {
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), 46.0),
                        egui::Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.set_min_height(46.0);
                            let key = player.key();
                            let mut selected = self.selected_players.contains(&key);
                            if check(ui, &mut selected).changed() {
                                if selected {
                                    self.selected_players.insert(key);
                                } else {
                                    self.selected_players.remove(&key);
                                }
                            }
                            ui.add_space(4.0);
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                let menu = icon_key(ui, Glyph::ChevronDown, true)
                                    .on_hover_text("SteamID and profiles");
                                Popup::menu(&menu).show(|ui| self.player_menu(ui, player));
                                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                                    let response = ui
                                        .vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 1.0;
                                            player_name(ui, player, bold(14.5));
                                            let stats = format!(
                                                "{}/{}/{} · {} ADR",
                                                number(player.kills),
                                                number(player.deaths),
                                                number(player.assists),
                                                player
                                                    .adr(game.completed_rounds)
                                                    .map(|v| format!("{v:.0}"))
                                                    .unwrap_or_else(|| "—".into())
                                            );
                                            ui.add(
                                                Label::new(
                                                    RichText::new(stats)
                                                        .font(mono(11.5))
                                                        .color(MUTED),
                                                )
                                                .truncate(),
                                            );
                                        })
                                        .response
                                        .interact(Sense::click());
                                    response.context_menu(|ui| self.player_menu(ui, player));
                                });
                            });
                        },
                    );
                });
            }
            if players.is_empty() {
                ui.add_space(space::S);
                hint(ui, "No recorded players on this team.");
            }
            ui.add_space(space::XS);
            divider(ui);
            ui.add_space(space::S);
            let all = !players.is_empty()
                && players
                    .iter()
                    .all(|p| self.selected_players.contains(&p.key()));
            if !players.is_empty()
                && small_link(
                    ui,
                    if all {
                        "Clear team selection"
                    } else {
                        "Select all"
                    },
                )
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
            // The content plus the card's 16-point padding above and below.
            ui.min_rect().height() + 32.0
        })
    }

    fn player_menu(&mut self, ui: &mut Ui, player: &Player) {
        player_name(ui, player, bold(14.5));
        if player.comm_muted {
            ui.set_max_width(260.0);
            hint(ui, MUTED_HINT);
        }
        let Some(steam) = &player.steam_id else {
            hint(ui, "SteamID64 not recorded. No profile is invented.");
            return;
        };
        ui.label(RichText::new(steam).font(mono(11.5)).color(MUTED));
        ui.separator();
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
        let chosen: Vec<(String, String)> = game
            .players
            .iter()
            .filter(|p| self.selected_players.contains(&p.key()))
            .map(|p| (p.key(), p.name.clone()))
            .collect();
        group(ui, |ui| {
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 0.0),
                egui::Layout::right_to_left(Align::Center),
                |ui| {
                    if icon_button(
                        ui,
                        Glyph::Wave,
                        "Copy voice command",
                        Tone::Primary,
                        !chosen.is_empty(),
                    )
                    .on_hover_text("Copies tv_listen_voice_indices for the selected players")
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
                    if !chosen.is_empty() {
                        ui.add_space(space::S);
                        if small_link(ui, "Clear").clicked() {
                            self.selected_players.clear();
                        }
                    }
                    ui.add_space(space::M);
                    ui.with_layout(
                        egui::Layout::left_to_right(Align::Center).with_main_wrap(true),
                        |ui| {
                            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                            if chosen.is_empty() {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    section(ui, "Spectator voice");
                                    hint(
                                        ui,
                                        "Select the players you would like to hear in the demo.",
                                    );
                                });
                            } else {
                                section(ui, "Listening to");
                                ui.add_space(2.0);
                                for (key, name) in &chosen {
                                    if chip(ui, name, true, true)
                                        .on_hover_text("Remove from the command")
                                        .clicked()
                                    {
                                        self.selected_players.remove(key);
                                    }
                                }
                            }
                        },
                    );
                },
            );
        });
    }

    fn timeline(&mut self, ui: &mut Ui, game: &MatchData) {
        group(ui, |ui| {
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 0.0),
                egui::Layout::right_to_left(Align::Center),
                |ui| {
                    for index in [1usize, 0] {
                        let name = if game.teams[index].name.is_empty() {
                            if index == 0 {
                                "Team A".to_string()
                            } else {
                                "Team B".to_string()
                            }
                        } else {
                            game.teams[index].name.clone()
                        };
                        let g = one_line(ui, name, body(12.5), LABEL, 140.0);
                        let (r, _) =
                            ui.allocate_exact_size(vec2(g.size().x + 16.0, 18.0), Sense::hover());
                        ui.painter().circle_filled(
                            pos2(r.left() + 4.0, r.center().y),
                            4.0,
                            team_color(index),
                        );
                        ui.painter().galley(
                            pos2(r.left() + 14.0, r.center().y - g.size().y / 2.0),
                            g,
                            LABEL,
                        );
                    }
                    ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                        section(ui, "Round timeline");
                        ui.label(
                            RichText::new(format!("{} observed rounds", game.rounds.len()))
                                .font(mono(11.5))
                                .color(MUTED),
                        );
                    });
                },
            );
            ui.add_space(space::M);
            if game.rounds.is_empty() {
                hint(ui, "Unavailable — no completed-round transitions recorded.");
                return;
            }
            let gap = 4.0;
            let width = ((ui.available_width() - (game.rounds.len() - 1) as f32 * gap)
                / game.rounds.len() as f32)
                .max(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(gap, 8.0);
                for round in &game.rounds {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(width, 33.0), Sense::click());
                    let (hover, press) = hover_press(ui, &response, true);
                    let color = match round.winner {
                        Some(i) if i < 2 => team_color(i),
                        _ => MUTED,
                    };
                    let face =
                        Rect::from_min_size(rect.min + vec2(0.0, 2.0 * press), vec2(width, 30.0));
                    let r = CornerRadius::same(radius::TINY + 1);
                    let p = ui.painter();
                    p.rect_filled(
                        Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(width, 30.0)),
                        r,
                        mix(color, INK, 0.15),
                    );
                    p.rect(
                        face,
                        r,
                        mix(KEY, color, 0.16 + 0.14 * hover),
                        Stroke::new(1.0, mix(EDGE, color, 0.55)),
                        StrokeKind::Inside,
                    );
                    if width > 16.0 {
                        p.text(
                            face.center(),
                            Align2::CENTER_CENTER,
                            round.number,
                            mono(11.0),
                            INK,
                        );
                    }
                    let response = response.on_hover_cursor(CursorIcon::PointingHand);
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
        group(ui, |ui| {
            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                section(ui, "Tags");
                ui.add_space(6.0);
                if !demo.tags.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        for tag in &demo.tags {
                            if chip(ui, tag, false, true)
                                .on_hover_text("Remove tag")
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
                    ui.add_space(8.0);
                }
                ui.horizontal(|ui| {
                    let response = text_field(
                        ui,
                        TextEdit::singleline(&mut self.tag_input)
                            .hint_text(RichText::new("Add a tag").color(MUTED))
                            .desired_width((ui.available_width() - 90.0).clamp(80.0, 220.0)),
                    );
                    if button(ui, "Add", Tone::Secondary, true).clicked()
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
                section(ui, "Notes");
                ui.add_space(6.0);
                if let Some(d) = self
                    .store
                    .as_mut()
                    .and_then(|s| s.library.demos.get_mut(&demo.id))
                {
                    let changed = text_field(
                        ui,
                        TextEdit::multiline(&mut d.note)
                            .hint_text(
                                RichText::new("Clutches, suspicious moments, tactics…")
                                    .color(MUTED),
                            )
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
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), 0.0),
            egui::Layout::right_to_left(Align::Center),
            |ui| {
                ui.label(
                    RichText::new("WAV · 48 kHz · mono · 16-bit PCM")
                        .font(mono(12.0))
                        .color(LABEL),
                );
                ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                    if back_pill(ui, "Match").clicked() {
                        self.voice_open = false;
                        self.playback = None;
                    }
                    ui.add_space(space::M);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        section(ui, "Voice extractor");
                        ui.add(
                            Label::new(RichText::new(demo.filename()).font(bold(17.0)).color(INK))
                                .truncate(),
                        );
                    });
                });
            },
        );
        ui.add_space(space::L);
        let players = demo
            .parsed
            .as_ref()
            .map(|g| g.players.clone())
            .unwrap_or_default();
        let height = ui.available_height();
        let left_width = 250.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            ui.allocate_ui_with_layout(
                vec2(left_width, height),
                egui::Layout::top_down(Align::Min),
                |ui| self.voice_players(ui, demo, &players, height),
            );
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), height),
                egui::Layout::top_down(Align::Min),
                |ui| self.voice_playback(ui, &players, height),
            );
        });
    }

    fn voice_players(&mut self, ui: &mut Ui, demo: &Demo, players: &[Player], height: f32) {
        group(ui, |ui| {
            ui.horizontal(|ui| {
                section(ui, "Players");
                ui.label(
                    RichText::new(players.len().to_string())
                        .font(mono(11.5))
                        .color(MUTED),
                );
            });
            ui.add_space(6.0);
            ScrollArea::vertical()
                .id_salt("voice_players_scroll")
                .max_height((height - 250.0).max(120.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for player in players {
                        let key = player.key();
                        let selected = self.selected_speaker.as_ref() == Some(&key);
                        let steam = player
                            .steam_id
                            .as_deref()
                            .unwrap_or("SteamID64 not recorded");
                        let sub = if player.comm_muted {
                            format!("Muted · {steam}")
                        } else {
                            steam.to_owned()
                        };
                        let row = nav_item(
                            ui,
                            ("speaker", &key),
                            None,
                            &player.name,
                            Some(&sub),
                            None,
                            selected,
                        );
                        let row = if player.comm_muted {
                            row.on_hover_text(MUTED_HINT)
                        } else {
                            row
                        };
                        if row.clicked() {
                            self.selected_speaker = Some(key);
                            let track = player
                                .steam_id
                                .as_deref()
                                .and_then(|steam| {
                                    audio::latest_track(
                                        &self.clips,
                                        ClipKind::PlayerTrack,
                                        Some(steam),
                                    )
                                })
                                .cloned();
                            self.playback = None;
                            if let Some(track) = track {
                                self.play(&track);
                            }
                        }
                    }
                });
            ui.add_space(space::M);
            let selected = players
                .iter()
                .find(|p| Some(p.key()) == self.selected_speaker);
            let width = ui.available_width();
            if icon_button_wide(
                ui,
                Glyph::Wave,
                "Extract player",
                Tone::Primary,
                width,
                self.active.is_none() && selected.and_then(|p| p.steam_id.as_ref()).is_some(),
            )
            .clicked()
            {
                self.parse_demo(demo, Some(selected.and_then(|p| p.steam_id.clone())));
            }
            ui.add_space(2.0);
            if wide_button(
                ui,
                "Extract all players",
                Tone::Secondary,
                width,
                self.active.is_none(),
            )
            .clicked()
            {
                self.parse_demo(demo, Some(None));
            }
            ui.add_space(4.0);
            hint(
                ui,
                "Only voice actually recorded in the demo can be extracted.",
            );
        });
    }

    fn voice_playback(&mut self, ui: &mut Ui, players: &[Player], height: f32) {
        group(ui, |ui| {
            // The card never grows past the window: in a small one its content scrolls,
            // so the clip list stays reachable.
            let viewport = (height - 48.0).max(160.0);
            ScrollArea::vertical()
                .id_salt("voice_playback_scroll")
                .max_height(viewport)
                .show(ui, |ui| {
                    let top = ui.cursor().top();
                    section(ui, "Voice playback");
                    hint(
                        ui,
                        "Speech only · silent gaps skipped · overlapping voices mixed · original clips kept",
                    );
                    ui.add_space(space::M);
                    let everyone =
                        audio::latest_track(&self.clips, ClipKind::Everyone, None).cloned();
                    self.voice_track_row(ui, "everyone", "Everyone", everyone.as_ref());
                    let selected = players
                        .iter()
                        .find(|player| Some(player.key()) == self.selected_speaker);
                    let player_track = selected
                        .and_then(|player| player.steam_id.as_deref())
                        .and_then(|steam| {
                            audio::latest_track(&self.clips, ClipKind::PlayerTrack, Some(steam))
                        })
                        .cloned();
                    self.voice_track_row(
                        ui,
                        "player",
                        selected
                            .map(|player| player.name.as_str())
                            .unwrap_or("Selected player"),
                        player_track.as_ref(),
                    );
                    ui.add_space(space::XS);
                    divider(ui);
                    ui.add_space(space::S);
                    let count = self
                        .clips
                        .iter()
                        .filter(|clip| clip.kind == ClipKind::Segment)
                        .count();
                    if disclosure(ui, "clips", &format!("Individual clips ({count})")) {
                        ui.add_space(space::S);
                        // The list fills what is left of the card, but never gets too
                        // short to use: then the card scrolls instead.
                        self.voice_clips(ui, top + viewport);
                    }
                });
        });
    }

    /// The searchable list of single voice clips, reaching down to `bottom` if it can.
    fn voice_clips(&mut self, ui: &mut Ui, bottom: f32) {
        const ROW: f32 = 48.0;
        text_field(
            ui,
            TextEdit::singleline(&mut self.clip_query)
                .hint_text(RichText::new("Search clips by player…").color(MUTED))
                .desired_width(f32::INFINITY),
        );
        ui.add_space(space::S);
        let query = self.clip_query.to_lowercase();
        let clips: Vec<&Clip> = self
            .clips
            .iter()
            .filter(|clip| {
                clip.kind == ClipKind::Segment && clip.player.to_lowercase().contains(&query)
            })
            .collect();
        let playing = self.playback.as_ref().map(|playback| &playback.clip_id);
        // Clicks are collected here and acted on once the list no longer borrows the clips.
        let mut toggle = None;
        let mut reveal = None;
        let height = (bottom - ui.cursor().top() - 1.0).max(3.0 * ROW);
        ScrollArea::vertical()
            .id_salt("voice_saved_scroll")
            .max_height(height)
            .min_scrolled_height(height)
            .show_rows(ui, ROW, clips.len(), |ui, rows| {
                for clip in &clips[rows] {
                    // Every row is exactly ROW tall, as the virtual list assumes.
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
                    let mut ui = ui.new_child(
                        UiBuilder::new()
                            .id_salt((&clip.id, &clip.path))
                            .max_rect(rect)
                            .layout(egui::Layout::left_to_right(Align::Center)),
                    );
                    let is_playing = playing == Some(&clip.id);
                    let glyph = if is_playing { Glyph::Stop } else { Glyph::Play };
                    if icon_key(&mut ui, glyph, true).clicked() {
                        toggle = Some((*clip).clone());
                    }
                    ui.add_space(4.0);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if icon_key(ui, Glyph::Folder, true)
                            .on_hover_text("Reveal original WAV clip")
                            .clicked()
                        {
                            reveal = Some(clip.path.clone());
                        }
                        ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.add(
                                    Label::new(
                                        RichText::new(&clip.player).font(bold(14.0)).color(INK),
                                    )
                                    .truncate(),
                                );
                                let round = clip
                                    .round
                                    .map(|round| format!("Round {round}"))
                                    .unwrap_or_else(|| "Unnumbered".into());
                                ui.add(
                                    Label::new(
                                        RichText::new(format!(
                                            "Demo {} · {:.2} s · {round}",
                                            duration(Some(clip.demo_seconds)),
                                            clip.duration,
                                        ))
                                        .font(mono(11.0))
                                        .color(MUTED),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                    });
                }
            });
        if let Some(clip) = toggle {
            if self
                .playback
                .as_ref()
                .is_some_and(|playback| playback.clip_id == clip.id)
            {
                self.playback = None;
            } else {
                self.play(&clip);
            }
        }
        if let Some(path) = reveal
            && let Err(error) = files::reveal(&path)
        {
            self.error(error);
        }
    }

    fn voice_track_row(&mut self, ui: &mut Ui, id: &str, title: &str, clip: Option<&Clip>) {
        ui.push_id(id, |ui| {
            Frame::new()
                .fill(PAGE)
                .stroke(Stroke::new(1.0, EDGE))
                .corner_radius(CornerRadius::same(radius::BUTTON))
                .inner_margin(Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), 0.0),
                        egui::Layout::right_to_left(Align::Center),
                        |ui| {
                            if let Some(clip) = clip {
                                if icon_key(ui, Glyph::Folder, true)
                                    .on_hover_text("Reveal combined WAV track")
                                    .clicked()
                                    && let Err(error) = files::reveal(&clip.path)
                                {
                                    self.error(error);
                                }
                                let loaded = self
                                    .playback
                                    .as_ref()
                                    .is_some_and(|playback| playback.clip_id == clip.id);
                                if icon_button(ui, Glyph::Stop, "Stop", Tone::Secondary, loaded)
                                    .clicked()
                                {
                                    self.playback = None;
                                }
                                let playing = self
                                    .playback
                                    .as_ref()
                                    .filter(|playback| playback.clip_id == clip.id);
                                let (glyph, label) = match playing {
                                    Some(playback) if playback.paused() => (Glyph::Play, "Resume"),
                                    Some(_) => (Glyph::Pause, "Pause"),
                                    None => (Glyph::Play, "Play"),
                                };
                                if icon_button(ui, glyph, label, Tone::Primary, true).clicked() {
                                    if let Some(playback) = &self
                                        .playback
                                        .as_ref()
                                        .filter(|playback| playback.clip_id == clip.id)
                                    {
                                        if playback.paused() {
                                            playback.resume();
                                        } else {
                                            playback.pause();
                                        }
                                    } else {
                                        self.play(clip);
                                    }
                                }
                            }
                            ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                                ui.add(
                                    Label::new(RichText::new(title).font(bold(15.0)).color(INK))
                                        .truncate(),
                                );
                                if let Some(clip) = clip {
                                    ui.label(
                                        RichText::new(duration(Some(clip.duration)))
                                            .font(mono(12.0))
                                            .color(LABEL),
                                    );
                                }
                            });
                        },
                    );
                    let Some(clip) = clip else {
                        ui.add_space(2.0);
                        hint(
                            ui,
                            if id == "everyone" {
                                "Extract all players to create the everyone track."
                            } else {
                                "Click a player, then extract that player or everyone to create their track."
                            },
                        );
                        return;
                    };
                    ui.add_space(space::S);
                    let progress = self
                        .playback
                        .as_ref()
                        .filter(|playback| playback.clip_id == clip.id)
                        .map(|playback| (playback.position() / clip.duration).clamp(0.0, 1.0) as f32)
                        .unwrap_or(0.0);
                    let (rect, response) = ui.allocate_exact_size(
                        vec2(ui.available_width().max(80.0), 46.0),
                        Sense::click(),
                    );
                    let response = response.on_hover_cursor(CursorIcon::PointingHand);
                    let p = ui.painter();
                    p.rect_filled(rect, CornerRadius::same(radius::CONTROL), GROUND);
                    let wave = rect.shrink2(vec2(10.0, 6.0));
                    let count = clip.peaks.len().max(1);
                    let bar = (wave.width() / count as f32 * 0.45).clamp(1.0, 5.0);
                    for (index, peak) in clip.peaks.iter().enumerate() {
                        let x = wave.left() + (index as f32 + 0.5) / count as f32 * wave.width();
                        let h = (peak.clamp(0.0, 1.0) * wave.height() * 0.5).max(1.0);
                        p.line_segment(
                            [pos2(x, wave.center().y - h), pos2(x, wave.center().y + h)],
                            Stroke::new(
                                bar,
                                if index as f32 / count as f32 <= progress && progress > 0.0 {
                                    SIGNAL
                                } else {
                                    mix(EDGE, MUTED, 0.45)
                                },
                            ),
                        );
                    }
                    if progress > 0.0 {
                        let x = wave.left() + progress * wave.width();
                        p.vline(x, rect.y_range().shrink(4.0), Stroke::new(1.5, INK));
                    }
                    if response.clicked()
                        && let Some(position) = response.interact_pointer_pos()
                    {
                        let fraction = ((position.x - wave.left()) / wave.width()).clamp(0.0, 1.0);
                        if !self
                            .playback
                            .as_ref()
                            .is_some_and(|playback| playback.clip_id == clip.id)
                        {
                            self.play(clip);
                        }
                        if let Some(playback) = self
                            .playback
                            .as_ref()
                            .filter(|playback| playback.clip_id == clip.id)
                            && let Err(error) = playback.seek(fraction as f64 * clip.duration)
                        {
                            self.error(error);
                        }
                    }
                });
            ui.add_space(space::S);
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
        let mut done = false;
        let (title, width) = match &dialog {
            Dialog::Settings => ("Settings", 560.0),
            Dialog::About => ("Small Demo Manager", 480.0),
            Dialog::InstallUpdate => ("Install the update?", 480.0),
            Dialog::Rename { to_cs2: true, .. } => ("Transfer to CS2", 520.0),
            Dialog::Rename { .. } => ("Rename demo", 520.0),
            Dialog::RemoveFolder(_) => ("Stop watching folder?", 460.0),
            Dialog::Trash(_) => ("Move to Recycle Bin?", 460.0),
        };
        let max_height = (ctx.content_rect().height() - 120.0).max(260.0);
        Modal::new(Id::unique("dialog"))
            .backdrop_color(Color32::from_rgba_unmultiplied(0x1B, 0x1B, 0x1F, 70))
            .frame(
                Frame::new()
                    .fill(KEY)
                    .stroke(Stroke::new(1.0, EDGE))
                    .corner_radius(CornerRadius::same(radius::CARD + 2))
                    .inner_margin(Margin::same(24))
                    .shadow(Shadow {
                        offset: [0, 10],
                        blur: 30,
                        spread: 0,
                        color: Color32::from_black_alpha(40),
                    }),
            )
            .show(ctx, |ui| {
                ui.set_width(width);
                ScrollArea::vertical()
                    .id_salt("dialog_scroll")
                    .max_height(max_height)
                    // A modal is centred at last frame's size; without a floor a long
                    // dialog would settle at whatever height it first happened to get.
                    .min_scrolled_height(max_height.min(640.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(title).font(display(22.0)).color(INK));
                        ui.add_space(space::M);
                        done = self.dialog_body(ui, ctx, &mut dialog);
                    });
            });
        if !done && !ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.dialog = Some(dialog);
        }
    }

    /// The contents of the open dialog; returns true when it should close.
    fn dialog_body(&mut self, ui: &mut Ui, ctx: &Context, dialog: &mut Dialog) -> bool {
        let mut done = false;
        match dialog {
            Dialog::About => {
                ui.label(
                    RichText::new(format!(
                        "v{} · native Rust · personal use",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .font(mono(12.0))
                    .color(LABEL),
                );
                ui.add_space(space::M);
                ui.label(RichText::new("Originally created by Pythaeus. Continued by darkcr0w in his absence, respecting his work and keeping his legacy alive.").font(body(14.5)).color(INK));
                ui.add_space(space::S);
                hint(
                    ui,
                    "Special thanks to KEROVSKI, HaiX, LobaCS2, neokCS, Throw, and everyone testing builds in the community.",
                );
                ui.add_space(space::M);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = space::L;
                    for (label, url) in [
                        (
                            "GitHub repository",
                            "https://github.com/thedarkcr0w/Small-Demo-Manager",
                        ),
                        ("Support darkcr0w", "https://ko-fi.com/darkcr0w"),
                        ("Support Pythaeus", "https://ko-fi.com/pythaeus"),
                    ] {
                        if small_link(ui, label).clicked()
                            && let Err(e) = open::that(url)
                        {
                            self.error(e);
                        }
                    }
                });
                ui.add_space(space::M);
                hint(
                    ui,
                    "Native UI: egui/eframe · Fonts: Onest, Unbounded, JetBrains Mono (OFL) · Parser: source2-demo · Audio: libopus / rodio. See LICENSE and THIRD-PARTY-NOTICES.md.",
                );
                ui.add_space(space::L);
                if button(ui, "Close", Tone::Secondary, true).clicked() {
                    done = true;
                }
            }
            Dialog::InstallUpdate => {
                let version = self
                    .updater
                    .prepared
                    .as_ref()
                    .map(|p| p.version.as_str())
                    .unwrap_or("—");
                ui.label(
                    RichText::new(format!(
                        "Install version {version} from the verified GitHub release?"
                    ))
                    .font(bold(15.0))
                    .color(INK),
                );
                ui.add_space(space::XS);
                hint(
                    ui,
                    "The app will save your library, close, install the update, and reopen this demo. Existing app files are backed up; failed installation or startup restores the previous files. Demos, notes, and saved audio are not replaced.",
                );
                ui.add_space(space::XS);
                hint(
                    ui,
                    "The app folder must be writable. No administrator prompt or external updater script is used.",
                );
                ui.add_space(space::L);
                ui.horizontal(|ui| {
                    if big_button(
                        ui,
                        None,
                        "Restart & install",
                        Tone::Primary,
                        0.0,
                        self.active.is_none()
                            && self.updater.task.is_none()
                            && self.updater.prepared.is_some(),
                    )
                    .clicked()
                    {
                        done = self.begin_update_install(ctx);
                    }
                    if big_button(ui, None, "Not now", Tone::Secondary, 0.0, true).clicked() {
                        done = true;
                    }
                });
            }
            Dialog::Settings => {
                section(ui, "CS2 demo destination");
                ui.add_space(4.0);
                group(ui, |ui| {
                    hint(
                        ui,
                        "Choose the game's game/csgo directory (or its replays subfolder).",
                    );
                    ui.add_space(space::S);
                    ui.horizontal(|ui| {
                        text_field(
                            ui,
                            TextEdit::singleline(&mut self.cs2_input)
                                .desired_width((ui.available_width() - 104.0).max(160.0))
                                .hint_text(
                                    RichText::new("…/Counter-Strike Global Offensive/game/csgo")
                                        .color(MUTED),
                                ),
                        );
                        if button(ui, "Browse…", Tone::Secondary, true).clicked()
                            && let Some(path) = rfd::FileDialog::new().pick_folder()
                        {
                            self.cs2_input = path.to_string_lossy().into_owned();
                        }
                    });
                    ui.add_space(space::S);
                    ui.horizontal(|ui| {
                        if button(
                            ui,
                            "Auto-detect Steam install",
                            Tone::Secondary,
                            self.active.is_none(),
                        )
                        .clicked()
                        {
                            self.send(Job::Detect, "Locating CS2 in Steam's libraries…", None);
                        }
                        if !self.cs2_input.trim().is_empty() {
                            ui.add_space(space::S);
                            match steam::validate_destination(&PathBuf::from(self.cs2_input.trim()))
                            {
                                Ok(()) => {
                                    let (r, _) =
                                        ui.allocate_exact_size(vec2(16.0, 18.0), Sense::hover());
                                    paint_glyph(ui.painter(), Glyph::Check, r.center(), 12.0, OK);
                                    ui.label(
                                        RichText::new("CS2 game directory verified")
                                            .font(medium(13.5))
                                            .color(OK),
                                    );
                                }
                                Err(e) => error_text(ui, e.to_string()),
                            }
                        }
                    });
                });
                section(ui, "Transfers");
                ui.add_space(4.0);
                group_with(ui, Margin::ZERO, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    control_row(
                        ui,
                        true,
                        "Move instead of copying",
                        "Removes the original after the copy is verified with SHA-256.",
                        |ui| {
                            switch(ui, &mut self.settings_draft.move_files);
                        },
                    );
                    let moving = self.settings_draft.move_files;
                    control_row(
                        ui,
                        false,
                        "Keep a verified backup before moving",
                        "Backups: %LocalAppData%/Small-Demo-Manager/Backups.native",
                        |ui| {
                            ui.add_enabled_ui(moving, |ui| {
                                switch(ui, &mut self.settings_draft.backup_before_move);
                            });
                        },
                    );
                });
                hint(
                    ui,
                    "Demos are parsed only when clicked or dropped. Adding or rescanning a folder never starts bulk parsing. Player search uses demos you have already opened.",
                );
                ui.add_space(space::L);
                self.update_settings(ui);
                ui.add_space(space::L);
                ui.horizontal(|ui| {
                    if big_button(ui, None, "Save settings", Tone::Primary, 0.0, true).clicked() {
                        let path = PathBuf::from(self.cs2_input.trim());
                        if !path.as_os_str().is_empty()
                            && let Err(e) = steam::validate_destination(&path)
                        {
                            self.error(e);
                            return;
                        }
                        self.settings_draft.cs2_path = path;
                        self.settings_draft.background_parse = false;
                        if let Some(store) = &mut self.store {
                            store.library.settings = self.settings_draft.clone();
                        }
                        self.persist();
                        done = true;
                    }
                    if big_button(ui, None, "Cancel", Tone::Secondary, 0.0, true).clicked() {
                        done = true;
                    }
                });
            }
            Dialog::Rename { id, name, to_cs2 } => {
                section(ui, "Filename (without .dem)");
                ui.add_space(4.0);
                text_field(ui, TextEdit::singleline(name).desired_width(f32::INFINITY));
                if let Err(e) = files::demo_filename(name) {
                    ui.add_space(4.0);
                    error_text(ui, e.to_string());
                }
                if *to_cs2 {
                    let settings = self
                        .store
                        .as_ref()
                        .map(|s| s.library.settings.clone())
                        .unwrap_or_default();
                    ui.add_space(space::M);
                    section(ui, "Destination");
                    ui.label(
                        RichText::new(settings.cs2_path.display().to_string())
                            .font(mono(12.0))
                            .color(INK),
                    );
                    ui.add_space(4.0);
                    hint(
                        ui,
                        if settings.move_files {
                            if settings.backup_before_move {
                                "Move · verified backup enabled · SHA-256 checked before original is removed"
                            } else {
                                "Move · backup disabled · SHA-256 checked before original is removed"
                            }
                        } else {
                            "Copy · original stays where it is · SHA-256 checked"
                        },
                    );
                }
                ui.add_space(space::L);
                ui.horizontal(|ui| {
                    if big_button(
                        ui,
                        None,
                        "Confirm",
                        Tone::Primary,
                        120.0,
                        self.active.is_none() && files::demo_filename(name).is_ok(),
                    )
                    .clicked()
                    {
                        let Some(store) = &self.store else {
                            return;
                        };
                        let Some(demo) = store.library.demos.get(id).cloned() else {
                            return;
                        };
                        if *to_cs2 {
                            let settings = store.library.settings.clone();
                            if let Err(e) = steam::validate_destination(&settings.cs2_path) {
                                self.error(format!("{e}. Configure this in Settings first"));
                                return;
                            }
                            let filename = match files::demo_filename(name) {
                                Ok(name) => name,
                                Err(e) => {
                                    self.error(e);
                                    return;
                                }
                            };
                            let backup = (settings.move_files && settings.backup_before_move)
                                .then(|| store.root.join("Backups.native"));
                            self.send(
                                Job::Transfer {
                                    id: id.clone(),
                                    source: demo.path,
                                    destination: settings.cs2_path.join(filename),
                                    moving: settings.move_files,
                                    backup,
                                },
                                "Transferring and verifying SHA-256…",
                                None,
                            );
                        } else {
                            self.send(
                                Job::Rename {
                                    id: id.clone(),
                                    source: demo.path,
                                    name: name.clone(),
                                },
                                "Renaming and verifying demo…",
                                None,
                            );
                        }
                        done = true;
                    }
                    if big_button(ui, None, "Cancel", Tone::Secondary, 120.0, true).clicked() {
                        done = true;
                    }
                });
            }
            Dialog::RemoveFolder(id) => {
                ui.label(RichText::new("Demo files, saved audio, notes, and tags stay on disk. You can add this folder again later.").font(body(14.5)).color(INK));
                ui.add_space(space::L);
                ui.horizontal(|ui| {
                    if big_button(
                        ui,
                        None,
                        "Stop watching",
                        Tone::Primary,
                        0.0,
                        self.active.is_none(),
                    )
                    .clicked()
                    {
                        if let Some(store) = &mut self.store {
                            store.library.folders.retain(|f| &f.id != id);
                            for demo in store
                                .library
                                .demos
                                .values_mut()
                                .filter(|d| &d.folder_id == id)
                            {
                                demo.available = false;
                            }
                        }
                        self.view = View::All;
                        self.refresh_watcher();
                        self.persist();
                        self.rescan_pending = true;
                        done = true;
                    }
                    if big_button(ui, None, "Cancel", Tone::Secondary, 0.0, true).clicked() {
                        done = true;
                    }
                });
            }
            Dialog::Trash(id) => {
                ui.label(RichText::new("The demo will be moved to the Windows Recycle Bin, not permanently deleted. Saved voice clips and annotations stay intact.").font(body(14.5)).color(INK));
                ui.add_space(space::L);
                ui.horizontal(|ui| {
                    if big_button(
                        ui,
                        None,
                        "Move to Recycle Bin",
                        Tone::Primary,
                        0.0,
                        self.active.is_none(),
                    )
                    .clicked()
                    {
                        if let Some(demo) = self
                            .store
                            .as_ref()
                            .and_then(|s| s.library.demos.get(id))
                            .cloned()
                        {
                            self.send(
                                Job::Trash {
                                    id: id.clone(),
                                    path: demo.path,
                                },
                                "Moving demo to the Recycle Bin…",
                                None,
                            );
                        }
                        done = true;
                    }
                    if big_button(ui, None, "Cancel", Tone::Secondary, 0.0, true).clicked() {
                        done = true;
                    }
                });
            }
        }
        done
    }
}
