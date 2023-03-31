CREATE TABLE IF NOT EXISTS "user" (
    "discord_id" bigint NOT NULL,
    "active" boolean NOT NULL,
    "embed" boolean NOT NULL,
    "data" character varying(2000) NOT NULL,
    PRIMARY KEY ("discord_id")
)