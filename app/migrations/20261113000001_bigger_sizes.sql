-- Every size step got bigger: the new Default is what Largest was. A saved Larger or Largest
-- moves to Default, so nobody's window suddenly grows past what they chose.
UPDATE setting SET value = 'default' WHERE key = 'size' AND value IN ('larger', 'largest');
