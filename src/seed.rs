use sqlx::PgPool;

use crate::{data::{self, DataLoadError}, db::DatabaseError, entities::{EntityRegistry, EntityRegistryError, Item, Location, Name, SpawnLocation}, model::{ids::Alias, world::World}, persistence};

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
        world: &World,
        entities: &EntityRegistry
    ) -> Result<(), SeedError>;
}

pub struct ItemSeeder;

impl Seeder for ItemSeeder {
    async fn seed(data_file: &str, pool: &PgPool, world: &World, entities: &EntityRegistry) -> Result<(), SeedError> {
        let items = data::load_items(data_file)?;

        for (id, item) in &items {
            let room_id = world.resolve_alias(&item.spawn_location)
                .ok_or(SeedError::UnknownAlias(item.spawn_location.clone()))?;

            let location = Location { value: room_id.as_entity() };
            let location = persistence::seed_location(id, &location, pool).await?;

            let id = entities.spawn(Some(*id), item.alias.clone())?;
            entities.update_component(&id, Item)?;
            entities.update_component(&id, Name { value: item.name.clone() })?;
            entities.update_component(&id, location)?;
            entities.update_component(&id, SpawnLocation { value: room_id.as_entity() })?;
        }

        tracing::debug!("Seeded {} items.", items.len());

        Ok(())
    }
}
