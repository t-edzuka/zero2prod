-- Add migration script here
ALTER TABLE issue_delivery_queue
    ADD COLUMN n_retries smallint NOT NULL DEFAULT 0;
ALTER TABLE issue_delivery_queue
    ADD COLUMN retry_after timestamptz;

