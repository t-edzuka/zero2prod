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
    docker exec -it psql-dev psql {{DATABASE_URL}}

list-db:
    docker exec -it psql-dev psql {{DATABASE_URL}} -c "\l"

show-tables:
    docker exec -it psql-dev psql {{DATABASE_URL}} -c "\dt"

clear-db:
    docker stop psql-dev
    docker rm psql-dev

reinit-db:clear-db init-db


# Create a new migration script
migrate-add script_name: reinit-db
    export DATABASE_URL={{DATABASE_URL}}
    sqlx migrate add {{ script_name }}

prepare-db:
    cargo sqlx prepare --database-url {{DATABASE_URL}} -- --all-targets --all-features

init-redis:
    bash ./scripts/init_redis.sh

deps:
    cargo +nightly udeps --all-targets


fix:
    cargo fix --allow-staged && cargo clippy --fix --allow-staged

pre-commit:tp prepare-db fix format test


# Test in chapter 8
# sqlx logs are a bit noisy, so we cut them out to make the output more readable
#    export TEST_LOG=true && \
t8:
    export TEST_LOG=enabled && \
    export RUST_LOG="sqlx=error,info" && \
    cargo test subscribe_fails_if_there_is_a_fatal_database_error

t9:
    export TEST_LOG=1 && export RUST_LOG="sqlx=error,info" && cargo t newsletters_are_delivered_to_confirmed_subscribers | bunyan

t11:
    export TEST_LOG=1 && export RUST_LOG="sqlx=error,info" && \
    cargo t --test api newsletters::concurrent_form_submission_is_handled_gracefully | bunyan

t11-2:
    export TEST_LOG=1 && export RUST_LOG="sqlx=error,info" && \
    cargo t --test api newsletters::transient_errors_do_not_cause_duplicate_deliveries_on_retries | bunyan

# reorder Cargo.toml
tp:
    taplo fmt --option reorder_keys=true Cargo.toml
# For a digital ocean new deployment.
#dauth:
#    doctl auth init
#new_deploy:
#    doctl apps create --spec=spec.yaml
#update_deploy app_id:
#    doctl apps update {{app_id}} --spec=spec.yaml
