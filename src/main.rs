#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod cli;
mod config;
mod ignore;

use clap::{IntoApp, Parser};
use cli::{print_completion, AliasCmd, Cmds, TemplateCmd, CLI};
use colored::*;
use config::Config;
use ignore::{project_dirs, GitIgnore};
use std::path::PathBuf;

macro_rules! config_or {
    ($sel:ident, $fun:ident) => {{
        if let Some(config) = $sel.config {
            config.$fun();
        } else {
            eprintln!("{}", "No config found".bold().yellow());
        }

        return Ok(());
    }};
    ($sel:ident, $fun:ident, $name:expr) => {{
        if let Some(mut config) = $sel.config {
            config.$fun($name)?;
        } else {
            eprintln!("{}", "No config found".bold().yellow());
        }

        return Ok(());
    }};
    ($sel:ident, $fun:ident, $name:expr, $vals:expr) => {{
        if let Some(mut config) = $sel.config {
            config.$fun($name, $vals)?;
        } else {
            eprintln!("{}", "No config found".bold().yellow());
        }

        return Ok(());
    }};
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = CLI::parse();
    let app = GitIgnore::new();

    match opt.cmd {
        Some(Cmds::Init { .. }) => {
            let dirs = project_dirs();

            let config_file: PathBuf = [
                dirs.config_dir()
                    .to_str()
                    .expect("Could not unwrap config directory"),
                "config.toml",
            ]
            .iter()
            .collect();

            Config::create(&config_file)?;
            return Ok(());
        }
        Some(Cmds::Alias(cmd)) => match cmd {
            AliasCmd::List => config_or!(app, list_aliases),
            AliasCmd::Add { name, aliases } => config_or!(app, add_alias, name, aliases),
            AliasCmd::Remove { name } => config_or!(app, remove_alias, name),
        },
        Some(Cmds::Template(cmd)) => match cmd {
            TemplateCmd::List => config_or!(app, list_templates),
            TemplateCmd::Add { name, path } => config_or!(app, add_template, name, path),
            TemplateCmd::Remove { name } => config_or!(app, remove_template, name),
        },
        Some(Cmds::Completion { shell }) => {
            let mut app = CLI::into_app();
            print_completion(shell, &mut app);
            return Ok(());
        }
        _ => {}
    };

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
    } else if opt.templates.is_empty() {
        let mut app = CLI::into_app();
        app.print_help()?;
    } else {
        app.get_templates(&opt.templates)?;
    }

    Ok(())
}
