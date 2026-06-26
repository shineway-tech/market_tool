#!/usr/bin/env bash
set -euo pipefail

APP_NAME="channel_nest_api"
DEPLOY_ENV="${1:-test}"
SERVER_HOST="${SERVER_HOST:-47.98.61.64}"
SSH_USER="${SSH_USER:-root}"
REMOTE_BASE="${REMOTE_BASE:-/data/apps}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_FILE="config/config.${DEPLOY_ENV}.json"
TIMESTAMP="$(date +%Y%m%d%H%M%S)"
SERVICE_NAME="${APP_NAME}-${DEPLOY_ENV}"
REMOTE_DIR="${REMOTE_BASE}/${SERVICE_NAME}"
ARCHIVE_NAME="${SERVICE_NAME}-${TIMESTAMP}.tar.gz"
ARCHIVE_PATH="/tmp/${ARCHIVE_NAME}"

if [[ "${DEPLOY_ENV}" != "test" && "${DEPLOY_ENV}" != "prod" ]]; then
  echo "Usage: $0 [test|prod]" >&2
  exit 1
fi

if [[ ! -f "${ROOT_DIR}/${CONFIG_FILE}" ]]; then
  echo "Missing config file: ${CONFIG_FILE}" >&2
  exit 1
fi

PORT="$(node -e "console.log(require('${ROOT_DIR}/${CONFIG_FILE}').port)")"
SSH_TARGET="${SSH_USER}@${SERVER_HOST}"

echo "Deploying ${SERVICE_NAME} to ${SERVER_HOST}:${PORT}"

LC_ALL=C COPYFILE_DISABLE=1 tar --format ustar -czf "${ARCHIVE_PATH}" \
  --exclude="./node_modules" \
  --exclude="./logs" \
  --exclude="./*.log" \
  --exclude="./.DS_Store" \
  --exclude="./._*" \
  --exclude="*/.DS_Store" \
  --exclude="*/._*" \
  -C "${ROOT_DIR}" .

ssh "${SSH_TARGET}" "mkdir -p '${REMOTE_DIR}/releases' '${REMOTE_DIR}/packages'"
scp "${ARCHIVE_PATH}" "${SSH_TARGET}:${REMOTE_DIR}/packages/${ARCHIVE_NAME}"

ssh "${SSH_TARGET}" "bash -s" <<EOF
set -euo pipefail

APP_NAME="${APP_NAME}"
DEPLOY_ENV="${DEPLOY_ENV}"
SERVICE_NAME="${SERVICE_NAME}"
REMOTE_DIR="${REMOTE_DIR}"
RELEASE_DIR="\${REMOTE_DIR}/releases/${TIMESTAMP}"
ARCHIVE_PATH="\${REMOTE_DIR}/packages/${ARCHIVE_NAME}"
CONFIG_FILE="${CONFIG_FILE}"
PORT="${PORT}"

mkdir -p "\${RELEASE_DIR}"
tar -xzf "\${ARCHIVE_PATH}" -C "\${RELEASE_DIR}"
mkdir -p "\${REMOTE_DIR}/shared/storage"
rm -rf "\${RELEASE_DIR}/storage"
ln -sfn "\${REMOTE_DIR}/shared/storage" "\${RELEASE_DIR}/storage"
cp "\${RELEASE_DIR}/\${CONFIG_FILE}" "\${RELEASE_DIR}/config/config.json"

cd "\${RELEASE_DIR}"
if [[ -f package-lock.json ]]; then
  npm ci --omit=dev
else
  npm install --omit=dev
fi
npm run migrate

ln -sfn "\${RELEASE_DIR}" "\${REMOTE_DIR}/current"

if command -v pm2 >/dev/null 2>&1; then
  pm2 delete "\${SERVICE_NAME}" >/dev/null 2>&1 || true
  pm2 start "\${REMOTE_DIR}/current/bin/www" --name "\${SERVICE_NAME}" --cwd "\${REMOTE_DIR}/current" --time
  pm2 save >/dev/null 2>&1 || true
else
  if [[ -f "\${REMOTE_DIR}/service.pid" ]]; then
    kill "\$(cat "\${REMOTE_DIR}/service.pid")" >/dev/null 2>&1 || true
  fi
  if command -v lsof >/dev/null 2>&1; then
    for pid in \$(lsof -tiTCP:"\${PORT}" -sTCP:LISTEN 2>/dev/null || true); do
      kill "\${pid}" >/dev/null 2>&1 || true
    done
  fi
  cd "\${REMOTE_DIR}/current"
  nohup node bin/www >> "\${REMOTE_DIR}/service.log" 2>&1 &
  echo \$! > "\${REMOTE_DIR}/service.pid"
fi

echo "Deployed \${SERVICE_NAME} on port \${PORT}"
EOF

rm -f "${ARCHIVE_PATH}"
echo "Done."
