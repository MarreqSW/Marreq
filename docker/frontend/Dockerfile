# Build SPA (Vite); JS/CSS live under frontend/static (Vite alias @static).
FROM node:20-bookworm AS build
ARG MARREQ_VERSION=0.1.0
ARG MARREQ_GIT_SHA=unknown
ENV MARREQ_GIT_SHA=${MARREQ_GIT_SHA}
WORKDIR /build/frontend
COPY frontend/package.json ./
RUN npm install
COPY frontend/ ./
RUN npm run build

FROM nginx:alpine
ARG MARREQ_VERSION=0.1.0
ARG MARREQ_GIT_SHA=unknown
LABEL org.opencontainers.image.version="${MARREQ_VERSION}" \
      org.opencontainers.image.revision="${MARREQ_GIT_SHA}"
# Only substitute MARREQ_* in templates — avoids envsubst touching unrelated nginx `$vars`.
ENV NGINX_ENVSUBST_FILTER=^MARREQ_
ENV MARREQ_BACKEND_HOST=marreq-server
ENV MARREQ_BACKEND_PORT=8000
ENV MARREQ_MCP_HOST=mcp-server
ENV MARREQ_MCP_PORT=3000
COPY --from=build /build/frontend/dist /usr/share/nginx/html
COPY docker/frontend/nginx.conf.template /etc/nginx/templates/default.conf.template
EXPOSE 80
