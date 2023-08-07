-- Add migration script here

CREATE TABLE USERS
(
    user_id  UUID PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
)
