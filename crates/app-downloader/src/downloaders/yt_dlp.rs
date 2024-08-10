use std::{
    io::Write,
    ops::Sub,
    path::PathBuf,
    process,
    time::{Duration, SystemTime},
};

use app_config::Config;
use app_helpers::{id::time_id, temp_dir::TempDir, temp_file::TempFile};
use app_logger::{debug, trace};
use http::header;
use url::Url;

use super::{
    DownloadFileRequest, DownloadResult, DownloadUrlInfo, Downloader, ResolvedDownloadFileRequest,
};
use crate::{common::USER_AGENT, downloaders::generic, DownloaderReturn};

#[derive(Debug, Default)]
pub struct YtDlpDownloader;

#[async_trait::async_trait]
impl Downloader for YtDlpDownloader {
    fn name(&self) -> &'static str {
        "yt-dlp"
    }

    fn description(&self) -> &'static str {
        "Downloads videos and images using yt-dlp. Supports a wide range of sites."
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        Ok(ResolvedDownloadFileRequest {
            request_info: req.clone(),
            resolved_urls: vec![DownloadUrlInfo::from_url(&req.original_url)],
        })
    }

    fn download_resolved(&self, resolved_file: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        resolved_file
            .resolved_urls
            .iter()
            .map(|x| self.download_one(&resolved_file.request_info, x))
            .collect()
    }
}

impl YtDlpDownloader {
    #[allow(clippy::too_many_lines)]
    pub fn download_one(
        &self,
        request_info: &DownloadFileRequest,
        url: &DownloadUrlInfo,
    ) -> Result<DownloadResult, String> {
        let yt_dlp = Config::global().dependency_paths.yt_dlp_path();
        trace!("`yt-dlp' binary: {:?}", &yt_dlp);
        let temp_dir = TempDir::in_tmp_with_prefix("downloader-hub_yt-dlp-")
            .map_err(|e| format!("Failed to create temporary directory for yt-dlp: {e:?}"))?;
        let output_template = get_output_template(temp_dir.path());

        let parsed_url = Url::parse(url.url()).expect("Failed to parse URL");
        let host_str = parsed_url.host_str().unwrap_or_default();
        let in_a_year = SystemTime::now()
            .checked_add(Duration::from_secs(60 * 60 * 24 * 365))
            .unwrap_or_else(SystemTime::now)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs();

        let cookie_values = url
            .headers()
            .get_all(header::COOKIE)
            .into_iter()
            .flat_map(|x| x.to_str())
            .flat_map(|x| {
                x.split("; ")
                    .map(|x| x.splitn(2, '=').collect::<Vec<&str>>())
                    .filter(|x| x.len() == 2)
                    .map(|x| (x[0].trim(), x[1].trim()))
                    .map(|(k, v)| {
                        format!(
                            "{host}\tFALSE\t/\tTRUE\t{expires}\t{k}\t{v}",
                            host = host_str,
                            expires = in_a_year,
                        )
                    })
            })
            .collect::<Vec<String>>();

        debug!("template: {:?}", &output_template);
        let mut cmd = process::Command::new(yt_dlp);
        let cmd = {
            let mut cmd = cmd
                .arg("--no-check-certificate")
                .args(["--socket-timeout", "120"])
                .arg("--no-part")
                .arg("--no-mtime")
                .arg("--no-embed-metadata")
                .arg("--no-config");

            if !cookie_values.is_empty() {
                debug!("Adding cookie headers: {:?}", &cookie_values);

                let mut cookie_file = TempFile::with_prefix("cookie-headers-").map_err(|e| {
                    format!("Failed to create temporary file for yt-dlp cookie headers: {e:?}")
                })?;

                cookie_file
                    .file_mut()
                    .write_all(
                        format!(
                            "# Netscape HTTP Cookie File\n{cookie_values}\n",
                            cookie_values = cookie_values.join("\n")
                        )
                        .as_bytes(),
                    )
                    .map_err(|e| format!("Failed to write cookie headers to file: {e:?}"))?;

                cmd = cmd.arg("--cookies").arg(cookie_file.path());
            }

            cmd = cmd
                .args([
                    "--trim-filenames",
                    generic::MAX_FILENAME_LENGTH.sub(5).to_string().as_str(),
                ])
                .args(
                    url.headers()
                        .iter()
                        .filter(|x| x.0 != header::COOKIE)
                        .flat_map(|(k, v)| {
                            vec![
                                "--add-header".to_string(),
                                format!("{k}:{v}", k = k, v = v.to_str().unwrap_or_default()),
                            ]
                        }),
                )
                .args([
                    "--output",
                    output_template
                        .to_str()
                        .ok_or_else(|| "Failed to convert path to string".to_string())?,
                ])
                .args(["--user-agent", USER_AGENT])
                .args(["--no-simulate", "--print", "after_move:filepath"])
                // .arg("--verbose")
                .arg(url.url());

            cmd
        };
        debug!("Running cmd: {:?}", &cmd);
        let cmd_output = cmd.output();
        trace!("Cmd output: {:?}", &cmd_output);
        let mut err = String::new();
        let new_file_path = match cmd_output {
            Ok(process::Output {
                stdout,
                stderr: _,
                status,
            }) if status.success() => {
                let output = String::from_utf8(stdout)
                    .map_err(|e| format!("Failed to convert output to UTF-8: {e:?}"))?;
                let output_path = PathBuf::from(output.trim());

                if !output_path.exists() {
                    return Err("yt-dlp finished but file does not exist.".to_string());
                }

                debug!("yt-dlp successful download to file: {:?}", output_path);
                output_path
            }
            Ok(process::Output {
                stdout: _,
                stderr,
                status: _,
            }) if is_image_error(stderr.clone()) => {
                return generic::download_one(request_info, url)
            }
            _ => {
                let msg = format!("yt-dlp failed downloading meme: {cmd_output:?}");
                err.push_str(msg.as_str());
                return Err(err.as_str().to_string());
            }
        };

        if !new_file_path.exists() {
            return Err("yt-dlp finished but file does not exist.".to_string());
        }

        let final_file_path = request_info
            .download_dir
            .join(new_file_path.file_name().unwrap_or_default());

        std::fs::copy(&new_file_path, &final_file_path).map_err(|e| {
            format!("Failed to copy file from {new_file_path:?} to {final_file_path:?}: {e:?}")
        })?;

        Ok(DownloadResult {
            request: request_info.clone(),
            path: final_file_path,
        })
    }
}

#[must_use]
pub fn download(req: &DownloadFileRequest) -> DownloaderReturn {
    YtDlpDownloader.download(req)
}

fn get_output_template<S: Into<PathBuf>>(download_dir: S) -> PathBuf {
    let file_identifier = time_id();
    let file_name = format!("{file_identifier}.%(id).64s.%(ext)s");

    download_dir.into().join(file_name)
}

fn is_image_error(output: Vec<u8>) -> bool {
    let output = String::from_utf8(output).unwrap_or_default();
    let output = output.trim();

    trace!("yt-dlp output: {output}");

    output.ends_with(". Maybe an image?")
}
