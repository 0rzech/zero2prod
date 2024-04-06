#!/usr/bin/env bash

set -eou pipefail

podman_cmd=('podman')
if [[ -f /run/.containerenv ]] && [[ -f /run/.toolboxenv ]]; then
  podman_cmd=('flatpak-spawn' '--host' 'podman')
fi

RUNNING_CONTAINER=$("${podman_cmd[@]}" ps --filter 'name=redis' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
  >&2 echo 'there is a redis container already running, kill it with'
  >&2 echo "    ${podman_cmd[@]} kill ${RUNNING_CONTAINER}"
  exit 1
fi

"${podman_cmd[@]}" run \
    --rm \
    --detach \
    --name "redis_$(date '+%s')" \
    --publish 6379:6379 \
    docker.io/library/redis:latest

>&2 echo 'Readis is ready to go!'
