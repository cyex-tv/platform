# The cyex.tv services (PLAN.md Phase 9): one build of the workspace, then one small image per
# service. Build a service with its target:
#   docker build --target api -t cyex-tv-api .
#   docker build --target events -t cyex-tv-events .
#   docker build --target worker -t cyex-tv-worker .
# The images carry no secrets; configuration comes from the environment (see .env.example).

FROM rust:1-bookworm AS build
WORKDIR /src
# A C compiler is already here (libwebp for the worker is built from source).
COPY . .
RUN cargo build --release --locked \
        --bin cyex-tv-api --bin cyex-tv-events --bin cyex-tv-worker \
    && mkdir /out \
    && cp target/release/cyex-tv-api target/release/cyex-tv-events target/release/cyex-tv-worker /out/

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home cyextv
USER cyextv
ENV CYEX_TV_LOG_FORMAT=json

FROM runtime AS api
COPY --from=build /out/cyex-tv-api /usr/local/bin/cyex-tv-api
EXPOSE 8080
HEALTHCHECK --interval=15s --timeout=3s --retries=3 CMD curl -fs http://127.0.0.1:8080/health || exit 1
ENTRYPOINT ["cyex-tv-api"]

FROM runtime AS events
COPY --from=build /out/cyex-tv-events /usr/local/bin/cyex-tv-events
ENV CYEX_TV_BIND=0.0.0.0:8081
EXPOSE 8081
HEALTHCHECK --interval=15s --timeout=3s --retries=3 CMD curl -fs http://127.0.0.1:8081/health || exit 1
ENTRYPOINT ["cyex-tv-events"]

FROM runtime AS worker
COPY --from=build /out/cyex-tv-worker /usr/local/bin/cyex-tv-worker
ENTRYPOINT ["cyex-tv-worker"]
