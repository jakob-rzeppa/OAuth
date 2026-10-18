CREATE TABLE IF NOT EXISTS "users" (
    "id"                        UUID            PRIMARY KEY,

    -- The unique username of the user. Used for login and auth.
    "user_name"                 VARCHAR(255)    NOT NULL UNIQUE,
    -- The display name of the user. Used for display purposes.
    "display_name"              VARCHAR(255)    NOT NULL,

    "password_hash"             VARCHAR(255)    NOT NULL,
    "has_temporary_password"    BOOLEAN         NOT NULL,

    -- The actual roles are set at seed/roles.yaml. Here we only reference the role ids.
    "roles"                     UUID[]          NOT NULL
);
