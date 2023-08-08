-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES ('4d0dea38-5c5a-4865-8de0-c9b84b52ea82',
        'd764d557-9a6a-46a5-ac35-3cfb0eb94aac',
        '$argon2id$v=19$m=15000,t=2,p=1$FZ4g+r3ZFVCKW7n61K2+fA$FrYEljfkD8qK3EgHY26tDoHI4Rz7fLDlz6cfQzWEsuI');

-- [tests/api/helpers.rs:161] &self.user_id = 4d0dea38-5c5a-4865-8de0-c9b84b52ea82
-- [tests/api/helpers.rs:162] &self.username = "d764d557-9a6a-46a5-ac35-3cfb0eb94aac"
-- [tests/api/helpers.rs:163] &self.password = "everything-has-to-start-somewhere"
-- [tests/api/helpers.rs:164] &password_hash = "$argon2id$v=19$m=15000,t=2,p=1$FZ4g+r3ZFVCKW7n61K2+fA$FrYEljfkD8qK3EgHY26tDoHI4Rz7fLDlz6cfQzWEsuI"
