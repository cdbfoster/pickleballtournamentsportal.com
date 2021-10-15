ARG APP_NAME=pickleballtournamentsportal

FROM rust:latest as builder
ARG APP_NAME

# Build all the dependencies first, separately
RUN cargo new --bin ${APP_NAME}
WORKDIR /${APP_NAME}
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release
RUN rm -r ./src; rm target/release/deps/${APP_NAME}*

# Build our actual project
COPY src ./src
RUN cargo build --release

FROM debian:11-slim
ARG APP_NAME

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

ENV APP_USER=appuser

RUN useradd $APP_USER && mkdir -p /${APP_NAME}
COPY --from=builder /${APP_NAME}/target/release/${APP_NAME} /${APP_NAME}/${APP_NAME}
COPY static /${APP_NAME}/static
COPY templates /${APP_NAME}/templates
RUN chown -R $APP_USER:$APP_USER /${APP_NAME}

USER $APP_USER
WORKDIR /${APP_NAME}

EXPOSE 8000
CMD ["./pickleballtournamentsportal"]