use std::{collections::HashSet, path::Path, str::FromStr};

use sqlx::PgPool;

use crate::{data::{self, DataLoadError}, db::DatabaseError, entities::{AiBehavior, Description, EntityRegistry, EntityRegistryError, EventHandler, Item, Location, Name, Npc, SpawnLocation}, event::EventType, model::{ids::{Alias, RoomId}, rooms::{RoomGraph, RoomGraphNode}}, persistence};

#[derive(Debug)]
pub enum SeedError {
    Database(DatabaseError),
    DataLoad(DataLoadError),
    EntityRegistry(EntityRegistryError),
    UnknownAlias(Alias),
    Data(String),
    Io(std::io::Error)
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

impl From<std::io::Error> for SeedError {
    fn from(value: std::io::Error) -> Self {
        SeedError::Io(value)
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

        if rooms.is_empty() {
            tracing::warn!("No rooms found in data file '{}'.", data_file);
        }

        let mut seeded_count: usize = 0;

        for (id, room) in rooms {
            let alias = room.alias;
            
            let id = entities.spawn(Some(id.as_entity()), alias.clone())?;
            entities.update_component(&id, Name::from(room.name))?;
            entities.update_component(&id, Description::from(room.description))?;

            let node = RoomGraphNode::new(room.exits);
            room_graph.update_room(RoomId::from_entity(id), node);

            seeded_count += 1;
        }

        tracing::debug!("Seeded {} rooms.", seeded_count);
        
        Ok(())
    }
}

pub struct ItemSeeder;

impl Seeder for ItemSeeder {
    async fn seed(data_file: &str, pool: &PgPool, _room_graph: &RoomGraph, entities: &EntityRegistry) -> Result<(), SeedError> {
        let items = data::load_items(data_file)?;

        if items.is_empty() {
            tracing::warn!("No items found in data file '{}'.", data_file);
        }

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

pub struct NpcSeeder;

impl Seeder for NpcSeeder {
    async fn seed(data_file: &str, pool: &PgPool, _room_graph: &RoomGraph, entities: &EntityRegistry) -> Result<(), SeedError> {
        let dir = Path::new(data_file).parent().unwrap();
        let npcs = data::load_npcs(data_file)?;

        if npcs.is_empty() {
            tracing::warn!("No NPCs found in data file '{}'.", data_file);
        }

        let mut seeded_count: usize = 0;

        for (id, npc) in npcs {
            let room_id = entities.resolve_alias(&npc.spawn_location)
                .ok_or(SeedError::UnknownAlias(npc.spawn_location.clone()))?;

            let location = Location { value: room_id };
            let location = persistence::seed_location(&id, &location, pool).await?;

            let id = entities.spawn(Some(id), npc.alias.clone())?;
            entities.update_component(&id, Npc)?;
            entities.update_component(&id, Name::from(npc.name))?;
            entities.update_component(&id, Description::from(npc.description))?;
            entities.update_component(&id, location)?;
            entities.update_component(&id, SpawnLocation { value: room_id })?;

            if npc.event_subs.is_some() && npc.behavior_template.is_none() {
                return Err(SeedError::Data(format!("NPC '{}' has event subscriptions but no behavior template", npc.alias)));
            }

            if let Some(template) = &npc.behavior_template {
                let template = if std::fs::exists(dir.join(template))? {
                    dir.join(template).to_string_lossy().to_string()
                } else {
                    template.into()
                };
                entities.update_component(&id, AiBehavior { template })?;
            }

            if let Some(subs) = npc.event_subs {
                let mut subs_parsed = HashSet::new();
                for sub in subs {
                    match EventType::from_str(&sub) {
                        Ok(event_type) => {
                            subs_parsed.insert(event_type);
                        },
                        Err(_) => {
                            return Err(SeedError::Data(format!("Unknown event type '{}' for NPC '{}'", sub, npc.alias)));
                        }
                    }
                }
                entities.update_component(&id, EventHandler::new(subs_parsed))?;
            }

            seeded_count += 1;
        }

        tracing::debug!("Seeded {} NPCs.", seeded_count);

        Ok(())
    }
}
