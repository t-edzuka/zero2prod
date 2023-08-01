#!/usr/bin/env bash
set -x
set -eo pipefail

CONTAINER_NAME="psql-dev"
IS_CONTAINER_RUNNING=$(docker ps -a --format "{{.Names}}"| grep ${CONTAINER_NAME}) || true # if grep fails, it will return 1, but we don't want to exit the script
echo "IS_CONTAINER_RUNNING: ${IS_CONTAINER_RUNNING}"

# In local development, we use psql-dev as the container name,
if [[ -z "${IS_CONTAINER_RUNNING}" ]]; then
  docker run \
      --name  ${CONTAINER_NAME} \
      -e POSTGRES_USER=postgres \
      -e POSTGRES_PASSWORD=password \
      -e POSTGRES_DB=newsletter \
      -p 5432:5432 \
      -d postgres \
      postgres -N 1000
  echo "Container ${CONTAINER_NAME} has been created."
fi

docker start ${CONTAINER_NAME}

DB_HOST="${POSTGRES_HOST:=localhost}"
DB_PORT="${POSTGRES_PORT:=5432}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"

export PGPASSWORD="${DB_PASSWORD}"
until docker exec -it ${CONTAINER_NAME} psql -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
  >&2 echo "Postgres is unavailable - sleeping"
  sleep 2
done

>&2 echo "Postgres is up and running on port ${DB_PORT}."
DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
export DATABASE_URL
sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"