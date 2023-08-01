#!/usr/bin/env bash
set -x
set -eo pipefail

DB_HOST="${POSTGRES_HOST:=localhost}"
DB_PORT="${POSTGRES_PORT:=5432}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"

DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
export DATABASE_URL
sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"