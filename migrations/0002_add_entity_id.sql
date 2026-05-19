CREATE TABLE positions (
    entity_id UUID NOT NULL,
    room_id INTEGER NOT NULL
);

INSERT INTO positions (entity_id, room_id)
SELECT id, current_room_id FROM accounts;

ALTER TABLE accounts DROP COLUMN current_room_id;
