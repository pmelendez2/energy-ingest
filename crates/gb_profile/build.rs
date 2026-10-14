// SPDX-License-Identifier: MIT
// AI-assisted: ChatGPT / GPT-6; 2026-10-08.

use std::env;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_owned())
}

fn main() {
    println!("cargo:rerun-if-env-changed=GB_PROFILE_OFFICIAL_RELEASE");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../.git/HEAD");

    let version = env::var("CARGO_PKG_VERSION").expect("Cargo version");
    let revision =
        git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    let short = revision.chars().take(12).collect::<String>();
    let tag = format!("gb_profile-v{version}");
    let head_tag_matches = git(&["rev-list", "-n", "1", &tag])
        .is_some_and(|tag_rev| tag_rev == revision);
    let is_clean = git(&["status", "--porcelain", "--untracked-files=no"])
        .is_some_and(|status| status.is_empty());
    let requested_release =
        env::var("GB_PROFILE_OFFICIAL_RELEASE").is_ok_and(|value| value == "1");

    if requested_release
        && (version.contains('-') || !head_tag_matches || !is_clean)
    {
        panic!(
            "official release requires stable SemVer, a clean checkout, \
             and matching annotated/release tag {tag} at HEAD"
        );
    }

    let channel = if requested_release {
        "stable"
    } else if version.contains("-rc.") {
        "release-candidate"
    } else if version.contains("-beta.") {
        "beta"
    } else if version.contains("-alpha.") {
        "alpha"
    } else {
        "development"
    };
    let build_profile =
        env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned());
    let commit_state = if is_clean { "clean" } else { "modified" };

    println!("cargo:rustc-env=GB_PROFILE_GIT_REVISION={short}");
    println!("cargo:rustc-env=GB_PROFILE_BUILD_CHANNEL={channel}");
    println!("cargo:rustc-env=GB_PROFILE_BUILD_PROFILE={build_profile}");
    println!("cargo:rustc-env=GB_PROFILE_SOURCE_STATE={commit_state}");
}
