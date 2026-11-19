use super::*;

struct Harness {
    app: App,
    jobs: mpsc::Receiver<Job>,
    events: mpsc::Sender<Event>,
    _directory: tempfile::TempDir,
}

fn harness() -> Harness {
    let directory = tempfile::tempdir().expect("isolated UI-controller fixture");
    let store = Store::open(directory.path().join("data")).expect("store");
    let settings = store.library.settings.clone();
    let (sender, jobs) = mpsc::channel();
    let (events, receiver) = mpsc::channel();
    let (watch_sender, watch_events) = mpsc::channel();
    Harness {
        app: App {
            store: Some(store),
            worker: Worker {
                sender,
                events: receiver,
            },
            active: None,
            rescan_pending: false,
            query: String::new(),
            view: View::All,
            sort: Sort::Newest,
            map_filter: String::new(),
            tag_filter: BTreeSet::new(),
            selected: None,
            startup: None,
            parse_requested: false,
            pending_imports: vec![],
            selected_players: BTreeSet::new(),
            voice_open: false,
            selected_speaker: None,
            clip_query: String::new(),
            clips: vec![],
            playback: None,
            dialog: None,
            settings_draft: settings,
            cs2_input: String::new(),
            tag_input: String::new(),
            dirty: None,
            notice: None,
            updater: updater::State::default(),
            textures: HashMap::new(),
            watcher: None,
            watch_events,
            watch_sender,
            watch_changed: None,
            list_collapsed: false,
        },
        jobs,
        events,
        _directory: directory,
    }
}

fn add_demo(app: &mut App, id: &str) {
    let store = app.store.as_mut().expect("store");
    store.library.demos.insert(
        id.into(),
        Demo {
            id: id.into(),
            path: store.root.join(format!("{id}.dem")),
            available: true,
            ..Default::default()
        },
    );
}

#[test]
fn scanning_never_selects_or_parses_the_first_demo() {
    let mut h = harness();
    let path = h
        .app
        .store
        .as_ref()
        .expect("store")
        .root
        .join("fixture.dem");
    h.events
        .send(Event::Scanned(store::Scan {
            entries: vec![store::ScannedDemo {
                path,
                folder_id: "native".into(),
                stamp: FileStamp::default(),
                fingerprint: "fixture".into(),
            }],
            ..Default::default()
        }))
        .expect("scan event");
    h.app.consume_events();
    h.app
        .store
        .as_mut()
        .expect("store")
        .library
        .settings
        .background_parse = true;
    h.app.schedule();
    assert!(h.app.selected.is_none());
    assert!(h.app.active.is_none());
    assert!(h.jobs.try_recv().is_err());
}

#[test]
fn clicking_a_demo_requests_only_that_demo() {
    let mut h = harness();
    add_demo(&mut h.app, "first");
    add_demo(&mut h.app, "second");
    h.app.select("second".into());
    h.app.schedule();
    assert!(
        matches!(h.jobs.try_recv().expect("parse"), Job::Parse { id, output: None, .. } if id == "second")
    );
    assert!(h.jobs.try_recv().is_err());
}

#[test]
fn double_clicking_the_current_demo_does_not_cancel_its_parse() {
    let mut h = harness();
    add_demo(&mut h.app, "selected");
    h.app.select("selected".into());
    h.app.schedule();
    let cancel = h
        .app
        .active
        .as_ref()
        .expect("active")
        .cancel
        .as_ref()
        .expect("cancel")
        .clone();
    h.app.select("selected".into());
    assert!(!cancel.load(Ordering::Relaxed));
}

#[test]
fn imported_demo_is_the_only_file_requested_for_parsing_after_the_scan() {
    let mut h = harness();
    add_demo(&mut h.app, "previous");
    h.app.selected = Some("previous".into());
    h.app.voice_open = true;
    h.app.selected_speaker = Some("previous speaker".into());
    h.app.selected_players.insert("previous player".into());
    h.app.clips.push(Clip::default());
    let root = h.app.store.as_ref().expect("store").root.clone();
    let dropped = root.join("dropped.dem");
    h.events
        .send(Event::Imported(Ok(vec![dropped.clone()])))
        .expect("import result");
    h.app.consume_events();
    h.app.schedule();
    assert!(matches!(h.jobs.try_recv().expect("rescan"), Job::Scan(_)));
    h.events
        .send(Event::Scanned(store::Scan {
            entries: vec![
                store::ScannedDemo {
                    path: dropped.clone(),
                    folder_id: "native".into(),
                    stamp: FileStamp::default(),
                    fingerprint: "drop".into(),
                },
                store::ScannedDemo {
                    path: root.join("not-clicked.dem"),
                    folder_id: "native".into(),
                    stamp: FileStamp::default(),
                    fingerprint: "other".into(),
                },
            ],
            ..Default::default()
        }))
        .expect("scan result");
    h.app.consume_events();
    assert!(!h.app.voice_open, "a new drop opens its own match view");
    assert!(h.app.selected_speaker.is_none());
    assert!(h.app.selected_players.is_empty());
    assert!(h.app.clips.is_empty(), "old demo audio must not carry over");
    h.app.schedule();
    assert!(
        matches!(h.jobs.try_recv().expect("only dropped parse"), Job::Parse { path, .. } if path == dropped)
    );
    assert!(h.jobs.try_recv().is_err());
}

#[test]
fn an_invalidated_selected_cache_is_not_reparsed_without_a_new_request() {
    let mut h = harness();
    add_demo(&mut h.app, "selected");
    h.app.selected = Some("selected".into());
    h.app.schedule();
    assert!(h.jobs.try_recv().is_err());
}

#[test]
fn changing_selection_without_a_click_does_not_request_parsing() {
    let mut h = harness();
    add_demo(&mut h.app, "relocated");
    h.app.change_selection("relocated".into());
    h.app.schedule();
    assert!(h.jobs.try_recv().is_err());
}

#[test]
fn first_drop_automatically_gets_a_safe_import_destination() {
    let mut h = harness();
    h.app.import(vec![PathBuf::from("dropped.dem")]);
    let root = &h.app.store.as_ref().expect("store").root;
    assert!(
        matches!(h.jobs.try_recv().expect("import"), Job::Import { destination, paths } if destination == root.join("Imports.native") && paths == [PathBuf::from("dropped.dem")])
    );
    assert_eq!(
        h.app.store.as_ref().expect("store").library.folders.len(),
        1
    );
}

#[test]
fn dropping_during_parsing_cancels_then_imports_instead_of_discarding_the_drop() {
    let mut h = harness();
    let cancel = Arc::new(AtomicBool::new(false));
    h.app.active = Some(Active {
        label: "Parsing fixture".into(),
        progress: None,
        cancel: Some(cancel.clone()),
        parsing: true,
        demo_id: Some("old".into()),
    });
    h.app.import(vec![PathBuf::from("new.dem")]);
    assert!(cancel.load(Ordering::Relaxed));
    assert_eq!(h.app.pending_imports, [PathBuf::from("new.dem")]);
    assert!(h.jobs.try_recv().is_err());
    h.app.active = None;
    h.app.schedule();
    assert!(matches!(
        h.jobs.try_recv().expect("queued import"),
        Job::Import { .. }
    ));
    assert!(h.app.pending_imports.is_empty());
}

#[test]
fn dropping_during_extraction_waits_without_interrupting_saved_audio_work() {
    let mut h = harness();
    let cancel = Arc::new(AtomicBool::new(false));
    h.app.active = Some(Active {
        label: "Extracting fixture".into(),
        progress: None,
        cancel: Some(cancel.clone()),
        parsing: false,
        demo_id: Some("old".into()),
    });
    h.app.import(vec![PathBuf::from("new.dem")]);
    assert!(!cancel.load(Ordering::Relaxed));
    h.app.schedule();
    assert!(h.jobs.try_recv().is_err());
    assert_eq!(h.app.pending_imports.len(), 1);
}

/// One headless frame of a 1000×680 window at `clock` (which then moves on 10 ms).
fn frame(
    ctx: &Context,
    clock: &mut f64,
    events: Vec<egui::Event>,
    add: &mut dyn FnMut(&mut Ui),
) -> egui::FullOutput {
    let input = egui::RawInput {
        time: Some(*clock),
        events,
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 680.0))),
        ..Default::default()
    };
    *clock += 0.01;
    let mut output = ctx.run_ui(input, |ui| add(ui));
    // Headless: no GPU takes the font atlas, so drop its upload.
    output.textures_delta.clear();
    output
}

/// Move to `pos`, press and release there; returns the frame of the release.
fn click(
    ctx: &Context,
    clock: &mut f64,
    pos: Pos2,
    add: &mut dyn FnMut(&mut Ui),
) -> egui::FullOutput {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    frame(ctx, clock, vec![egui::Event::PointerMoved(pos)], add);
    frame(ctx, clock, vec![button(true)], add);
    frame(ctx, clock, vec![button(false)], add)
}

#[test]
fn the_window_can_still_be_closed_while_a_dialog_is_open() {
    let mut h = harness();
    h.app.dialog = Some(Dialog::About);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut clock = 0.0;
    let mut draw = |ui: &mut Ui| {
        h.app.chrome(ui);
        h.app.dialogs(&ui.ctx().clone());
    };
    // Let the modal layer settle before the pointer arrives.
    frame(&ctx, &mut clock, vec![], &mut draw);
    frame(&ctx, &mut clock, vec![], &mut draw);
    let close_button = pos2(1000.0 - 8.0 - 21.0, 27.0);
    let output = click(&ctx, &mut clock, close_button, &mut draw);
    let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, ViewportCommand::Close)),
        "the title bar's close button must work above a modal dialog"
    );
    assert!(h.app.dialog.is_some(), "the dialog itself was not touched");
}

#[test]
fn a_row_click_opens_the_demo_and_its_star_only_toggles_the_favorite() {
    let mut h = harness();
    add_demo(&mut h.app, "first");
    add_demo(&mut h.app, "second");
    let ctx = Context::default();
    theme::install(&ctx);
    let mut clock = 0.0;
    frame(&ctx, &mut clock, vec![], &mut |ui| h.app.library(ui, false));
    let row = ctx
        .read_response(Id::unique(("demo-row", "second")))
        .expect("the row was drawn")
        .rect;
    let star = row.left_top() + vec2(22.0, 21.0);
    click(&ctx, &mut clock, star, &mut |ui| h.app.library(ui, false));
    let demos = &h.app.store.as_ref().expect("store").library.demos;
    assert!(demos["second"].favorite);
    assert!(!demos["first"].favorite);
    assert!(h.app.selected.is_none(), "the star does not open the demo");
    assert!(!h.app.parse_requested);
    assert!(h.app.dirty.is_some(), "the favorite is saved");

    click(&ctx, &mut clock, row.center(), &mut |ui| {
        h.app.library(ui, false);
    });
    assert_eq!(h.app.selected.as_deref(), Some("second"));
    assert!(h.app.parse_requested, "a click asks for this demo's data");
    let demos = &h.app.store.as_ref().expect("store").library.demos;
    assert!(
        demos["second"].favorite,
        "opening leaves the favorite alone"
    );
}

#[test]
fn a_card_click_opens_the_demo() {
    let mut h = harness();
    add_demo(&mut h.app, "first");
    let ctx = Context::default();
    theme::install(&ctx);
    let mut clock = 0.0;
    frame(&ctx, &mut clock, vec![], &mut |ui| h.app.library(ui, true));
    let card = ctx
        .read_response(Id::unique(("demo-card", "first")))
        .expect("the card was drawn")
        .rect;
    click(&ctx, &mut clock, card.center(), &mut |ui| {
        h.app.library(ui, true);
    });
    assert_eq!(h.app.selected.as_deref(), Some("first"));
    assert!(h.app.parse_requested);
}

#[test]
fn the_match_view_stacks_instead_of_overflowing_a_narrow_panel() {
    let mut h = harness();
    let players = (0..10)
        .map(|i| Player {
            name: format!("Player {i}"),
            steam_id: Some(format!("7656119800000000{i}")),
            team: Some(i % 2),
            ..Default::default()
        })
        .collect();
    let demo = Demo {
        id: "fixture".into(),
        path: PathBuf::from("fixture.dem"),
        available: true,
        parsed: Some(MatchData {
            players,
            ..Default::default()
        }),
        ..Default::default()
    };
    h.app.selected = Some(demo.id.clone());
    h.app
        .store
        .as_mut()
        .expect("store")
        .library
        .demos
        .insert(demo.id.clone(), demo);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut clock = 0.0;
    // The details pane of the split view in a minimum-size window is about this wide.
    let pane = Rect::from_min_size(pos2(0.0, 0.0), vec2(386.0, 680.0));
    let mut output = None;
    for _ in 0..3 {
        output = Some(frame(&ctx, &mut clock, vec![], &mut |ui| {
            let mut ui = ui.new_child(UiBuilder::new().max_rect(pane));
            h.app.details(&mut ui);
        }));
    }
    let shapes = output.expect("frames ran").shapes;
    // Neither squeezed: every player name is shown in full …
    let mut names = 0;
    for clipped in &shapes {
        if let egui::epaint::Shape::Text(text) = &clipped.shape
            && text.galley.job.text.starts_with("Player ")
        {
            names += 1;
            assert!(!text.galley.elided, "{} is cut off", text.galley.job.text);
        }
    }
    assert!(names >= 5, "the first team's players are on screen");
    // … nor overflowing: whatever is painted (even where the scroll area would clip it)
    // stays inside the pane.
    let right = shapes
        .iter()
        .map(|clipped| clipped.shape.visual_bounding_rect())
        .filter(|rect| rect.is_positive())
        .map(|rect| rect.right())
        .fold(0.0, f32::max);
    assert!(
        right <= pane.right() + 2.0,
        "painted up to x = {right}, past the {} wide pane",
        pane.width()
    );
}

#[test]
fn voice_view_renders_everyone_and_selected_player_as_separate_tracks() {
    let mut h = harness();
    let player = Player {
        name: "Fixture player".into(),
        steam_id: Some("76561198000000001".into()),
        ..Default::default()
    };
    h.app.selected_speaker = Some(player.key());
    let demo = Demo {
        id: "fixture".into(),
        path: PathBuf::from("fixture.dem"),
        available: true,
        parsed: Some(MatchData {
            players: vec![player],
            ..Default::default()
        }),
        ..Default::default()
    };
    h.app.selected = Some(demo.id.clone());
    h.app.voice_open = true;
    h.app
        .store
        .as_mut()
        .expect("store")
        .library
        .demos
        .insert(demo.id.clone(), demo);
    h.app.clips = vec![
        Clip {
            id: "everyone".into(),
            player: "Everyone".into(),
            path: PathBuf::from("fixture/tracks/everyone.wav"),
            duration: 1.0,
            peaks: audio::waveform(&[1_000, -2_000, 3_000, -4_000], 64),
            kind: ClipKind::Everyone,
            ..Default::default()
        },
        Clip {
            id: "player".into(),
            player: "Fixture player".into(),
            steam_id: Some("76561198000000001".into()),
            path: PathBuf::from("fixture/tracks/player.wav"),
            duration: 1.0,
            peaks: audio::waveform(&[800, -1_600], 64),
            kind: ClipKind::PlayerTrack,
            ..Default::default()
        },
    ];
    let context = Context::default();
    theme::install(&context);
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1380.0, 860.0))),
            ..Default::default()
        },
        |ui| h.app.details(ui),
    );
    // This headless renderer inspects shapes without uploading a GPU texture.
    output.textures_delta.clear();
    fn text(shape: &egui::epaint::Shape, output: &mut Vec<String>) {
        match shape {
            egui::epaint::Shape::Text(value) => output.push(value.galley.job.text.clone()),
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    text(shape, output);
                }
            }
            _ => {}
        }
    }
    let mut labels = Vec::new();
    for shape in output.shapes {
        text(&shape.shape, &mut labels);
    }
    assert!(labels.iter().any(|text| text == "Everyone"));
    assert_eq!(labels.iter().filter(|text| *text == "Play").count(), 2);
    assert_eq!(labels.iter().filter(|text| *text == "Stop").count(), 2);
    assert!(
        labels
            .iter()
            .filter(|text| *text == "Fixture player")
            .count()
            >= 2
    );
    assert!(
        h.jobs.try_recv().is_err(),
        "rendering voice controls must not start a parse"
    );
}

#[test]
fn a_recorded_communication_mute_is_badged_in_the_team_card() {
    let mut h = harness();
    let player = |name: &str, steam: &str, muted: bool| Player {
        name: name.into(),
        steam_id: Some(steam.into()),
        team: Some(0),
        comm_muted: muted,
        ..Default::default()
    };
    let demo = Demo {
        id: "fixture".into(),
        path: PathBuf::from("fixture.dem"),
        available: true,
        parsed: Some(MatchData {
            players: vec![
                player("Loud player", "76561198000000001", false),
                player("Muted player", "76561198000000002", true),
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    h.app.selected = Some(demo.id.clone());
    h.app
        .store
        .as_mut()
        .expect("store")
        .library
        .demos
        .insert(demo.id.clone(), demo);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut clock = 0.0;
    let mut output = None;
    for _ in 0..2 {
        output = Some(frame(&ctx, &mut clock, vec![], &mut |ui| h.app.details(ui)));
    }
    let texts: Vec<String> = output
        .expect("frames ran")
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t == "Muted player"));
    assert!(texts.iter().any(|t| t == "Loud player"));
    assert_eq!(
        texts.iter().filter(|t| *t == "MUTED").count(),
        1,
        "only the muted player carries the badge: {texts:?}"
    );
}
