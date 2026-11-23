-- Migration : renomme card.due en next_review_at ("date de prochaine révision"), plus explicite.

ALTER TABLE card RENAME COLUMN due TO next_review_at;

DROP INDEX idx_card_due;
CREATE INDEX idx_card_next_review_at ON card (next_review_at);
