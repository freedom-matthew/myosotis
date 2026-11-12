-- Desired retention is no longer a setting I can see, so it goes back to FSRS's default of 90%.
UPDATE setting SET value = '90' WHERE key = 'desired_retention_percent';
