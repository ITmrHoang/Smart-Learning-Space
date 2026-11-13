#!/usr/bin/env sh
# Khởi tạo Garage MỘT LẦN sau khi container garage đã chạy: gán dung lượng cho node, tạo bucket `files`,
# nhập khóa truy cập cho api. Chạy lại nhiều lần không sao (bước đã làm thì bỏ qua). Windows: dùng Git Bash.
#
#   sh deploy/garage/init.sh          # dev  (deploy/docker-compose.dev.yml, khóa dev trong backend/.env.example)
#   sh deploy/garage/init.sh prod     # production (deploy/docker-compose.yml, khóa + dung lượng đọc từ deploy/.env)
set -eu
# Git Bash trên Windows tự đổi "/garage" thành đường dẫn Windows → tắt việc đổi đường dẫn.
export MSYS_NO_PATHCONV=1
cd "$(dirname "$0")/.."

if [ "${1:-dev}" = "prod" ]; then
  COMPOSE_FILE=docker-compose.yml
  env_value() { sed -n "s/^$1=//p" .env | tail -n 1 | tr -d '"\r'; }
  KEY_ID=$(env_value S3_ACCESS_KEY_ID)
  SECRET=$(env_value S3_SECRET_ACCESS_KEY)
  CAPACITY=$(env_value GARAGE_CAPACITY)
  CAPACITY=${CAPACITY:-100G}
else
  COMPOSE_FILE=docker-compose.dev.yml
  KEY_ID=GK736c732d6465762d6b657930
  SECRET=736c732d6465762d7365637265742d6b65792d30313233343536373839616263
  CAPACITY=2G
fi
[ -n "$KEY_ID" ] && [ -n "$SECRET" ] || { echo "Thiếu S3_ACCESS_KEY_ID / S3_SECRET_ACCESS_KEY trong deploy/.env" >&2; exit 1; }
garage() { docker compose -f "$COMPOSE_FILE" exec -T garage /garage "$@"; }

NODE=$(garage node id -q | cut -d@ -f1)
[ -n "$NODE" ] || { echo "Không đọc được node id của Garage — container đã chạy chưa?" >&2; exit 1; }
if garage layout show | grep -q "$(echo "$NODE" | cut -c1-16)"; then
  echo "Layout đã có node $NODE"
else
  garage layout assign -z dc1 -c "$CAPACITY" "$NODE"
  garage layout apply --version 1
fi
garage bucket info files >/dev/null 2>&1 || garage bucket create files
garage key info "$KEY_ID" >/dev/null 2>&1 || garage key import --yes -n sls-api "$KEY_ID" "$SECRET"
garage bucket allow --read --write --owner files --key "$KEY_ID" >/dev/null
echo "Garage sẵn sàng: bucket files, khóa $KEY_ID"
