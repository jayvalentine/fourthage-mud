use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use mlua::Error::RuntimeError;
use mlua::{FromLuaMulti, Function, IntoLuaMulti, Lua, LuaOptions, RegistryKey, StdLib, UserData};
use parking_lot::Mutex;

use crate::behavior::BehaviorAction;
use crate::entities::{AiMemory, AiMemoryValue, EntityRegistry, EntityRegistryError};
use crate::event::{EventType, GameEvent};
use crate::model::ids::EntityId;
use crate::model::rooms::Direction;

#[derive(Debug)]
pub enum ScriptError {
    Lua(mlua::Error),
    Io(io::Error),
    FunctionNotFound(String)
}

impl From<mlua::Error> for ScriptError {
    fn from(value: mlua::Error) -> Self {
        Self::Lua(value)
    }
}

impl From<io::Error> for ScriptError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub struct ScriptEngine {
    lua: Lua,
    cache: HashMap<String, HashMap<String, RegistryKey>>,
    instruction_count: Arc<AtomicU32>
}

impl ScriptEngine {
    const LUA_INSTRUCTION_LIMIT: u32 = 100_000;

    fn register_globals(lua: &Lua) -> Result<(), ScriptError> {
        let log = lua.create_function(|_, msg: String| {
            tracing::debug!("[script] {msg}");
            Ok(())
        })?;
        lua.globals().set("log", log)?;
        Ok(())
    }

    fn register_instruction_limit(lua: &Lua, instruction_count: Arc<AtomicU32>) {
        lua.set_hook(
            mlua::HookTriggers::every_nth_instruction(mlua::HookTriggers::new(), Self::LUA_INSTRUCTION_LIMIT/100),
            move |_lua, _debug| {
                let current = instruction_count.fetch_add(1, Ordering::Relaxed);
                if current > (Self::LUA_INSTRUCTION_LIMIT/1000) {
                    Err(mlua::Error::RuntimeError(
                        "script exceeded instruction limit".into()
                    ))
                } else {
                    Ok(())
                }
            }
        );
    }

    pub fn new() -> Result<Self, ScriptError> {
        let lua = Lua::new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH,
            LuaOptions::default()
        )?;
        Self::register_globals(&lua)?;

        let instruction_count = Arc::new(AtomicU32::new(0));
        Self::register_instruction_limit(&lua, instruction_count.clone());

        let cache = HashMap::new();

        Ok(ScriptEngine { lua, cache, instruction_count })
    }

    /// Load a function from a Lua script.
    fn load_function<'lua>(path: &str, function_name: &str, lua: &'lua Lua) -> Result<Function<'lua>, ScriptError> {
        let func: Function = {
            let source = std::fs::read_to_string(path)?;
            let table: mlua::Table = lua
                .load(&source)
                .set_name(path)
                .eval()?;
            let f = table
                .get(function_name)
                .map_err(|_| ScriptError::FunctionNotFound(function_name.to_string()))?;
            f
        };

        Ok(func)
    }

    pub fn call<'lua, A, R>(&'lua mut self, path: &str, function_name: &str, args: A) -> Result<R, ScriptError>
        where
            A: IntoLuaMulti<'lua>,
            R: FromLuaMulti<'lua>
    {
        if !self.cache.contains_key(path) {
            self.cache.insert(path.into(), HashMap::new());
        }

        let script = self.cache.get_mut(path).expect("Entry was just added; it must exist.");
        if !script.contains_key(function_name) {
            let function = Self::load_function(path, function_name, &self.lua)?;
            let key = self.lua.create_registry_value(function)?;
            script.insert(function_name.into(), key);
        }

        let key = script.get(function_name).expect("Entry was just added; it must exist.");

        let func: Function = self.lua.registry_value(key)?;

        self.instruction_count.store(0, Ordering::Relaxed);
        func.call::<A, R>(args).map_err(ScriptError::from)
    }
}

pub struct ScriptNpc {
    entity: EntityId,
    entity_registry: Arc<EntityRegistry>,
    actions: Mutex<Vec<BehaviorAction>>
}

impl ScriptNpc {
    pub fn new(entity: EntityId, entity_registry: Arc<EntityRegistry>) -> ScriptNpc {
        ScriptNpc {
            entity,
            entity_registry,
            actions: Mutex::new(Vec::new())
        }
    }

    pub fn actions(&self) -> Vec<BehaviorAction> {
        let mut actions = self.actions.lock();
        actions.drain(0..).collect()
    }

    /// Return the value in memory with given key and type.
    ///
    /// Returns default if no value exists with that key.
    /// Returns None if the value exists but is the wrong type.
    fn get_memory<T>(this: &ScriptNpc, key: &str, default: T) -> Result<Option<T>, EntityRegistryError>
        where Option<T>: From<AiMemoryValue>
    {
        let memory_value: Option<T> = if let Some(memory) = this.entity_registry.get_component::<AiMemory>(&this.entity)?
        {
            if let Some(value) = memory.get(key) {
                value.clone().into()
            } else {
                Some(default)
            }
        } else {
            Some(default)
        };

        Ok(memory_value)
    }

    fn set_memory<T>(this: &ScriptNpc, key: String, value: T) -> Result<(), EntityRegistryError>
        where T: Into<AiMemoryValue>
    {
        let mut memory = match this.entity_registry.get_component::<AiMemory>(&this.entity)? {
            Some(m) => m,
            None => AiMemory::new()
        };

        memory.insert(key, value.into());

        this.entity_registry.update_component(&this.entity, memory)
    }
}

impl From<EntityRegistryError> for mlua::Error {
    fn from(value: EntityRegistryError) -> Self {
        RuntimeError(format!("Error in entity registry: {:?}", value))
    }
}

impl UserData for ScriptNpc {
    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("emote", |_, this, emote: String| {
            this.actions.lock().push(BehaviorAction::Emote(emote));
            Ok(())
        });

        methods.add_method("say", |_, this, message: String| {
            this.actions.lock().push(BehaviorAction::Say(message));
            Ok(())
        });

        methods.add_method("move", |_, this, direction: String| {
            let direction = Direction::from_string(&direction)
                .map_err(|_| RuntimeError(format!("Invalid direction: {direction}")))?;
            this.actions.lock().push(BehaviorAction::Move(direction));
            Ok(())
        });

        methods.add_method("get_memory_str", |_, this, args: (String, String)| {
            let key = args.0;
            let default = args.1;

            let memory_value = match ScriptNpc::get_memory::<String>(this, &key, default)? {
                Some(m) => m,
                None => return Err(RuntimeError(format!("Unexpected type in memory for key: {key}")))
            };

            Ok(memory_value)
        });

        methods.add_method("set_memory_str", |_, this, args: (String, String)| {
            let key = args.0;
            let value = args.1;

            ScriptNpc::set_memory::<String>(this, key, value)?;
            Ok(())
        });

        methods.add_method("get_memory_int", |_, this, args: (String, i64)| {
            let key = args.0;
            let default = args.1;

            let memory_value = match ScriptNpc::get_memory::<i64>(this, &key, default)? {
                Some(m) => m,
                None => return Err(RuntimeError(format!("Unexpected type in memory for key: {key}")))
            };

            Ok(memory_value)
        });

        methods.add_method("set_memory_int", |_, this, args: (String, i64)| {
            let key = args.0;
            let value = args.1;

            ScriptNpc::set_memory::<i64>(this, key, value)?;
            Ok(())
        });
    }
}

pub struct ScriptEvent {
    inner: GameEvent
}

impl From<GameEvent> for ScriptEvent {
    fn from(value: GameEvent) -> Self {
        ScriptEvent { inner: value }
    }
}

impl UserData for ScriptEvent {
    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("type", |_, this, ()| {
            Ok(EventType::from(&this.inner).to_string())
        });

        methods.add_method("text", |_, this, ()| {
            let text = match &this.inner {
                GameEvent::Message(s) => Some(s.to_string()),
                GameEvent::PlayerSaid(_, s) => Some(s.to_string()),
                _ => None
            };
            Ok(text)
        });
    }
}
