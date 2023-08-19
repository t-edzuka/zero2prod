#!/usr/bin/env bash
set -x
set -eo pipefail
# if a redis container is running, print instructions to kill it and exit
RUNNING_CONTAINER=$(docker ps --filter 'name=redis' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
#echo >&2 "there is a redis container already running, kill it with" echo >&2 " docker kill ${RUNNING_CONTAINER}"
docker stop "$RUNNING_CONTAINER" && docker rm "$RUNNING_CONTAINER"
fi
# Launch Redis using Docker
docker run \
    -p "6379:6379" \
    -d \
    --name "redis_$(date '+%s')" \
    redis:latest
>&2 echo "Redis is ready to go!"