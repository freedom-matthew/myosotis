ALTER TABLE card ADD COLUMN duplicate_of INTEGER;  -- a draft that resembles this existing card

-- Pairs the user said are different, so Tidy up doesn't propose them again.
CREATE TABLE not_duplicate (
    card_a INTEGER NOT NULL,
    card_b INTEGER NOT NULL,
    PRIMARY KEY (card_a, card_b)
);
