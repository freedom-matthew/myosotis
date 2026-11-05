ALTER TABLE card ADD COLUMN sibling_id INTEGER;         -- the other direction of a reverse card
ALTER TABLE import ADD COLUMN fill_in_blank INTEGER NOT NULL DEFAULT 1;
ALTER TABLE import ADD COLUMN parts_total INTEGER NOT NULL DEFAULT 1;
ALTER TABLE import ADD COLUMN parts_done INTEGER NOT NULL DEFAULT 0;
