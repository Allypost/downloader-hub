use std::{fmt::Debug, path::PathBuf, result::Result};

use app_config::Config;
use app_downloader::downloaders::DownloadFileRequest;
use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() {
    init_log();

    let config = Config::global();

    app_logger::debug!(config = ?*config, "Running with config");

    let urls = get_explicit_urls();
    let mut urls = print_errors("urls", urls);

    let files = get_explicit_files();
    let mut files = print_errors("files", files);

    let cli_config = config.cli();

    for x in &cli_config.entries_group.urls_or_files {
        let mut errs = vec![];

        match parse_url(x) {
            Ok(url) => {
                urls.push(url);
                continue;
            }
            Err(e) => errs.push(e),
        }

        match parse_file(x) {
            Ok(file) => {
                files.push(file);
                continue;
            }
            Err(e) => errs.push(e),
        }

        app_logger::warn!("Failed to parse {x:?} as URL or file: {errs:?}");
    }

    let urls = urls.into_iter().map(|x| x.to_string()).collect::<Vec<_>>();

    app_logger::debug!(urls = ?urls, files = ?files, "Parsed urls and files");

    app_logger::info!("Outputting to {:?}", cli_config.output_directory);

    app_logger::info!("Starting download");
    let downloaded_urls = {
        let downloaded_urls = urls
            .into_iter()
            .map(|url| async move {
                tokio::task::spawn_blocking(move || {
                    app_downloader::download_file(&DownloadFileRequest::new(
                        &url,
                        &cli_config.output_directory,
                    ))
                    .into_iter()
                    .map(|x| x.map_err(|e| (url.to_string(), e)))
                    .collect::<Vec<_>>()
                })
                .await
            })
            .collect::<Vec<_>>();

        futures::future::join_all(downloaded_urls)
            .await
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>()
    };
    app_logger::debug!(urls = ?downloaded_urls, "Downloaded urls");

    let (downloaded, failed_downloaded) = split_vec_err(downloaded_urls);
    app_logger::info!(
        "Download completed: downloaded {} files, failed to download {} files",
        downloaded.len(),
        failed_downloaded.len()
    );

    let to_fix = downloaded
        .into_iter()
        .map(|x| x.path)
        .chain(files.clone())
        .collect::<Vec<_>>();

    app_logger::debug!(files = ?to_fix, "Files to fix");
    app_logger::info!("Starting fixing of {} files", to_fix.len());
    let fixed_files = {
        let fixed_files = to_fix
            .into_iter()
            .map(|x| async move {
                app_fixers::fix_file(&x)
                    .await
                    .map(|n| (x.clone(), n))
                    .map_err(|e| (x, e))
            })
            .collect::<Vec<_>>();

        futures::future::join_all(fixed_files).await
    };

    let (fixed, failed_fixed) = split_vec_err(fixed_files);
    app_logger::info!(
        "Fixing completed: fixed {} files, failed to fix {} files",
        fixed.len(),
        failed_fixed.len()
    );

    if !failed_downloaded.is_empty() || !failed_fixed.is_empty() {
        for (x, e) in failed_downloaded {
            app_logger::error!("Failed to download {x:?}: {e}");
        }

        for (x, e) in failed_fixed {
            app_logger::error!("Failed to fix {x:?}: {e}");
        }

        std::process::exit(1);
    }
}

fn split_vec_err<T: Debug, E: Debug>(v: Vec<Result<T, E>>) -> (Vec<T>, Vec<E>) {
    let (ok, err) = v.into_iter().partition::<Vec<_>, _>(Result::is_ok);
    (
        ok.into_iter().map(Result::unwrap).collect(),
        err.into_iter().map(Result::unwrap_err).collect(),
    )
}

fn print_errors<T: Sized>(name: &str, maybe_errors: Vec<Result<T, String>>) -> Vec<T> {
    let errors = maybe_errors
        .iter()
        .filter_map(|maybe_err| match maybe_err {
            Ok(_) => None,
            Err(err) => Some(err),
        })
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        let err = format!("Errors parsing {}", name);
        app_logger::error!(err);
        for error in errors {
            app_logger::error!("{error}");
        }
        std::process::exit(1);
    }

    maybe_errors
        .into_iter()
        .filter_map(|maybe_err| maybe_err.map_or_else(|_| None, |x| Some(x)))
        .collect()
}

fn get_explicit_urls() -> Vec<Result<url::Url, String>> {
    Config::global()
        .cli()
        .entries_group
        .urls
        .iter()
        .map(|x| (x, parse_url(x)))
        .map(|(u, maybe_err)| match maybe_err {
            Ok(x) => Ok(x),
            Err(err) => Err(format!("Failed to parse {u:?} as URL: {err}")),
        })
        .collect::<Vec<_>>()
}

fn parse_url(u: &str) -> Result<url::Url, String> {
    url::Url::parse(u).map_err(|x| x.to_string())
}

fn get_explicit_files() -> Vec<Result<PathBuf, String>> {
    Config::global()
        .cli()
        .entries_group
        .files
        .iter()
        .map(|x| (x, parse_file(x)))
        .map(|(f, maybe_err)| match maybe_err {
            Ok(x) => Ok(x),
            Err(err) => Err(format!("Failed to parse {f:?} as path: {err}")),
        })
        .collect::<Vec<_>>()
}

fn parse_file<T: Into<PathBuf>>(f: T) -> Result<PathBuf, String> {
    let f = f.into();

    if !f.exists() {
        return Err("File does not exist".to_string());
    }

    if !f.is_file() {
        return Err("Is not a file".to_string());
    }

    Ok(f)
}

fn init_log() {
    tracing_subscriber::fmt()
        .with_ansi(true)
        .with_env_filter(
            tracing_subscriber::filter::Builder::default()
                .with_default_directive(LevelFilter::INFO.into())
                .with_env_var("DOWNLOADER_HUB_LOG_LEVEL")
                .from_env_lossy(),
        )
        .finish()
        .init();
}
