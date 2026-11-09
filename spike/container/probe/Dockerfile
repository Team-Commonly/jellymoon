# F32 spike probe image. Cloudflare Containers run linux/amd64; the tag and its index digest are pinned (oven/bun 1.4.4-slim,
# the repo's .tool-versions bun).
FROM --platform=linux/amd64 oven/bun:1.4.4-slim@sha256:1846adf22496bd711036aae8be147e57bdebbaea2e5264e3e0b5af1db187d9a4
WORKDIR /app
COPY server.ts ./
ENV PORT=8080
EXPOSE 8080
USER bun
CMD ["bun", "server.ts"]
