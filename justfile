export DATABASE_URL := "postgres://postgres:password@127.0.0.1:5432/newsletter"
export RUST_LOG := "debug"
alias t := test
alias f := format
alias l := lint
alias d := dev
alias ct := create_table
alias b := build

run:
    cargo run

dev:
    cargo watch -x check -x test -x run

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
    TEST_LOG=true cargo test | bunyan

build_test:
    cargo build --tests

init_db:
    bash scripts/init_db.sh

psql:
    docker exec -it psql-dev psql {{DATABASE_URL}}

list-db:
    docker exec -it psql-dev psql {{DATABASE_URL}} -c "\l"
clear-db:
    docker stop psql-dev
    docker rm psql-dev

reinit-db:clear-db init_db



create_table script_name: init_db
    export DATABASE_URL={{DATABASE_URL}}
    sqlx migrate add {{ script_name }}

prepare_db:
    cargo sqlx prepare --database-url {{DATABASE_URL}} -- --all-targets --all-features

deps:
    cargo +nightly udeps


fix:
    cargo fix --allow-dirty && cargo clippy --fix --allow-dirty

pre-commit:prepare_db fix format test

