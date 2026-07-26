FROM rust:1.95-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/xiaozhongsi /usr/local/bin/xiaozhongsi
# 容器里没有系统 sshd，所以内部端口随便挑，由 Fly 边缘把外部 22 映射进来
ENV SSH_PORT=2222
EXPOSE 2222
CMD ["xiaozhongsi"]
