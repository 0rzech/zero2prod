#!/usr/bin/env bash

set -euo pipefail

if [[ ! -x $(command -v psql) ]]; then
  >&2 echo 'Error: psql not installed.'
  exit 1
fi

if [[ ! -x $(command -v sqlx) ]]; then
  >&2 echo 'Error: sqlx not installed.'
  >&2 echo 'Use:'
  >&2 echo '    cargo install sqlx'
  >&2 echo 'to install it.'
  exit 1
fi

db_host="${POSTGRES_HOST:=localhost}"
db_port="${POSTGRES_PORT:=5432}"
db_user="${POSTGRES_USER:=postgres}"
db_password="${POSTGRES_PASSWORD:=password}"
db_name="${POSTGRES_DB:=newsletter}"

if [[ ${SKIP_PODMAN:=false} == false ]]; then
  podman_cmd=('podman')
  if [[ -f /run/.containerenv ]] && [[ -f /run/.toolboxenv ]]; then
    podman_cmd=('flatpak-spawn' '--host' 'podman')
  fi

  "${podman_cmd[@]}" run \
    --rm \
    --detach \
    --name postgres \
    --publish "${db_port}:5432" \
    --env POSTGRES_USER="${db_user}" \
    --env POSTGRES_PASSWORD="${db_password}" \
    --env POSTGRES_DB="${db_name}" \
    postgres \
    postgres -N 1000

  sleep 5
fi

export PGPASSWORD="${db_password}"
until psql -h "${db_host}" -p "${db_port}" -U "${db_user}" -d 'postgres' -c '\q'; do
  >&2 echo 'Postgres is still unavailable - sleeping...'
  sleep 1
done

>&2 echo "Postgres is up and running on port ${db_port}!"

export DATABASE_URL="postgres://${db_user}:${db_password}@${db_host}:${db_port}/${db_name}"
sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"
