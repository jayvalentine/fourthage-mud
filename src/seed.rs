use sqlx::PgPool;

use crate::{data::{self, DataLoadError}, db::DatabaseError, entities::{Description, EntityRegistry, EntityRegistryError, Item, Location, Name, SpawnLocation}, model::{ids::{Alias, RoomId}, rooms::{RoomGraph, RoomGraphNode}}, persistence};

#[derive(Debug)]
pub enum SeedError {
    Database(DatabaseError),
    DataLoad(DataLoadError),
    EntityRegistry(EntityRegistryError),
    UnknownAlias(Alias),
}

impl From<DatabaseError> for SeedError {
    fn from(value: DatabaseError) -> Self {
        SeedError::Database(value)
    }
}

impl From<DataLoadError> for SeedError {
    fn from(value: DataLoadError) -> Self {
        SeedError::DataLoad(value)
    }
}

impl From<EntityRegistryError> for SeedError {
    fn from(value: EntityRegistryError) -> Self {
        SeedError::EntityRegistry(value)
    }
}

pub trait Seeder {
    async fn seed(
        data_file: &str,
        pool: &PgPool,
        room_graph: &RoomGraph,
        entities: &EntityRegistry
    ) -> Result<(), SeedError>;
}

pub struct RoomSeeder;

impl Seeder for RoomSeeder {
    async fn seed(data_file: &str, _pool: &PgPool, room_graph: &RoomGraph, entities: &EntityRegistry) -> Result<(), SeedError>
    {
        let rooms = data::load_rooms(data_file)?;

        for (id, room) in rooms {
            let alias = room.alias;
            
            let id = entities.spawn(Some(id.as_entity()), alias.clone())?;
            entities.update_component(&id, Name::from(room.name))?;
            entities.update_component(&id, Description::from(room.description))?;

            let node = RoomGraphNode::new(room.exits);
            room_graph.update_room(RoomId::from_entity(id), node);
        }
        Ok(())
    }
}

pub struct ItemSeeder;

impl Seeder for ItemSeeder {
    async fn seed(data_file: &str, pool: &PgPool, _room_graph: &RoomGraph, entities: &EntityRegistry) -> Result<(), SeedError> {
        let items = data::load_items(data_file)?;

        let mut seeded_count: usize = 0;

        for (id, item) in items {
            let room_id = entities.resolve_alias(&item.spawn_location)
                .ok_or(SeedError::UnknownAlias(item.spawn_location.clone()))?;

            let location = Location { value: room_id };
            let location = persistence::seed_location(&id, &location, pool).await?;

            let id = entities.spawn(Some(id), item.alias.clone())?;
            entities.update_component(&id, Item)?;
            entities.update_component(&id, Name::from(item.name))?;
            entities.update_component(&id, Description::from(item.description))?;
            entities.update_component(&id, location)?;
            entities.update_component(&id, SpawnLocation { value: room_id })?;

            seeded_count += 1;
        }

        tracing::debug!("Seeded {} items.", seeded_count);

        Ok(())
    }
}
