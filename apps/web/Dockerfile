# The cyex.tv website (PLAN.md Phase 9): SvelteKit's Node build, run as a non-root user.
#   docker build -t cyex-tv-web apps/web
# Configuration at run time: PUBLIC_API_URL (default https://api.cyex.tv), ORIGIN (the site's own
# URL, https://cyex.tv in production), PORT (3000). No secrets; the site holds none.

FROM node:22-bookworm-slim AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY . .
RUN npm run build

FROM node:22-bookworm-slim AS web
WORKDIR /app
ENV NODE_ENV=production PORT=3000 HOST=0.0.0.0
# adapter-node bundles the app's code; package.json says it's an ES module.
COPY --from=build /app/build ./build
COPY --from=build /app/package.json ./package.json
USER node
EXPOSE 3000
# A page that needs no API call, so the check doesn't load the API every 15 s.
HEALTHCHECK --interval=15s --timeout=3s --retries=3 \
    CMD node -e "fetch('http://127.0.0.1:3000/developers').then(r => process.exit(r.ok ? 0 : 1), () => process.exit(1))"
CMD ["node", "build"]
