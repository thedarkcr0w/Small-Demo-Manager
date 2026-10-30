use small_demo_manager::{files, parser};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

/// Download the pinned, public demofile-net fixtures (see workflow), not private
/// user matches. This is intentionally ignored without the fixture directory.
#[test]
#[ignore = "Set SDM_DEMO_FIXTURES to the pinned public demo fixtures directory"]
fn public_protocol_versions_parse_without_modifying_the_files() {
    let root = PathBuf::from(
        std::env::var_os("SDM_DEMO_FIXTURES").expect("SDM_DEMO_FIXTURES is required"),
    );
    for name in ["14011.dem", "14150_ag2.dem", "14160.dem"] {
        let path = root.join(name);
        let before = files::hash_file(&path).expect("fixture hash");
        let parsed = parser::parse(
            &path,
            Arc::new(AtomicBool::new(false)),
            Arc::new(|_| {}),
            None,
        )
        .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        assert!(!parsed.game.map.is_empty(), "{name}: no recorded map");
        assert!(parsed.game.ticks > 0, "{name}: no demo ticks");
        assert_eq!(
            files::hash_file(&path).expect("fixture hash"),
            before,
            "{name}: source was modified"
        );
    }
}
