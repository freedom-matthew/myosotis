-- Cards come in three kinds: question, concept (reviewed both ways) and describe.
UPDATE card SET kind = 'question' WHERE kind = 'qa';
UPDATE card SET kind = 'concept' WHERE kind = 'reverse';

-- Fill-the-gap cards become questions with the gap left blank; the answer already is the
-- gap's text, and review history is kept.
UPDATE card
SET question = substr(question, 1, instr(question, '{{') - 1) || '___'
             || substr(question, instr(question, '}}') + 2)
WHERE kind = 'cloze' AND instr(question, '{{') > 0 AND instr(question, '}}') > instr(question, '{{');
UPDATE card SET kind = 'question' WHERE kind = 'cloze';

ALTER TABLE import DROP COLUMN fill_in_blank;
ALTER TABLE import ADD COLUMN describe INTEGER NOT NULL DEFAULT 1;
