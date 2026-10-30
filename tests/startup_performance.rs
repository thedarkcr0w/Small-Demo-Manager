use small_demo_manager::{files, model::*, store};
use std::{fs, time::Instant};

/// A reproducible filesystem benchmark, not a wall-clock assertion for CI.
/// All files and annotations below are synthetic, isolated test fixtures.
#[test]
#[ignore = "Run manually to measure library loading and scan reconciliation"]
fn large_library_startup_timings() {
    let root = tempfile::tempdir().expect("benchmark directory");
    let demos = root.path().join("benchmark-only-demos");
    let data = root.path().join("benchmark-only-data");
    fs::create_dir(&demos).expect("demo directory");
    fs::create_dir(&data).expect("data directory");
    fs::write(
        root.path().join("STARTUP-BENCHMARK.txt"),
        "Small Demo Manager synthetic startup fixture\n",
    )
    .expect("fixture marker");
    let mut library = Library::default();
    library.settings.background_parse = false;
    library.settings.check_for_updates = false;
    library.folders.push(Folder {
        id: "benchmark-folder".into(),
        label: "Synthetic startup benchmark".into(),
        path: demos.clone(),
    });
    for index in 0..1_000 {
        let path = demos.join(format!("benchmark-only-{index:04}.dem"));
        fs::write(&path, []).expect("empty benchmark file");
        let id = format!("benchmark-{index:04}");
        library.demos.insert(
            id.clone(),
            Demo {
                id,
                path: path.clone(),
                folder_id: "benchmark-folder".into(),
                stamp: files::stamp(&path).expect("stamp"),
                fingerprint: files::fingerprint(&path).expect("fingerprint"),
                note: "Synthetic benchmark annotation. ".repeat(128),
                tags: vec!["benchmark-only".into()],
                parsed: Some(MatchData {
                    parser_version: PARSER_VERSION.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
    }
    let bytes = serde_json::to_vec_pretty(&library).expect("fixture JSON");
    fs::write(data.join("Library.native.json"), &bytes).expect("write fixture");
    println!(
        "fixture: {} demos, {} JSON bytes",
        library.demos.len(),
        bytes.len()
    );

    let started = Instant::now();
    let mut opened = store::Store::open(data).expect("open fixture library");
    println!("Store::open: {:?}", started.elapsed());
    let started = Instant::now();
    let scan = store::scan(&opened.library.folders);
    println!("background filesystem scan: {:?}", started.elapsed());
    let started = Instant::now();
    opened.apply_scan(scan);
    println!("UI-thread scan reconciliation: {:?}", started.elapsed());
    let started = Instant::now();
    opened.save().expect("save fixture library");
    println!("Store::save: {:?}", started.elapsed());

    assert_eq!(opened.library.demos.len(), 1_000);
    assert!(opened.library.demos.values().all(|demo| demo.available));
    assert!(
        opened
            .library
            .demos
            .values()
            .all(|demo| demo.tags == ["benchmark-only"])
    );
    assert!(opened.library.demos.values().all(|demo| demo.cache_valid()));
    assert!(
        opened
            .library
            .demos
            .values()
            .all(|demo| demo.note == "Synthetic benchmark annotation. ".repeat(128))
    );
    if std::env::var_os("SDM_KEEP_STARTUP_FIXTURE").is_some() {
        drop(opened);
        println!("retained synthetic fixture: {}", root.keep().display());
    }
}
