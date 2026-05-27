DROP TABLE positions;
CREATE TABLE positions (
    entity_id UUID PRIMARY KEY,
    room_id UUID NOT NULL
);
