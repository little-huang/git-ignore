//! # git-ignore [![Build Status](https://travis-ci.com/sondr3/git-ignore.svg?token=jVZ9BLfdPx6kBm4z8gXS&branch=master)](https://travis-ci.com/sondr3/git-ignore) [![Crates.io](https://img.shields.io/crates/v/git-ignore-generator.svg)](https://crates.io/crates/git-ignore-generator)
//!
//! ## What and why
//!
//! Far too often I find myself going to [gitignore.io](https://www.gitignore.io/)
//! to quickly get `.gitignore` templates for my projects, so what would any
//! reasonable programmer do for repetitive tasks?
//! [Automate](https://xkcd.com/1319/) [it](https://xkcd.com/1205/)! Now you can
//! quickly and easily get and list all the templates available on gitignore.io, it
//! even works offline by caching the templates!
//!
//! # Installation
//!
//! There are two ways of installing it, either via Cargo (easiest) or via Nix
//! (authors preference). See installation and usage instructions below.
//!
//! ## Cargo
//!
//! Make sure you have Rust installed (I recommend installing via
//! [rustup](https://rustup.rs/)), then run `cargo install git-ignore-generator`.
//!
//! ## Nix
//!
//! Run `nix-env -iA nixpkgs.gitAndTools.git-ignore`. This version also includes man
//! pages.
//!
//! # Usage
//!
//! To download and cache all available templates, use `--update`. This can also be
//! used in combination with any of the other flags/arguments, or be ran as a
//! standalone flag.
//!
//! ``` sh
//! $ git ignore -u
//! ```
//!
//! To list all the available templates:
//!
//! ```sh
//! $ git ignore --list
//! [
//!     "1c",
//!     "1c-bitrix",
//!     "a-frame",
//!     "actionscript",
//!     "ada",
//!     [...],
//!     "zukencr8000"
//! ]
//! ```
//!
//! You can also search for templates with the `--list` flag. **Note**: Listing
//! templates doesn't require exact matches, any template matching the start of your
//! query will be matched. See the example below for this, `intellij` matches all
//! three templates starting with `intellij`:
//!
//! ```sh
//! $ git ignore rust intellij --list
//! [
//!     "intellij",
//!     "intellij+all",
//!     "intellij+iml",
//!     "rust"
//! ]
//! ```
//!
//! Then you can print the templates by omitting `--list`. **Note:** While listing
//! do not require exact matches, printing templates do. Use `--list` to find
//! templates. There will also be a notice about using cached results, this is
//! printed to `stderr` as to not interfere with piping.
//!
//! ```sh
//! $ git ignore rust intellij+all
//!
//! ### Created by https://www.gitignore.io
//! ### Rust ###
//!
//! [...]
//!
//! # These are backup files generated by rustfmt
//! **/*.rs.bk
//! ```
//!
//! Finally, help is always available with `git ignore -h` (or `--help` if you used
//! Nix).
#![forbid(unsafe_code)]

use colored::*;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use structopt::{clap::AppSettings, StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(
    name = "git ignore",
    global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp]
)]
/// Quickly and easily add templates to .gitignore
struct Opt {
    /// List <templates> or all available templates.
    #[structopt(short, long)]
    list: bool,
    /// Update templates by fetching them from gitignore.io
    #[structopt(short, long)]
    update: bool,
    /// Names of templates to show/search for
    #[structopt(required = false)]
    templates: Vec<String>,
}

#[derive(Debug)]
struct GitIgnore {
    server: String,
    cache_dir: PathBuf,
    ignore_file: PathBuf,
}

#[derive(Deserialize, Serialize, Debug)]
struct Language {
    key: String,
    name: String,
    #[serde(rename = "fileName")]
    file_name: String,
    contents: String,
}

impl GitIgnore {
    /// Creates a new instance of the `git-ignore` program. Thanks to
    /// `directories` we support crossplatform caching of our results, the cache
    /// directories works on macOS, Linux and Windows. See the documentation for
    /// their locations.
    fn new() -> Self {
        let proj_dir = ProjectDirs::from("com", "Sondre Nilsen", "git-ignore")
            .expect("Could not find project directory.");

        let cache_dir: PathBuf = proj_dir.cache_dir().into();
        let ignore_file: PathBuf = [
            cache_dir
                .to_str()
                .expect("Could not unwrap cache directory."),
            "ignore.json",
        ]
        .iter()
        .collect();

        GitIgnore {
            server: "https://www.gitignore.io/api/list?format=json".into(),
            cache_dir,
            ignore_file,
        }
    }

    /// Returns true if the cache directory or `ignore.json` file exists, false
    /// otherwise.
    fn cache_exists(&self) -> bool {
        self.cache_dir.exists() || self.ignore_file.exists()
    }

    /// Creates the cache dir if it doesn't exist.
    fn create_cache_dir(&self) -> std::io::Result<()> {
        if !self.cache_exists() {
            std::fs::create_dir_all(&self.cache_dir)?;
        }

        Ok(())
    }

    /// Both updates and initializes `git-ignore`. Creates the cache directory
    /// if it doesn't exist and then downloads the templates from
    /// [gitignore.io](https://www.gitignore.io), saving them in the cache
    /// directory.
    fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.create_cache_dir()?;
        self.fetch_gitignore()?;

        eprintln!("{}: Update successful", "Info".bold().green());
        Ok(())
    }

    /// Fetches all the templates from [gitignore.io](http://gitignore.io/),
    /// and writes the contents to the cache for easy future retrieval.
    fn fetch_gitignore(&self) -> Result<(), Box<dyn std::error::Error>> {
        let res = attohttpc::get(&self.server).send()?;

        let mut file = File::create(&self.ignore_file)?;
        file.write_all(&res.bytes()?)?;

        Ok(())
    }

    /// Iterates over the HashMap from `read_file` and either filters out
    /// entries not in the `names` Vector or adds all of them, finally sorting
    /// them for consistent output.
    fn get_template_names(
        &self,
        names: &[String],
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let templates = self.read_file()?;
        let result = {
            let mut result: Vec<String> = Vec::new();

            for entry in templates.keys() {
                if names.is_empty() {
                    result.push(entry.to_string());
                } else {
                    for name in names {
                        if entry.starts_with(name) {
                            result.push(entry.to_string())
                        }
                    }
                }
            }

            result.sort_unstable();
            result
        };

        Ok(result)
    }

    /// Writes the `content` field for each entry in templates from `read_file`
    /// to `stdout`.
    fn get_templates(&self, names: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let mut templates = self.read_file()?;
        templates.retain(|k, _| names.contains(k));
        let mut result = String::new();

        for language in templates.values() {
            result.push_str(&language.contents);
        }

        if !result.is_empty() {
            let mut header = "\n\n### Created by https://www.gitignore.io".to_string();
            header.push_str(&result);
            result = header;
        }

        println!("{}", result);
        Ok(())
    }

    /// Reads the `ignore.json` and serializes it using Serde to a HashMap where
    /// the keys are each individual template and the value the contents (and
    /// some other stuff).
    fn read_file(&self) -> Result<HashMap<String, Language>, Box<dyn std::error::Error>> {
        let file = Path::new(&self.ignore_file);
        let file = read_to_string(file)?;

        let result: HashMap<String, Language> = serde_json::from_str(&file)?;
        Ok(result)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let app = GitIgnore::new();
    if opt.update {
        app.update()?;
    } else if !app.cache_exists() {
        eprintln!(
            "{}: Cache directory or ignore file not found, attempting update.",
            "Warning".bold().red(),
        );
        app.update()?;
    } else {
        eprintln!(
            "{}: You are using cached results, pass '-u' to update the cache\n",
            "Info".bold().green(),
        );
    }

    if opt.list {
        println!("{:#?}", app.get_template_names(&opt.templates)?);
    } else {
        app.get_templates(&opt.templates)?;
    }

    Ok(())
}
