# Multi-stage build for NEXUS Web Frontend
# Stage 1: Build
FROM node:20-alpine AS builder

WORKDIR /build

COPY package.json package-lock.json /build/

RUN npm ci

COPY . /build/

RUN npm run build

# Stage 2: Production
FROM nginx:alpine

COPY --from=builder /build/dist /usr/share/nginx/html
COPY ./nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost/index.html || exit 1

CMD ["nginx", "-g", "daemon off;"]
