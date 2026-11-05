ALTER TABLE cards RENAME TO card;
ALTER TABLE imports RENAME TO import;
ALTER TABLE origins RENAME TO origin;
ALTER TABLE reviews RENAME TO review;

DROP INDEX cards_status_due;
DROP INDEX origins_card;
DROP INDEX origins_import;
DROP INDEX reviews_card;
DROP INDEX reviews_reviewed_at;

CREATE INDEX card_status_due ON card (status, due);
CREATE INDEX origin_card ON origin (card_id);
CREATE INDEX origin_import ON origin (import_id);
CREATE INDEX review_card ON review (card_id);
CREATE INDEX review_reviewed_at ON review (reviewed_at);
