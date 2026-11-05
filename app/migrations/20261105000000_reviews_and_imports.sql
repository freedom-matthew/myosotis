-- Existing cards stay active and become due immediately.
ALTER TABLE cards ADD COLUMN status TEXT NOT NULL DEFAULT 'active'; -- draft | active
ALTER TABLE cards ADD COLUMN kind TEXT NOT NULL DEFAULT 'qa';
ALTER TABLE cards ADD COLUMN created_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00.000Z';
ALTER TABLE cards ADD COLUMN due TEXT NOT NULL DEFAULT '1970-01-01T00:00:00.000Z';
ALTER TABLE cards ADD COLUMN stability REAL NOT NULL DEFAULT 0;
ALTER TABLE cards ADD COLUMN difficulty REAL NOT NULL DEFAULT 0;
ALTER TABLE cards ADD COLUMN elapsed_days INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cards ADD COLUMN scheduled_days INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cards ADD COLUMN reps INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cards ADD COLUMN lapses INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cards ADD COLUMN phase TEXT NOT NULL DEFAULT 'new';
ALTER TABLE cards ADD COLUMN last_review TEXT NOT NULL DEFAULT '1970-01-01T00:00:00.000Z';

CREATE INDEX cards_status_due ON cards (status, due);

CREATE TABLE imports (
    id         INTEGER PRIMARY KEY,
    title      TEXT NOT NULL,
    full_text  TEXT,              -- deleted once the Inbox for this import is done
    status     TEXT NOT NULL,     -- generating | in_inbox | failed | done
    depth      TEXT NOT NULL,
    focus      TEXT,
    error      TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE origins (
    id           INTEGER PRIMARY KEY,
    card_id      INTEGER NOT NULL REFERENCES cards (id) ON DELETE CASCADE,
    import_id    INTEGER NOT NULL REFERENCES imports (id),
    source_title TEXT NOT NULL,
    source_kind  TEXT NOT NULL DEFAULT 'text',
    excerpt      TEXT NOT NULL,
    imported_at  TEXT NOT NULL
);

CREATE INDEX origins_card ON origins (card_id);
CREATE INDEX origins_import ON origins (import_id);

-- Append-only: the raw history FSRS could later be tuned on.
CREATE TABLE reviews (
    id          INTEGER PRIMARY KEY,
    card_id     INTEGER NOT NULL REFERENCES cards (id),
    grade       TEXT NOT NULL,
    was_new     INTEGER NOT NULL,
    reviewed_at TEXT NOT NULL,
    next_due    TEXT NOT NULL
);

CREATE INDEX reviews_card ON reviews (card_id);
CREATE INDEX reviews_reviewed_at ON reviews (reviewed_at);
