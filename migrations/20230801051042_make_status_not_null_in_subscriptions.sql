-- Add migration script here
BEGIN;
-- Change status from NULL to 'confirmed' for existing rows
UPDATE subscriptions
SET status='confirmed'
WHERE status IS NULL;
-- Make status column NOT NULL
ALTER TABLE subscriptions
    ALTER COLUMN status SET NOT NULL;
COMMIT;