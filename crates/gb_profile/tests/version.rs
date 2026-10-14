// SPDX-License-Identifier: MIT
// AI-assisted: ChatGPT / GPT-6; 2026-10-08.

use std::process::Command;

#[test]
fn version_reports_cargo_semver() {
    let executable = env!("CARGO_BIN_EXE_gb_profile");
    let output = Command::new(executable)
        .arg("--version")
        .output()
        .expect("run gb_profile");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.starts_with(&format!(
            "gb_profile {} (",
            env!("CARGO_PKG_VERSION")
        ))
    );
}

#[test]
fn verbose_version_reports_provenance() {
    let output = Command::new(env!("CARGO_BIN_EXE_gb_profile"))
        .args(["version", "--verbose"])
        .output()
        .expect("run gb_profile");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Git revision:",
        "Source state:",
        "Build profile:",
        "Package format: 1",
        "Profile schema: 1",
    ] {
        assert!(stdout.contains(expected), "missing {expected}");
    }
}
