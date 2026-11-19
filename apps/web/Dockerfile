# The cyex.tv website (PLAN.md Phase 9): SvelteKit's Node build, run as a non-root user.
#   docker build -f apps/web/Dockerfile -t cyex-tv-web .   (from the repository root)
# Configuration at run time: PUBLIC_API_URL (default https://api.cyex.tv), ORIGIN (the site's own
# URL, https://cyex.tv in production), PORT (3000). No secrets; the site holds none.
# The Content-Security-Policy is fixed at build time and allows the API origin PUBLIC_API_URL has
# then: for an API other than https://api.cyex.tv, also pass --build-arg PUBLIC_API_URL=<origin>,
# and for pictures served from somewhere other than https://cdn.cyex.tv, --build-arg PUBLIC_CDN_URL=<url>.

FROM node:22-bookworm-slim AS build
WORKDIR /repo/apps/web
COPY apps/web/package.json apps/web/package-lock.json ./
RUN npm ci --no-audit --no-fund
# The API docs page is built from these two files (they sit two levels above the app).
COPY openapi /repo/openapi
COPY schema /repo/schema
COPY apps/web ./
ARG PUBLIC_API_URL=
ARG PUBLIC_CDN_URL=
RUN npm run build

FROM node:22-bookworm-slim AS web
WORKDIR /app
ENV NODE_ENV=production PORT=3000 HOST=0.0.0.0
# adapter-node bundles the app's code; package.json says it's an ES module.
COPY --from=build /repo/apps/web/build ./build
COPY --from=build /repo/apps/web/package.json ./package.json
USER node
EXPOSE 3000
# A page that needs no API call, so the check doesn't load the API every 15 s.
HEALTHCHECK --interval=15s --timeout=3s --retries=3 \
    CMD node -e "fetch('http://127.0.0.1:3000/developers').then(r => process.exit(r.ok ? 0 : 1), () => process.exit(1))"
CMD ["node", "build"]
