FROM rust:alpine as build
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories
RUN apk add musl-dev zlib-dev openssl-dev \
    && rm -rf /var/cache/apk/*
WORKDIR /app
RUN cargo init . --name bootstrap
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY .cargo ./.cargo
RUN cargo build --release
COPY src /app/src
RUN touch src/main.rs && cargo build --release

FROM alpine
VOLUME /root/.ssh
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories
RUN apk add libgcc \
    && rm -rf /var/cache/apk/*
WORKDIR /app
COPY --from=build /app/target/release/repo-mirror /app/repo-mirror
COPY repos.json /app
CMD /app/repo-mirror
