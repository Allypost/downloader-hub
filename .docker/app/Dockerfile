ARG RUST_VERSION='1.76'
ARG RUST_TARGET='x86_64-unknown-linux-musl'
ARG BINARY_NAME='downloader-hub'

ARG RUN_USERNAME='app'
ARG RUN_USER_ID='1000'
ARG RUN_GROUP_ID='1000'

ARG FFMPEG_DOWNLOAD_URL='https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz'

##########
# Step 0 #
##########
##
## Setup base image with cargo-chef
##
FROM rust:${RUST_VERSION} AS chef
# `curl` and `bash` are needed for cargo-binstall
# `musl-tools` and `musl-dev` are needed to build app with musl target
RUN apt-get update && apt-get install -y curl bash musl-tools musl-dev
# Install cargo-binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf 'https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh' | bash
# Install cargo-chef
RUN cargo binstall cargo-chef --locked --no-confirm
# Add proper target and compile flags
ARG RUST_TARGET
RUN rustup target add "${RUST_TARGET}"
ENV RUSTFLAGS='-C target-feature=+crt-static'
WORKDIR /app


##########
# Step 1 #
##########
##
## Generate a build plan for rust dependencies
##
FROM chef AS planner
COPY . .
# Generate "lockfile" aka dependency dump
RUN cargo chef prepare \
  --recipe-path recipe.json


##########
# Step 2 #
##########
##
## Build app with the cached dependencies
##
FROM chef AS builder
COPY --from=planner /app/recipe.json .
# Build dependencies
ARG RUST_TARGET
RUN cargo chef cook \
  --release \
  --target "${RUST_TARGET}" \
  --recipe-path recipe.json
# Copy rest of files and compile
# only the remaining app code
COPY . .
ARG RUST_TARGET
ARG BINARY_NAME
RUN cargo build \
  --release \
  --target="${RUST_TARGET}" \
  --bin "${BINARY_NAME}"


##########
# Step 3 #
##########
##
## Run the app in a configured environment
##
FROM ubuntu:rolling as runner
RUN apt-get update && apt-get install -y \
  sudo \
  curl \
  fontconfig \
  bzip2 \
  python3 \
  xattr \
  xz-utils \
  && echo "Done installing packages"
# Install latest ffmpeg
ARG FFMPEG_DOWNLOAD_URL
RUN cd "$(mktemp --directory)" && \
  curl -svL "${FFMPEG_DOWNLOAD_URL}" | tar xvJ \
  && cd ffmpeg-*-amd64-static \
  && mv ffmpeg ffprobe qt-faststart /usr/local/bin/ \
  && cd .. \
  && rm -rf "$(pwd)"
# Install latest phantomjs (yt-dlp uses as a helper)
ENV OPENSSL_CONF=/dev/null
RUN mkdir -p /usr/share \
  && cd /usr/share \
  && curl -L 'https://bitbucket.org/ariya/phantomjs/downloads/phantomjs-2.1.1-linux-x86_64.tar.bz2' | tar xj \
  && ln -s '/usr/share/phantomjs-2.1.1-linux-x86_64/bin/phantomjs' /usr/bin/phantomjs \
  && phantomjs --version
# Install yt-dlp
COPY ./.docker/app/update_yt-dlp.sh /usr/bin/update_yt-dlp.sh
RUN chmod a=rx /usr/bin/update_yt-dlp.sh
RUN /usr/bin/update_yt-dlp.sh
# Delete default ubuntu user
RUN userdel --remove ubuntu; groupdel ubuntu; echo "Deleted default ubuntu user"
# Create run user
ARG RUN_USERNAME
ARG RUN_USER_ID
ARG RUN_GROUP_ID
RUN groupadd --gid "${RUN_GROUP_ID}" "${RUN_USERNAME}"
RUN useradd --create-home --uid "${RUN_USER_ID}" --gid "${RUN_GROUP_ID}" "${RUN_USERNAME}"
# Install app
ARG RUST_TARGET
ARG BINARY_NAME
COPY --from=builder "/app/target/${RUST_TARGET}/release/${BINARY_NAME}" /usr/local/bin/
RUN chmod a=rx "/usr/local/bin/${BINARY_NAME}"
# Run app
USER ${RUN_USERNAME}
LABEL org.opencontainers.image.authors="me@allypost.net"
ENTRYPOINT "${BINARY_NAME}"
CMD [ "--help" ]
