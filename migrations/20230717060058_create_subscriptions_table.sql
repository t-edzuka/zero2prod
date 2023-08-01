-- Add migration script here

--Create subscriptions.rs table
create table subscriptions
(
    id            uuid        not null primary key,
    email         text unique not null,
    name          text        not null,
    subscribed_at timestamptz not null
)