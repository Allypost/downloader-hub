FROM rust:1.76 AS builder
RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN rustup toolchain install nightly
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/src/app
COPY . .
ENV RUSTFLAGS='-C target-feature=+crt-static'
RUN cargo build \
  --release \
  --target='x86_64-unknown-linux-musl' \
  --bin 'downloader-hub' \
  && echo "Done compiling app"

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
ARG FFMPEG_DOWNLOAD_URL=https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz
RUN cd "$(mktemp --directory)" && \
  curl -svL "${FFMPEG_DOWNLOAD_URL}" | tar xvJ \
  && cd ffmpeg-*-amd64-static \
  && mv ffmpeg ffprobe qt-faststart /usr/local/bin/ \
  && cd .. \
  && rm -rf "$(pwd)"
ENV OPENSSL_CONF=/dev/null
RUN mkdir -p /usr/share \
  && cd /usr/share \
  && curl -L https://bitbucket.org/ariya/phantomjs/downloads/phantomjs-2.1.1-linux-x86_64.tar.bz2 | tar xj \
  && ln -s /usr/share/phantomjs-2.1.1-linux-x86_64/bin/phantomjs /usr/bin/phantomjs \
  && phantomjs --version
ARG username=app
ARG user_id=1000
ARG group_id=1000
RUN userdel --remove ubuntu; groupdel ubuntu; echo "Deleted default ubuntu user"
RUN groupadd --gid ${group_id} ${username}
RUN useradd --create-home --uid ${user_id} --gid ${group_id} ${username}
COPY ./.docker/app/update_yt-dlp.sh /usr/bin/update_yt-dlp.sh
RUN chmod a=rx,u+s /usr/bin/update_yt-dlp.sh
RUN /usr/bin/update_yt-dlp.sh
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/downloader-hub /usr/local/bin/
USER ${username}
LABEL org.opencontainers.image.authors="me@allypost.net"
ENTRYPOINT ["downloader-hub"]
CMD [ "--help" ]
