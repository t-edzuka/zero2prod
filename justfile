export DATABASE_URL := "postgres://postgres:password@127.0.0.1:5432/newsletter"
export RUST_LOG := "debug"
export RUSTC_WRAPPER := `which sccache`

alias t := test
alias c := check
alias f := format
alias l := lint
alias d := dev
alias ma := migrate-add
alias b := build
alias pc := pre-commit

run:
    cargo run | jq .

check:
    cargo check

dev:
    export TEST_LOG=true && cargo watch -x check -x test -x run | bunyan

build:
    cargo build

format:
    cargo fmt
    cargo clippy
    cargo check

lint:
    cargo fmt --version
    cargo fmt --all -- --check
    cargo clippy --version
    cargo clippy -- -D warnings

# cargo install bynyan

# "bunyan" prettifies the outputted logs
test:
    export TEST_LOG=true && export RUST_LOG="sqlx=error,info" && cargo test -q | bunyan

build-test:
    cargo build --tests

init-db:
    bash scripts/init_db.sh && bash scripts/init_redis.sh

psql:
    docker exec -it psql-dev psql {{ DATABASE_URL }}

list-db:
    docker exec -it psql-dev psql {{ DATABASE_URL }} -c "\l"

show-tables:
    docker exec -it psql-dev psql {{ DATABASE_URL }} -c "\dt"

clear-db:
    docker stop psql-dev
    docker rm psql-dev

reinit-db: clear-db init-db

# Create a new migration script
migrate-add script_name: reinit-db
    export DATABASE_URL={{ DATABASE_URL }}
    sqlx migrate add {{ script_name }}

prepare-db:
    cargo sqlx prepare --database-url {{ DATABASE_URL }} -- --all-targets --all-features

init-redis:
    bash ./scripts/init_redis.sh

deps:
    cargo +nightly udeps --all-targets

fix:
    cargo fix --allow-dirty && cargo clippy --fix --allow-dirty

pre-commit: tp prepare-db fix format test

# reorder Cargo.toml
tp:
    taplo fmt --option reorder_keys=true Cargo.toml

audit:
    cargo deny check advisories

# For a digital ocean new deployment.
#dauth:
#    doctl auth init
#new_deploy:
#    doctl apps create --spec=spec.yaml
#update_deploy app_id:
#    doctl apps update {{app_id}} --spec=spec.yaml

# Stop and remove current working postgres and redis container. AI?
stop-conatiners:
    containers=$(docker ps -a -q) && docker stop $containers && docker rm $containers

remove-images:
    docker rmi $(docker images -q)

prune-volumes:
    yes | docker volume prune

clean-docker: stop-conatiners remove-images prune-volumes
