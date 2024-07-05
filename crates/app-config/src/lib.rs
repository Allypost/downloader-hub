pub mod cli;
pub mod common;
pub mod conditional;

use std::{env, path::PathBuf};

use clap::Parser;
use cli::CliArgs;
use common::DumpConfigType;
use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use validator::Validate;

static CONFIG: Lazy<Config> = Lazy::new(Config::new);

pub static APPLICATION_NAME: &str = "downloader-hub";
pub static ORGANIZATION_NAME: &str = "allypost";
pub static ORGANIZATION_QUALIFIER: &str = "net";

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
pub struct Config {
    #[serde(skip)]
    #[validate(nested)]
    pub run: common::RunConfig,

    /// Path to various programs used by the application at runtime
    #[validate(nested)]
    pub dependency_paths: common::ProgramPathConfig,

    /// Specifying external endpoints that the application will use
    #[validate(nested)]
    pub endpoint: common::EndpointConfig,

    #[validate(nested)]
    pub conditional: conditional::ConditionalConfig,
}
impl Config {
    #[must_use]
    #[inline]
    pub fn global() -> &'static Self {
        &CONFIG
    }

    #[must_use]
    #[inline]
    pub fn config_dir() -> Option<PathBuf> {
        Self::get_project_dir().map(|x| x.config_dir().into())
    }

    #[must_use]
    #[inline]
    pub fn get_config_dir(&self) -> Option<PathBuf> {
        Self::config_dir()
    }

    #[must_use]
    #[inline]
    pub fn cache_dir() -> PathBuf {
        Self::get_project_dir().map_or_else(
            || env::temp_dir().join(APPLICATION_NAME),
            |x| x.cache_dir().into(),
        )
    }

    #[cfg(feature = "cli")]
    #[must_use]
    #[inline]
    pub const fn cli(&self) -> &conditional::cli::CliConfig {
        &self.conditional.cli
    }

    #[cfg(feature = "server")]
    #[must_use]
    #[inline]
    pub const fn server(&self) -> &conditional::server::ServerConfig {
        &self.conditional.server
    }

    #[must_use]
    #[inline]
    pub fn get_cache_dir(&self) -> PathBuf {
        Self::cache_dir()
    }

    pub fn dump_config_if_needed<T>(data: &T, dump_type: &Option<Option<DumpConfigType>>)
    where
        T: Serialize + ?Sized,
    {
        match dump_type {
            Some(dump_type) => {
                let out = match dump_type {
                    None | Some(DumpConfigType::Json) => serde_json::to_string_pretty(data)
                        .expect("Failed to serialize config to JSON"),

                    Some(DumpConfigType::Toml) => {
                        toml::to_string_pretty(data).expect("Failed to serialize config to TOML")
                    }
                };

                println!("{}", out.trim());
                std::process::exit(0);
            }
            None => (),
        }
    }

    #[inline]
    pub fn validate_config_and_exit<T: Validate>(conf: T) -> T {
        if let Err(e) = conf.validate() {
            eprintln!("Errors validating configuration:");
            print_validation_errors(&e, "  ", 1);
            std::process::exit(1);
        }

        conf
    }

    fn new() -> Self {
        let args = CliArgs::parse();

        Self::default()
            .merge_with_cli(args)
            .resolve_paths()
            .validate_self()
            .dump_if_needed()
    }

    fn merge_with_cli(mut self, args: CliArgs) -> Self {
        self.run = args.run;
        self.dependency_paths = args.dependency_path;
        self.endpoint = args.endpoint;
        self.conditional = args.conditional;

        self
    }

    fn resolve_paths(mut self) -> Self {
        self.dependency_paths = self.dependency_paths.resolve_paths();

        self
    }

    fn dump_if_needed(self) -> Self {
        Self::dump_config_if_needed(&self, &self.run.dump_config);
        self
    }

    #[inline]
    fn validate_self(self) -> Self {
        Self::validate_config_and_exit(self)
    }

    #[inline]
    fn get_project_dir() -> Option<ProjectDirs> {
        ProjectDirs::from(ORGANIZATION_QUALIFIER, ORGANIZATION_NAME, APPLICATION_NAME)
    }
}

pub fn print_validation_errors(e: &validator::ValidationErrors, prefix: &str, level: usize) {
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
                print_validation_errors(e, prefix, level + 1);
            }

            validator::ValidationErrorsKind::List(e) => {
                eprintln!("{}{}:", prefix, e_name);
                for e in e.values() {
                    print_validation_errors(e, prefix, level + 1);
                }
            }
        }
    }
}
