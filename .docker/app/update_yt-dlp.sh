#!/usr/bin/env sh
# set -ex

YT_DLP_VERSION="${YT_DLP_VERSION:-latest}"
YT_DLP_UPDATE_LOG_FILE="${YT_DLP_UPDATE_LOG_FILE:-/tmp/update_yt-dlp.log}"

curl -L "https://github.com/yt-dlp/yt-dlp/releases/${YT_DLP_VERSION}/download/yt-dlp" -o '/usr/local/bin/yt-dlp' &&
  chmod a+rx '/usr/local/bin/yt-dlp'

echo "[$(date -R)] yt-dlp updated to ${YT_DLP_VERSION}" >>"${YT_DLP_UPDATE_LOG_FILE}"
chmod a+rw "${YT_DLP_UPDATE_LOG_FILE}"
