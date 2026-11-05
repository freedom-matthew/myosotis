-- The schedule from before each review, so a review can be undone.
-- Older reviews have none and can't be undone.
ALTER TABLE reviews ADD COLUMN previous_due TEXT;
ALTER TABLE reviews ADD COLUMN previous_stability REAL;
ALTER TABLE reviews ADD COLUMN previous_difficulty REAL;
ALTER TABLE reviews ADD COLUMN previous_elapsed_days INTEGER;
ALTER TABLE reviews ADD COLUMN previous_scheduled_days INTEGER;
ALTER TABLE reviews ADD COLUMN previous_reps INTEGER;
ALTER TABLE reviews ADD COLUMN previous_lapses INTEGER;
ALTER TABLE reviews ADD COLUMN previous_phase TEXT;
ALTER TABLE reviews ADD COLUMN previous_last_review TEXT;
