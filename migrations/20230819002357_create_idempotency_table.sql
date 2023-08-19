-- Add migration script here
create table header_pair
(
    name  text,
    value bytea
);

create table idempotency
(
    user_id              uuid          NOT NULL,
    idempotency_key      TEXT          NOT NULL,
    response_status_code smallint      NOT NULL,
    response_headers     header_pair[] NOT NULL,
    response_body        bytea         NOT NULL,
    created_at timestamptz NOT NULL,
    primary key (user_id, idempotency_key)

);
