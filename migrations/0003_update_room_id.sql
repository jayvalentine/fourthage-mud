DELETE FROM positions;
ALTER TABLE positions DROP COLUMN room_id;
ALTER TABLE positions ADD COLUMN room_id UUID NOT NULL;
