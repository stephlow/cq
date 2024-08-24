CREATE TABLE servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    addr INET NOT NULL,
    port INTEGER NOT NULL,
    last_ping TIMESTAMPTZ NOT NULL
)
