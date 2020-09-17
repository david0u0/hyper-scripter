CREATE TABLE IF NOT EXISTS events (
    id integer PRIMARY KEY NOT NULL,
    script_id integer NOT NULL,
    type varchar(10) NOT NULL,
    cmd text NOT NULL,
    content text,
    time timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

