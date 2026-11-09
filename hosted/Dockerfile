# The hosted instance's image (ADR 0010): the same server/ release bundle a self-hoster runs (ADR 0006, "Deployed form") and the
# built web client, nothing else. Built from the repo root (wrangler.jsonc: image_build_context ".."). Cloudflare Containers run
# linux/amd64. The tag and its index digest are pinned: oven/bun 1.4.4-slim, the repo's .tool-versions bun.
FROM --platform=linux/amd64 oven/bun:1.4.4-slim@sha256:1846adf22496bd711036aae8be147e57bdebbaea2e5264e3e0b5af1db187d9a4 AS build
WORKDIR /src
COPY package.json bun.lock ./
# bun install applies package.json's patchedDependencies, so the patch it names is copied first.
COPY worker/patches ./worker/patches
RUN bun install --frozen-lockfile
COPY tsconfig.json ./
COPY gen ./gen
COPY podcore ./podcore
COPY server ./server
COPY web ./web
# The Makefile's e2e-deployed build, exactly: pglite stays external, and absent from the image (it is the local fallback only).
RUN bun build server/src/main.ts --target=bun --external @electric-sql/pglite --outfile /out/server.js
# The image has no Node, so vite runs on Bun.
RUN bun --bun run build:web && cp -R web/dist /out/web

FROM --platform=linux/amd64 oven/bun:1.4.4-slim@sha256:1846adf22496bd711036aae8be147e57bdebbaea2e5264e3e0b5af1db187d9a4
WORKDIR /app
COPY --from=build /out/server.js /app/server.js
COPY --from=build /out/web /app/web
# The hosted shape: ENV unset, no TEST_BEARER. DATABASE_URL and the other secrets come from the front Worker (hosted/src/index.ts).
ENV JM_WEB_DIR=/app/web PORT=8080 TRUST_PROXY=1
EXPOSE 8080
USER bun
CMD ["bun", "/app/server.js"]
