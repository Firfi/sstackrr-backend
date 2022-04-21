create type bot_type as enum ('RANDY');

ALTER TABLE games
    ADD bot_id bot_type;