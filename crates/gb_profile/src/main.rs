// SPDX-License-Identifier: MIT
// AI-assisted: ChatGPT / GPT-6; 2026-10-08.

//! Standalone local profile management.

use gb::profile;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const CHANNEL: &str = env!("GB_PROFILE_BUILD_CHANNEL");
const REVISION: &str = env!("GB_PROFILE_GIT_REVISION");
const BUILD_PROFILE: &str = env!("GB_PROFILE_BUILD_PROFILE");
const SOURCE_STATE: &str = env!("GB_PROFILE_SOURCE_STATE");

fn print_version(verbose: bool) {
    println!("gb_profile {VERSION} ({CHANNEL})");
    if verbose {
        println!("Git revision: {REVISION}");
        println!("Source state: {SOURCE_STATE}");
        println!("Build profile: {BUILD_PROFILE}");
        println!("Package format: 1");
        println!("Profile schema: 1");
    }
}

fn usage() -> &'static str {
    "Usage: gb_profile [--workspace DIR] <command> [arguments]\n\
     Commands: list | init ID | add ID EXPECTATION --reviewed | \
     validate ID | validate-all | package ID OUTPUT.zip | --version | -V | version --verbose"
}

fn workspace(args: &[String]) -> Result<(PathBuf, Vec<String>)> {
    let mut path = env::current_dir()?;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--workspace" {
            i += 1;
            let value = args.get(i).ok_or("--workspace requires a path")?;
            path = PathBuf::from(value);
        } else {
            rest.push(args[i].clone());
        }
        i += 1;
    }
    Ok((path, rest))
}

fn add_to_zip(
    zip: &mut zip::ZipWriter<fs::File>,
    src: &Path,
    name: &str,
) -> Result<()> {
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name, options)?;
    let mut file = fs::File::open(src)?;
    std::io::copy(&mut file, zip)?;
    Ok(())
}

fn package(profile_dir: &Path, output: &Path) -> Result<()> {
    let loaded = profile::load(profile_dir)?;
    profile::validate(&loaded)?;
    // ADR-0028: validation checks data shape, not privacy clearance.
    // Reject publication of any fixture until the privacy-review
    // authorization protocol is completed and integrated here.
    if !loaded.metadata.fixtures.is_empty() {
        return Err(concat!(
            "fixture packaging is blocked pending ADR-0028 ",
            "privacy authorization integration"
        )
        .into());
    }
    let output = if output.is_absolute() {
        output.to_path_buf()
    } else {
        env::current_dir()?.join(output)
    };
    if output.exists() {
        return Err("output already exists (refusing overwrite)".into());
    }
    if output.extension().and_then(|e| e.to_str()) != Some("zip") {
        return Err("package output must end in .zip".into());
    }
    let base = format!("profiles/{}/", loaded.metadata.id);
    let source = fs::File::create(&output)?;
    let mut writer = zip::ZipWriter::new(source);
    let result = (|| -> Result<()> {
        let manifest = format!(
            concat!(
                "package_format_version = 1\n\n",
                "[generator]\n",
                "name = \"gb_profile\"\n",
                "version = \"{}\"\n",
                "channel = \"{}\"\n",
                "git_revision = \"{}\"\n",
                "build_profile = \"{}\"\n",
                "source_state = \"{}\"\n\n",
                "[profile]\n",
                "id = \"{}\"\n",
                "schema_version = {}\n",
            ),
            VERSION,
            CHANNEL,
            REVISION,
            BUILD_PROFILE,
            SOURCE_STATE,
            loaded.metadata.id,
            loaded.metadata.schema_version,
        );
        writer.start_file(
            "submission.toml",
            FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )?;
        writer.write_all(manifest.as_bytes())?;
        add_to_zip(
            &mut writer,
            &profile_dir.join("profile.toml"),
            &format!("{base}profile.toml"),
        )?;
        let readme = profile_dir.join("README.md");
        if readme.is_file() {
            add_to_zip(&mut writer, &readme, &format!("{base}README.md"))?;
        }
        for fixture in &loaded.metadata.fixtures {
            let rel = Path::new(&fixture.file);
            let name = format!("{base}fixtures/{}", fixture.file);
            add_to_zip(
                &mut writer,
                &profile_dir.join("fixtures").join(rel),
                &name,
            )?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        drop(writer);
        let _ = fs::remove_file(&output);
        return Err(error);
    }
    writer.finish()?;
    let mut reader = fs::File::open(&output)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let digest = hasher.finalize();
    let hash = digest.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let file_name = output
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid output filename")?;
    let checksum = output.with_file_name(format!("{file_name}.sha256"));
    if checksum.exists() {
        return Err("checksum already exists".into());
    }
    fs::write(&checksum, format!("{hash}  {file_name}\n"))?;
    println!("Created {}", output.display());
    println!("SHA-256 {hash}");
    Ok(())
}

fn run(args: &[String]) -> Result<()> {
    let (root, commands) = workspace(args)?;
    if commands == ["--version"]
        || commands == ["-V"]
        || commands == ["version"]
    {
        print_version(false);
        return Ok(());
    }
    if commands == ["version", "--verbose"] {
        print_version(true);
        return Ok(());
    }
    let profiles = root.join("profiles");
    match commands.as_slice() {
        [cmd] if cmd == "list" => {
            for p in profile::discover(&profiles)? {
                println!("{}: {}", p.metadata.id, p.metadata.name);
            }
        }
        [cmd, id] if cmd == "init" => {
            fs::create_dir_all(&profiles)?;
            profile::init(&profiles, id)?;
            println!("Initialized {id}");
        }
        [cmd, id, path, flag] if cmd == "add" && flag == "--reviewed" => {
            profile::add(&profiles.join(id), Path::new(path))?;
            println!("Registered fixture in {id}");
        }
        [cmd, id] if cmd == "validate" => {
            let p = profile::load(&profiles.join(id))?;
            println!(
                "{}: {} fixtures validated",
                p.metadata.id,
                profile::validate(&p)?
            );
        }
        [cmd] if cmd == "validate-all" => {
            let found = profile::discover(&profiles)?;
            if found.is_empty() {
                return Err("no profiles found".into());
            }
            for p in found {
                println!(
                    "{}: {} fixtures validated",
                    p.metadata.id,
                    profile::validate(&p)?
                );
            }
        }
        [cmd, id, output] if cmd == "package" => {
            package(&profiles.join(id), Path::new(output))?;
        }
        _ => return Err(usage().into()),
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
