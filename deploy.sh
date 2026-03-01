#!/usr/bin/env bash
set -euo pipefail

# Production deploy script — builds the image and runs it with docker run on a VPS.
# Use this on your Oracle Cloud VPS (or similar) where you want host networking and
# restart policy handled by Docker.

IMAGE_NAME=solana-copy-bot:latest
CONTAINER_NAME=solana-copy-bot

echo "Building image ${IMAGE_NAME}..."
docker build -t "${IMAGE_NAME}" .

echo "Stopping and removing existing container if present..."
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
  docker rm -f "${CONTAINER_NAME}" || true
fi

echo "Running container on host network..."
docker run -d \
  --name "${CONTAINER_NAME}" \
  --network host \
  --restart unless-stopped \
  --env-file .env \
  --cpus "3.5" \
  --memory "20g" \
  -v "$(pwd)/config.toml:/app/config.toml:ro" \
  -v "$(pwd)/.env:/app/.env:ro" \
  "${IMAGE_NAME}"

echo "Container started: ${CONTAINER_NAME}"

echo "To view logs: docker logs -f ${CONTAINER_NAME}"
