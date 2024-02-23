mod cli;
pub(crate) mod common;

use std::{env, path::PathBuf};

use clap::Parser;
use cli::CliArgs;
use common::DumpConfigType;
use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub static CONFIG: Lazy<Config> = Lazy::new(Config::new);

pub static APPLICATION_NAME: &str = "downloader-hub";
pub static ORGANIZATION_NAME: &str = "allypost";
pub static ORGANIZATION_QUALIFIER: &str = "net";

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
pub struct Config {
    #[serde(skip)]
    #[validate]
    pub run: common::RunConfig,

    /// Path to various programs used by the application at runtime
    #[validate]
    pub dependency_paths: common::ProgramPathConfig,

    /// Server configuration
    #[validate]
    pub server: common::ServerConfig,

    /// Specifying external endpoints that the application will use
    #[validate]
    pub endpoint: common::EndpointConfig,

    /// Database configuration
    #[validate]
    pub database: common::DatabaseConfig,

    /// Application config
    #[validate]
    pub app: common::AppConfig,
}
impl Config {
    #[must_use]
    pub fn global() -> &'static Self {
        &CONFIG
    }

    #[must_use]
    pub fn config_dir() -> Option<PathBuf> {
        Self::get_project_dir().map(|x| x.config_dir().into())
    }

    #[must_use]
    pub fn get_config_dir(&self) -> Option<PathBuf> {
        Self::config_dir()
    }

    #[must_use]
    pub fn cache_dir() -> PathBuf {
        Self::get_project_dir().map_or_else(
            || env::temp_dir().join(APPLICATION_NAME),
            |x| x.cache_dir().into(),
        )
    }

    #[must_use]
    pub fn get_cache_dir(&self) -> PathBuf {
        Self::cache_dir()
    }

    fn new() -> Self {
        let args = CliArgs::parse();

        Self::default()
            .merge_with_cli(args)
            .resolve_paths()
            .dump_if_needed()
            .validate_self()
    }

    fn merge_with_cli(mut self, args: CliArgs) -> Self {
        self.run = args.run;
        self.dependency_paths = args.dependency_path;
        self.server = args.server;
        self.endpoint = args.endpoint;
        self.database = args.database;
        self.app = args.app;

        self
    }

    fn resolve_paths(mut self) -> Self {
        self.dependency_paths = self.dependency_paths.resolve_paths();

        self
    }

    fn dump_if_needed(self) -> Self {
        match &self.run.dump_config {
            Some(dump_config_type) => {
                let out = match dump_config_type {
                    None | Some(DumpConfigType::Json) => serde_json::to_string_pretty(&self)
                        .expect("Failed to serialize config to JSON"),

                    Some(DumpConfigType::Toml) => {
                        toml::to_string_pretty(&self).expect("Failed to serialize config to TOML")
                    }
                };

                println!("{}", out.trim());

                std::process::exit(0);
            }
            None => self,
        }
    }

    fn validate_self(self) -> Self {
        fn print_errors(e: &validator::ValidationErrors, prefix: &str, level: usize) {
            let level = level.max(1);
            for (e_name, e) in e.errors() {
                match e {
                    validator::ValidationErrorsKind::Field(e) => {
                        let prefix_rep = prefix.repeat(level);
                        eprintln!(
                            "{prefix_rep}{e_name}:\n{}",
                            e.iter()
                                .map(|x| format!("{} {:?}", x.code, x.params))
                                .fold(String::new(), |acc, a| format!(
                                    "{acc}{prefix_rep}{prefix}- {a}\n"
                                ))
                                .trim_end()
                        );
                    }

                    validator::ValidationErrorsKind::Struct(e) => {
                        eprintln!("{}{}:", prefix, e_name);
                        print_errors(e, prefix, level + 1);
                    }

                    validator::ValidationErrorsKind::List(e) => {
                        eprintln!("{}{}:", prefix, e_name);
                        for e in e.values() {
                            print_errors(e, prefix, level + 1);
                        }
                    }
                }
            }
        }

        if let Err(e) = self.validate() {
            eprintln!("Errors validating configuration:");
            print_errors(&e, "  ", 1);
            std::process::exit(1);
        }

        self
    }

    fn get_project_dir() -> Option<ProjectDirs> {
        ProjectDirs::from(ORGANIZATION_QUALIFIER, ORGANIZATION_NAME, APPLICATION_NAME)
    }
}
