# Downloader Hub

[![Build Status](https://drone.allypost.net/api/badges/Allypost/downloader-hub/status.svg)](https://drone.allypost.net/Allypost/downloader-hub)

Downloader Hub: [![Downloader Hub Image Size](https://img.shields.io/docker/image-size/allypost/downloader-hub)](https://hub.docker.com/r/allypost/downloader-hub)

Telegram Bot: [![Downloader Telegram Bot Image Size](https://img.shields.io/docker/image-size/allypost/downloader-telegram-bot)](https://hub.docker.com/r/allypost/downloader-telegram-bot)


A simple hub to allow clients to download files from various sources.

Downloaded files are also processed and converted to standard formats.

Clients/apps each get an API key they use to authenticate and have a defined download location (eg. a local folder).

The hub can also provide timed public file links for clients to download or share.

Currently there is no UI for the hub. All interactions are done via the API.

## Getting started

### Runtime dependencies

The app requires some external dependencies to effectively download and process files:

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) | Download and process videos
- [ffmpeg](https://ffmpeg.org/) | Convert videos to standard formats
- [ffprobe](https://ffmpeg.org/ffprobe.html)
- [scenedetect](https://scenedetect.com) (optional)

A [PostgreSQL](https://www.postgresql.org/) database is also required for the hub.

### Usage

```text
A hub for downloading media from various platforms, process the results and aggregate them in one place

Usage: downloader-hub [OPTIONS] --admin-key <ADMIN_KEY> --signing-key <SIGNING_KEY> --database-url <URL> --public-url <PUBLIC_URL>

Options:
      --help
          Print help

Program paths:
      --yt-dlp-path <YT_DLP_PATH>
          Path to the yt-dlp executable.
          
          If not provided, yt-dlp will be searched for in $PATH
          
          [env: DOWNLOADER_HUB_YT_DLP=]

      --ffmpeg-path <FFMPEG_PATH>
          Path to the ffmpeg executable.
          
          If not provided, ffmpeg will be searched for in $PATH
          
          [env: DOWNLOADER_HUB_FFMPEG=]

      --ffprobe-path <FFPROBE_PATH>
          Path to the ffprobe executable.
          
          If not provided, ffprobe will be searched for in $PATH
          
          [env: DOWNLOADER_HUB_FFPROBE=]

      --scenedetect-path <SCENEDETECT_PATH>
          Path to the scenedetect executable.
          
          If not provided, scenedetect will be searched for in $PATH
          
          [env: DOWNLOADER_HUB_SCENEDETECT=]

External endpoints/APIs:
      --twitter-screenshot-base-url <TWITTER_SCREENSHOT_BASE_URL>
          The base URL for the Twitter screenshot API
          
          [env: DOWNLOADER_HUB_ENDPOINT_TWITTER_SCREENSHOT=]
          [default: https://twitter.igr.ec]

Run options:
      --dump-config [<DUMP_CONFIG>]
          Dump the config to stdout
          
          [possible values: json, toml]

Server options:
      --port <PORT>
          The port on which the server will listen
          
          [env: PORT=]
          [default: 8000]

      --host <HOST>
          The host on which the server will listen
          
          [env: HOST=]
          [default: 127.0.0.1]

      --admin-key <ADMIN_KEY>
          The admin key for the server. Used to authenticate admin requests. Should be at least 32 characters long and securely random
          
          [env: DOWNLOADER_HUB_ADMIN_KEY=]

      --signing-key <SIGNING_KEY>
          The key used for signing various tokens. Should be at least 32 characters long and securely random
          
          [env: DOWNLOADER_HUB_SIGNING_KEY=]

Database options:
      --database-url <URL>
          PostgreSQL database URL.
          
          Should be in the format of `postgres://username:password@db-host:5432/database-name`
          
          [env: DATABASE_URL=]

Application options:
      --public-url <PUBLIC_URL>
          The public URL where the application is served. This is used to generate links to the application. Should be in the format of `https://www.example.com/some/path` or `http://127.0.0.1:8000`
          
          [env: DOWNLOADER_HUB_PUBLIC_URL=]
```
