CREATE TABLE games (
                       id UUID PRIMARY KEY,
                       player_red UUID,
                       player_blue UUID,
                       state TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_player_red
    ON games(player_red);

CREATE UNIQUE INDEX player_blue
    ON games(player_blue);