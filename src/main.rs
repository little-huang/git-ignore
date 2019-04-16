//! # `git ignore`
//!
//! `git-ignore-generator` is a simple and easy to use way to quickly and
//! effortlessly list and grab `.gitignore` templates from
//! [gitignore.io](https://www.gitignore.io/) from the command line.
//!
//! ## What and why
//!
//! Far too often I find myself going to
//! [gitignore.io](https://www.gitignore.io/) to quickly get `.gitignore`
//! templates for my projects, so what would any reasonable programmer do for
//! menial and repetitive tasks? [Automate](https://xkcd.com/1319/)
//! [it](https://xkcd.com/1205/)! Now you can quickly and easily get and list
//! all the templates available on gitignore.io.
//!
//! # Installation
//!
//! Make sure you have Rust installed (I recommend installing via
//! [rustup](https://rustup.rs/)), then run `cargo install
//! git-ignore-generator`.
//!
//! To list all the available templates:
//!
//! ```sh
//! $ git ignore --list
//! [
//! "1c",
//! "1c-bitrix",
//! "a-frame",
//! "actionscript",
//! "ada",
//! [...],
//! "zukencr8000"
//! ]
//! ```
//!
//! You can also search for templates (`--list` can be both before and after the
//! queries):
//!
//! ```sh
//! $ git ignore rust intellij --list
//! [
//! "intellij",
//! "intellij+all",
//! "intellij+iml",
//! "rust"
//! ]
//! ```
//!
//! Then you can download the templates by omitting `--list`:
//!
//! ```sh
//! $ git ignore rust intellij+all
//!
//! # Created by https://www.gitignore.io/api/rust,intellij+all
//! # Edit at https://www.gitignore.io/?templates=rust,intellij+all
//!
//! [...]
//!
//! # These are backup files generated by rustfmt
//! **/*.rs.bk
//!
//! # End of https://www.gitignore.io/api/rust,intellij+all
//! ```
//!
//! Finally, if need be, you can always run `git ignore -h` to see more options
//! --- spoiler alert, there are none.
#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![deny(clippy::all)]
#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    unused_import_braces,
    unused_allocation
)]

use directories::ProjectDirs;
use reqwest;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(
    name = "git ignore",
    about = "Quickly and easily add templates to .gitignore",
    raw(global_settings = "&[AppSettings::ColoredHelp]")
)]
struct Opt {
    /// List available .gitignore templates
    #[structopt(short, long)]
    list: bool,
    /// Update templates from gitignore.io
    #[structopt(short, long)]
    update: bool,
    /// List of .gitignore templates to fetch/list
    #[structopt(raw(required = "false"))]
    templates: Vec<String>,
}

#[derive(Debug)]
struct GitIgnore {
    cache_dir: PathBuf,
}

impl GitIgnore {
    fn new() -> Self {
        let proj_dir = ProjectDirs::from("com", "sondr3", "git-ignore")
            .expect("Could not find project directory.");

        GitIgnore {
            cache_dir: proj_dir.cache_dir().into(),
        }
    }

    fn create_cache_dir(&self) -> std::io::Result<()> {
        if !self.cache_dir.exists() {
            std::fs::create_dir(&self.cache_dir)?;
        }
        Ok(())
    }

    fn get_gitignore_templates(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.create_cache_dir()?;

        let url = "https://www.gitignore.io/api/list";
        let mut res = reqwest::get(url)?;

        let mut response = Vec::new();
        res.read_to_end(&mut response)?;
        let response = String::from_utf8(response)?;
        let response = {
            let mut list: Vec<String> = Vec::new();
            for line in response.lines() {
                for entry in line.split(",") {
                    list.push(entry.to_string());
                }
            }

            list
        };

        Ok(response)
    }

    fn write_gitignore_list(&self) -> Result<(), Box<dyn std::error::Error>> {
        let templates = self.get_gitignore_templates()?;
        let file = self.cache_dir.to_str().unwrap();
        let file: PathBuf = [file, "list.txt"].iter().collect();
        let mut file = File::create(file)?;
        for entry in templates {
            write!(file, "{}\n", entry)?;
        }
        Ok(())
    }
}

/// Returns a list of all templates matching the names given to this function,
/// if none are passed it will display all available templates.
fn gitignore_list(templates: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://www.gitignore.io/api/list";
    let mut res = reqwest::get(url)?;

    let all = templates.is_empty();

    let mut response = Vec::new();
    res.read_to_end(&mut response)?;
    let response = String::from_utf8(response)?;
    let response = {
        let tmp = response.replace("\n", ",");
        let tmp = tmp.split(',');
        let mut list: Vec<String> = Vec::new();

        for entry in tmp {
            if all {
                list.push(entry.to_string());
            } else {
                for item in templates {
                    if entry.to_string().starts_with(item) {
                        list.push(entry.to_string());
                    }
                }
            }
        }

        list
    };
    println!("{:#?}", response);

    Ok(())
}

/// Get the `.gitignore` templates matching the supplied names.
fn get_gitignore(templates: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let url = {
        let mut tmp = "https://www.gitignore.io/api/".to_string();
        for entry in templates {
            tmp.push_str(entry);
            tmp.push_str(",");
        }
        let len = tmp.len() - 1;
        tmp.remove(len);

        tmp
    };

    let mut res = reqwest::get(&url)?;
    std::io::copy(&mut res, &mut std::io::stdout())?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let app = GitIgnore::new();
    app.get_gitignore_templates()?;
    if opt.list {
        gitignore_list(&opt.templates)?;
    } else if opt.update {
        app.write_gitignore_list()?;
    } else {
        get_gitignore(&opt.templates)?;
    }

    Ok(())
}
