CREATE TABLE IF NOT EXISTS last_events (
    script_id integer PRIMARY KEY NOT NULL,
    last_time datetime,
    write datetime,
    read datetime,
    exec datetime,
    exec_done datetime
);

