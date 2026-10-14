// SPDX-License-Identifier: MIT

use gb::profile;
use std::fs;
use std::path::Path;

#[test]
fn discover_and_validate_all_registered_profiles() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let profiles = profile::discover(&root).unwrap();
    assert!(!profiles.is_empty());
    for p in profiles {
        assert!(profile::validate(&p).unwrap() > 0);
    }
}

#[test]
fn profile_ids_reject_path_traversal() {
    assert!(!profile::valid_id("../private"));
    assert!(!profile::valid_id(""));
    assert!(profile::valid_id("sdge__cca_a"));
}

#[test]
fn duplicate_ids_and_invalid_paths_fail() {
    let temp = std::env::temp_dir().join(format!(
        "greenbutton-profile-test-{}-{:?}",
        std::process::id(),
        std::thread::current().id(),
    ));
    let dir = temp.join("example");
    fs::create_dir_all(dir.join("fixtures")).unwrap();
    fs::write(
        dir.join("profile.toml"),
        r#"
schema_version = 1
id = "example"
name = "Example"
[provider]
edc = "example"
[[fixture]]
file = "../../private.xml"
commodity = "electricity"
flow_direction = 1
unit_of_measure = 72
interval_seconds = 3600
reading_count = 1
start_date = 2026-01-01
end_date = 2026-01-01
"#,
    )
    .unwrap();
    assert!(profile::load(&dir).is_err());
    fs::remove_dir_all(&temp).unwrap();
}
