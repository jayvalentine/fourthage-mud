use std::collections::HashMap;
use std::io;
use mlua::{FromLuaMulti, Function, IntoLuaMulti, Lua, RegistryKey, UserData};
use parking_lot::Mutex;

use crate::behavior::BehaviorAction;
use crate::event::{EventType, GameEvent};

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
    cache: HashMap<String, HashMap<String, RegistryKey>>
}

impl ScriptEngine {
    fn register_globals(lua: &Lua) -> Result<(), ScriptError> {
        let log = lua.create_function(|_, msg: String| {
            tracing::debug!("[script] {msg}");
            Ok(())
        })?;
        lua.globals().set("log", log)?;
        Ok(())
    }

    pub fn new() -> Result<Self, ScriptError> {
        let lua = Lua::new();
        Self::register_globals(&lua)?;

        let cache = HashMap::new();

        Ok(ScriptEngine { lua, cache })
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
        func.call::<A, R>(args).map_err(ScriptError::from)
    }
}

pub struct ScriptNpc {
    actions: Mutex<Vec<BehaviorAction>>
}

impl ScriptNpc {
    pub fn new() -> ScriptNpc {
        ScriptNpc { actions: Mutex::new(Vec::new()) }
    }

    pub fn actions(&self) -> Vec<BehaviorAction> {
        let mut actions = self.actions.lock();
        actions.drain(0..).collect()
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
