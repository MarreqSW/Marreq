# Build SPA (Vite); legacy JS/CSS from backend/src/html/static (vite alias @static).
FROM node:20-bookworm AS build
WORKDIR /build/frontend
COPY frontend/package.json ./
RUN npm install
COPY frontend/ ./
COPY backend/src/html/static /build/backend/src/html/static
RUN npm run build

FROM nginx:alpine
COPY --from=build /build/frontend/dist /usr/share/nginx/html
COPY docker/frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
