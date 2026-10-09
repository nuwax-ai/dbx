# syntax=docker/dockerfile:1

# Stage 1: Build frontend (native, no emulation)
FROM --platform=$BUILDPLATFORM node:22-slim AS frontend
WORKDIR /app
RUN npm i -g pnpm
# Keep the workspace package available because Vite resolves it directly from source.
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY packages/mongo-shell/ packages/mongo-shell/
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
COPY apps/desktop/ apps/desktop/
# The vite connection-types plugin runs scripts/sync-connection-types.mjs during
# `pnpm build`, which reads plugins/connection-types + plugins/dialects and
# regenerates the manifest and generated TS types in place.
COPY scripts/ scripts/
COPY plugins/ plugins/
COPY crates/dbx-core/assets/database-drivers.manifest.json crates/dbx-core/assets/database-drivers.manifest.json
RUN pnpm build

# Stage 2: Cross-compile Rust backend for the requested platform (native, no emulation)
FROM --platform=$BUILDPLATFORM rust:1-bookworm AS backend
ARG TARGETARCH
WORKDIR /app
ARG PIP_INDEX_URL=https://pypi.org/simple
# nuwax fork: 使用 LinuxMirrors 一键配置阿里云 apt 镜像源（国内构建加速）
# 参考: https://github.com/SuperManito/LinuxMirrors
# rust:1-bookworm (基于 buildpack-deps) 已预装 curl，可直接执行脚本配置镜像源
RUN curl -sSL https://linuxmirrors.cn/main.sh | bash -s -- \
    --source mirrors.aliyun.com \
    --protocol https \
    --use-intranet-source false \
    --install-epel false \
    --backup false \
    --upgrade-software false \
    --clean-cache true \
    --ignore-backup-tips
RUN dpkg --add-architecture arm64 \
    && apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake pkg-config perl python3-pip gcc-aarch64-linux-gnu \
    libfontconfig-dev:arm64 libfreetype-dev:arm64 \
    && rm -rf /var/lib/apt/lists/*
# ziglang wheel ~95MB：国内镜像偶发连接整段死掉（>120s 零数据），pip 单次
# 失败不重试不续传——外层循环换新 TCP 连接重试；独立层隔离，pip 失败
# 重试不连累 apt / cargo install 重跑
RUN attempts=5; \
    for i in $(seq 1 $attempts); do \
      pip3 install --break-system-packages --timeout 120 -i ${PIP_INDEX_URL} ziglang && break; \
      if [ "$i" -eq "$attempts" ]; then echo "ziglang install failed after $attempts attempts" >&2; exit 1; fi; \
      echo "pip attempt $i failed, retrying with a fresh connection..."; sleep 3; \
    done
RUN cargo install cargo-zigbuild \
    && rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

# nuwax fork: 补装 amd64 交叉的 fontconfig/freetype dev 库（独立层，不动上游层——
# arm64 构建缓存保留）。上游只装 :arm64 覆盖 "amd64 runner 交叉 arm64"；在 arm64
# 宿主（OrbStack mac）交叉 amd64 时缺 amd64 库，链接报
# "libfontconfig.so is incompatible with elf_x86_64"。
RUN dpkg --add-architecture amd64 && apt-get update \
    && apt-get install -y --no-install-recommends \
        libfontconfig-dev:amd64 libfreetype-dev:amd64 \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests only and create dummy sources to pre-compile dependencies.
# This layer is cached by GHA as long as Cargo.toml/Cargo.lock don't change.
COPY Cargo.toml Cargo.lock ./
COPY crates/dbx-core/Cargo.toml crates/dbx-core/
COPY crates/dbx-platform/Cargo.toml crates/dbx-platform/
COPY crates/dbx-types/Cargo.toml crates/dbx-types/
COPY crates/dbx-sql/Cargo.toml crates/dbx-sql/
COPY crates/dbx-sql-core/Cargo.toml crates/dbx-sql-core/
COPY crates/dbx-sql-data/Cargo.toml crates/dbx-sql-data/
COPY crates/dbx-sql-dialect/Cargo.toml crates/dbx-sql-dialect/
COPY crates/dbx-sql-schema/Cargo.toml crates/dbx-sql-schema/
COPY crates/dbx-formats/Cargo.toml crates/dbx-formats/
COPY crates/dbx-ai-provider/Cargo.toml crates/dbx-ai-provider/
COPY crates/dbx-plugin-runtime/Cargo.toml crates/dbx-plugin-runtime/
COPY crates/dbx-driver-agent/Cargo.toml crates/dbx-driver-agent/
COPY crates/dbx-driver-support/Cargo.toml crates/dbx-driver-support/
COPY crates/dbx-driver-elasticsearch/Cargo.toml crates/dbx-driver-elasticsearch/
COPY crates/dbx-driver-mongodb/Cargo.toml crates/dbx-driver-mongodb/
COPY crates/dbx-driver-mysql/Cargo.toml crates/dbx-driver-mysql/
COPY crates/dbx-driver-postgres/Cargo.toml crates/dbx-driver-postgres/
COPY crates/dbx-driver-redis/Cargo.toml crates/dbx-driver-redis/
COPY crates/dbx-driver-sqlserver/Cargo.toml crates/dbx-driver-sqlserver/
COPY crates/dbx-drivers/Cargo.toml crates/dbx-drivers/
COPY crates/dbx-sqlite-worker/Cargo.toml crates/dbx-sqlite-worker/
COPY crates/dbx-web/Cargo.toml crates/dbx-web/
COPY crates/dbx-mcp/Cargo.toml crates/dbx-mcp/
COPY crates/dbx-cli/Cargo.toml crates/dbx-cli/
COPY crates/dbx-tauri-consul/Cargo.toml crates/dbx-tauri-consul/
COPY crates/dbx-tauri-schema/Cargo.toml crates/dbx-tauri-schema/
COPY src-tauri/Cargo.toml src-tauri/
# Copy every vendored crate: root Cargo.toml [patch.crates-io] points at
# vendor/{ctor,rumqttc,dirs-sys,pageant,wry}; omitting any breaks the offline build.
COPY vendor/ vendor/
RUN for crate in dbx-platform dbx-types dbx-sql dbx-sql-core dbx-sql-data dbx-sql-dialect dbx-sql-schema \
      dbx-formats dbx-ai-provider dbx-plugin-runtime dbx-driver-agent dbx-driver-support \
      dbx-driver-elasticsearch dbx-driver-mongodb dbx-driver-mysql dbx-driver-postgres \
      dbx-driver-redis dbx-driver-sqlserver dbx-drivers dbx-sqlite-worker dbx-tauri-consul dbx-tauri-schema; do \
      mkdir -p "crates/$crate/src" && printf "" > "crates/$crate/src/lib.rs"; \
    done \
    && echo 'fn main() {}' > crates/dbx-sqlite-worker/src/main.rs
RUN mkdir -p crates/dbx-core/src && echo '' > crates/dbx-core/src/lib.rs \
    && mkdir -p crates/dbx-web/src && echo 'fn main() {}' > crates/dbx-web/src/main.rs \
    && mkdir -p crates/dbx-mcp/src && echo '' > crates/dbx-mcp/src/lib.rs && echo 'fn main() {}' > crates/dbx-mcp/src/main.rs \
    && mkdir -p crates/dbx-cli/src && echo 'fn main() {}' > crates/dbx-cli/src/main.rs \
    && mkdir -p crates/dbx-sqlite-worker/src && echo 'fn main() {}' > crates/dbx-sqlite-worker/src/main.rs \
    && mkdir -p src-tauri/src && echo 'fn main() {}' > src-tauri/src/main.rs && echo 'pub fn run() {}' > src-tauri/src/lib.rs

COPY src-tauri/build.rs src-tauri/
COPY src-tauri/tauri.conf.json src-tauri/

RUN case "$TARGETARCH" in \
      amd64) rust_target=x86_64-unknown-linux-gnu; lib_arch=x86_64-linux-gnu ;; \
      arm64) rust_target=aarch64-unknown-linux-gnu; lib_arch=aarch64-linux-gnu ;; \
      *) echo "Unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac && \
    LIBRARY_PATH="/usr/lib/$lib_arch" \
    PKG_CONFIG_PATH="/usr/lib/$lib_arch/pkgconfig" \
    PKG_CONFIG_SYSROOT_DIR="/" \
    RUSTFLAGS="-L native=/usr/lib/$lib_arch" \
    cargo zigbuild --release -p dbx-web --target "$rust_target" || true

# Copy real sources and rebuild (only application code recompiles)
COPY crates/ crates/
# dbx-sql-dialect/build.rs embeds plugins/dialects; dbx-types/build.rs embeds
# plugins/connection-types (database manifest + DatabaseType enum);
# either directory missing makes the build script panic.
COPY plugins/dialects/ plugins/dialects/
COPY plugins/connection-types/ plugins/connection-types/

# Docker COPY preserves original file timestamps, which may be older than the
# dummy-source binary compiled above. Touch all .rs files so cargo detects the
# source change and recompiles instead of reusing the dummy binary.
RUN find crates/ -name '*.rs' -exec touch {} +

RUN case "$TARGETARCH" in \
      amd64) rust_target=x86_64-unknown-linux-gnu; lib_arch=x86_64-linux-gnu ;; \
      arm64) rust_target=aarch64-unknown-linux-gnu; lib_arch=aarch64-linux-gnu ;; \
      *) echo "Unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac && \
    LIBRARY_PATH="/usr/lib/$lib_arch" \
    PKG_CONFIG_PATH="/usr/lib/$lib_arch/pkgconfig" \
    PKG_CONFIG_SYSROOT_DIR="/" \
    RUSTFLAGS="-L native=/usr/lib/$lib_arch" \
    cargo zigbuild --release -p dbx-web --target "$rust_target" && \
    mkdir -p "/out/linux/$TARGETARCH" && \
    cp "/app/target/$rust_target/release/dbx-web" "/out/linux/$TARGETARCH/"

# Stage 3: Final image
FROM debian:bookworm-slim
ARG TARGETPLATFORM
# nuwax fork: runtime 阶段同样换阿里云 apt 镜像（本镜像仅作 dbx-web 二进制/静态资源
# 的抽取载体,不关心体积——先装 curl 走一次官方源,再统一用 LinuxMirrors 脚本,
# 后续 ~200MB 的 openjdk 等全走阿里云;curl 层有缓存,仅首次构建付官方源过路费）
RUN apt-get update && apt-get install -y --no-install-recommends curl \
    && curl -sSL https://linuxmirrors.cn/main.sh | bash -s -- \
    --source mirrors.aliyun.com \
    --protocol https \
    --use-intranet-source false \
    --install-epel false \
    --backup false \
    --upgrade-software false \
    --clean-cache true \
    --ignore-backup-tips \
    && rm -rf /var/lib/apt/lists/*
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    fontconfig \
    fonts-dejavu-core \
    libfreetype6 \
    libssl3 \
    openjdk-17-jre-headless \
    && rm -rf /var/lib/apt/lists/*
COPY --from=backend /out/${TARGETPLATFORM}/dbx-web /usr/local/bin/
COPY --from=frontend /app/dist /app/static
ENV DBX_STATIC_DIR=/app/static
ENV DBX_DATA_DIR=/app/data
ENV DBX_BACKUP_ROOT=/app/backups
ENV DBX_JAVA_BIN=/usr/bin/java
EXPOSE 4224
CMD ["dbx-web"]
