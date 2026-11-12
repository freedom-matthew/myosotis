-- A concept's other direction is now written as its own question, not the front and back
-- swapped. While a concept waits in the Inbox, its reverse waits here; accepting it creates
-- the reverse card and clears these. Nothing is dropped or converted.
ALTER TABLE card ADD COLUMN reverse_question TEXT;
ALTER TABLE card ADD COLUMN reverse_answer TEXT;
