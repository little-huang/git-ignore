use crate::config::Config;
use colored::Colorize;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{read_to_string, File},
    io::Write,
    path::{Path, PathBuf},
};

pub fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "Sondre Nilsen", "git-ignore")
        .expect("Could not find project directory.")
}

#[derive(Debug)]
pub struct Core {
    server: String,
    cache_dir: PathBuf,
    ignore_file: PathBuf,
    pub config: Option<Config>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Language {
    key: String,
    name: String,
    #[serde(rename = "fileName")]
    file_name: String,
    contents: String,
}

impl Core {
    /// Creates a new instance of the `git-ignore` program. Thanks to
    /// `directories` we support crossplatform caching of our results, the cache
    /// directories works on macOS, Linux and Windows. See the documentation for
    /// their locations.
    pub fn new() -> Self {
        let proj_dir = project_dirs();
        let cache_dir: PathBuf = proj_dir.cache_dir().into();
        let ignore_file: PathBuf = [
            cache_dir
                .to_str()
                .expect("Could not unwrap cache directory."),
            "ignore.json",
        ]
        .iter()
        .collect();

        let config = Config::from_dir();

        Core {
            server: "https://www.gitignore.io/api/list?format=json".into(),
            cache_dir,
            ignore_file,
            config,
        }
    }

    /// Both updates and initializes `git-ignore`. Creates the cache directory
    /// if it doesn't exist and then downloads the templates from
    /// [gitignore.io](https://www.gitignore.io), saving them in the cache
    /// directory.
    pub fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.create_dirs()?;
        self.fetch_gitignore()?;

        eprintln!("{}: Update successful", "Info".bold().green());
        Ok(())
    }

    /// Iterates over the `HashMap` from `read_file` and either filters out
    /// entries not in the `names` Vector or adds all of them, finally sorting
    /// them for consistent output.
    pub fn get_template_names(
        &self,
        names: &[String],
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let templates = self.all_names()?;
        let result = {
            let mut result: Vec<String> = Vec::new();

            for entry in templates {
                if names.is_empty() {
                    result.push(entry.to_string());
                } else {
                    for name in names {
                        if entry.contains(name) {
                            result.push(entry.to_string());
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
    pub fn get_templates(&self, names: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let aliases = match &self.config {
            Some(config) => config.aliases.clone(),
            None => HashMap::new(),
        };

        let templates = self.read_file()?;
        let mut result = String::new();

        for name in names {
            if let Some(val) = aliases.get(name) {
                for alias in val {
                    if let Some(language) = templates.get(alias) {
                        result.push_str(&language.contents);
                    }
                }
            } else if let Some(language) = templates.get(name) {
                result.push_str(&language.contents);
            }
        }

        if !result.is_empty() {
            let mut header = "\n\n### Created by https://www.gitignore.io".to_string();
            header.push_str(&result);
            result = header;
        }

        println!("{}", result);
        Ok(())
    }

    fn all_names(&self) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
        let templates = self.read_file()?;
        let config_names = match &self.config {
            Some(config) => config.names(),
            _ => vec![],
        };

        let mut combined: HashSet<String> = config_names.into_iter().collect();
        combined.extend(templates.keys().cloned());

        Ok(combined)
    }

    /// Fetches all the templates from [gitignore.io](http://gitignore.io/),
    /// and writes the contents to the cache for easy future retrieval.
    fn fetch_gitignore(&self) -> Result<(), Box<dyn std::error::Error>> {
        let res = attohttpc::get(&self.server).send()?;

        let mut file = File::create(&self.ignore_file)?;
        file.write_all(&res.bytes()?)?;

        Ok(())
    }

    /// Returns true if the cache directory or `ignore.json` file exists, false
    /// otherwise.
    pub fn cache_exists(&self) -> bool {
        self.cache_dir.exists() || self.ignore_file.exists()
    }

    /// Creates the cache dir if it doesn't exist.
    fn create_dirs(&self) -> std::io::Result<()> {
        if !self.cache_exists() {
            std::fs::create_dir_all(&self.cache_dir)?;
        }

        Ok(())
    }

    /// Reads the `ignore.json` and serializes it using Serde to a `HashMap` where
    /// the keys are each individual template and the value the contents (and
    /// some other stuff).
    fn read_file(&self) -> Result<HashMap<String, Language>, Box<dyn std::error::Error>> {
        let file = Path::new(&self.ignore_file);
        let file = read_to_string(file)?;

        let result: HashMap<String, Language> = serde_json::from_str(&file)?;
        Ok(result)
    }
}
