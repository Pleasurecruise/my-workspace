CREATE TABLE IF NOT EXISTS dashboard_widgets (
    id TEXT PRIMARY KEY NOT NULL,
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    configuration TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS dashboard_layout (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    island_widget_id TEXT REFERENCES dashboard_widgets(id) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE IF NOT EXISTS todo_items (
    date TEXT NOT NULL,
    id TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    text TEXT NOT NULL CHECK (length(text) BETWEEN 1 AND 120),
    completed BOOLEAN NOT NULL CHECK (completed IN (0, 1)),
    rollover BOOLEAN NOT NULL DEFAULT 0 CHECK (rollover IN (0, 1)),
    calendar TEXT,
    start_date TEXT,
    start_time TEXT,
    end_date TEXT,
    end_time TEXT,
    location TEXT,
    description TEXT,
    PRIMARY KEY (date, id),
    UNIQUE (date, position),
    CHECK ((calendar IS NOT NULL AND start_date IS NOT NULL) OR
        (calendar IS NULL AND start_date IS NULL AND start_time IS NULL AND end_date IS NULL
        AND end_time IS NULL AND location IS NULL))
);
CREATE TABLE IF NOT EXISTS todo_occurrences (
    date TEXT NOT NULL,
    key TEXT NOT NULL,
    PRIMARY KEY (date, key)
);
CREATE TABLE IF NOT EXISTS credentials (
    account TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY NOT NULL,
    position INTEGER NOT NULL UNIQUE,
    topic TEXT NOT NULL,
    source TEXT NOT NULL,
    title TEXT,
    message TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    tags TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS notification_cursor (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    last_id TEXT
);
CREATE TABLE IF NOT EXISTS telegram_session (id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1), data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS game_accounts (
    game TEXT NOT NULL, uid TEXT NOT NULL, name TEXT NOT NULL, region TEXT NOT NULL,
    role_id TEXT NOT NULL, synced_at BIGINT NOT NULL, PRIMARY KEY(game, uid)
);
CREATE TABLE IF NOT EXISTS game_pulls (
    game TEXT NOT NULL, uid TEXT NOT NULL, id TEXT NOT NULL, pool TEXT NOT NULL,
    pool_name TEXT NOT NULL, item_id TEXT, name TEXT NOT NULL,
    rarity INTEGER NOT NULL CHECK(rarity BETWEEN 1 AND 6), time TEXT NOT NULL,
    is_free BOOLEAN, is_new BOOLEAN,
    PRIMARY KEY(game, uid, id), FOREIGN KEY(game, uid) REFERENCES game_accounts(game, uid)
);
CREATE TABLE IF NOT EXISTS game_reports (
    game TEXT NOT NULL, uid TEXT NOT NULL, payload TEXT NOT NULL,
    PRIMARY KEY(game, uid), FOREIGN KEY(game, uid) REFERENCES game_accounts(game, uid)
);
CREATE TABLE IF NOT EXISTS game_diagnostic (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    game TEXT NOT NULL, stage TEXT NOT NULL, retcode BIGINT NOT NULL,
    has_trace BOOLEAN NOT NULL, timestamp BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS check_ins (
    id TEXT NOT NULL,
    date TEXT NOT NULL,
    PRIMARY KEY (id, date)
);

CREATE TABLE IF NOT EXISTS ledger_entries (
    id TEXT PRIMARY KEY NOT NULL,
    date TEXT NOT NULL,
    amount_pence BIGINT NOT NULL CHECK (amount_pence BETWEEN 1 AND 99999999),
    category TEXT NOT NULL CHECK (length(category) BETWEEN 1 AND 40),
    description TEXT CHECK (description IS NULL OR length(description) <= 500),
    created_at BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS ledger_entries_date ON ledger_entries(date);
