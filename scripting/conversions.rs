use bevy::prelude::*;
use mlua::prelude::*;

use crate::{
    components::Health,
    monsters::{Monster, MonsterKind},
    player::Player,
};

#[derive(Clone, Debug)]
pub struct ScriptPlayer {
    pub id: u8,
    pub name: Option<String>,
    pub health: Option<i16>,
    pub score: Option<u16>,
    pub frags: Option<u16>,
    pub deaths: Option<u16>,
    pub translation: Option<ScriptVec3>,
    pub rotation: Option<ScriptQuat>,
    pub scale: Option<ScriptVec3>,
}

impl Default for ScriptPlayer {
    fn default() -> Self {
        ScriptPlayer {
            id: 0,
            name: Some("Player".to_string()),
            health: Some(100),
            score: Some(0),
            frags: Some(0),
            deaths: Some(0),
            translation: Some(ScriptVec3::default()),
            rotation: Some(ScriptQuat::default()),
            scale: Some(ScriptVec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            }),
        }
    }
}

impl ScriptPlayer {
    pub fn from_components(
        transform: &Transform,
        health_component: &Health,
        player: &Player,
    ) -> Self {
        ScriptPlayer {
            id: player.id,
            name: Some(player.name.clone()),
            health: Some(health_component.current()),
            score: Some(player.score),
            frags: Some(player.frags),
            deaths: Some(player.deaths),
            translation: Some(transform.translation.into()),
            rotation: Some(transform.rotation.into()),
            scale: Some(transform.scale.into()),
        }
    }

    pub fn update_components(
        &self,
        transform: &mut Transform,
        health_component: &mut Health,
        player: &mut Player,
    ) {
        if let Some(translation) = &self.translation {
            transform.translation = translation.into();
        }
        if let Some(rotation) = &self.rotation {
            transform.rotation = rotation.into();
        }
        if let Some(scale) = &self.scale {
            transform.scale = scale.into();
        }
        if let Some(health) = self.health {
            health_component.set_current(health);
        }
        if let Some(score) = self.score {
            player.score = score;
        }
        if let Some(frags) = self.frags {
            player.frags = frags;
        }
        if let Some(deaths) = self.deaths {
            player.deaths = deaths;
        }
    }
}

impl<'lua> FromLua<'lua> for ScriptPlayer {
    fn from_lua(value: mlua::Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        if let Some(table) = value.as_table() {
            let id = table.get("id")?;

            let name = if let Ok(name) = table.get("name") {
                Some(name)
            } else {
                None
            };

            let health = if let Ok(health) = table.get("health") {
                Some(health)
            } else {
                None
            };

            let score = if let Ok(score) = table.get("score") {
                Some(score)
            } else {
                None
            };

            let frags = if let Ok(frags) = table.get("frags") {
                Some(frags)
            } else {
                None
            };

            let deaths = if let Ok(deaths) = table.get("deaths") {
                Some(deaths)
            } else {
                None
            };

            let translation = if let Ok(translation) = table.get("translation") {
                Some(translation)
            } else {
                None
            };

            let rotation = if let Ok(rotation) = table.get("rotation") {
                Some(rotation)
            } else {
                None
            };

            let scale = if let Ok(scale) = table.get("scale") {
                Some(scale)
            } else {
                None
            };

            Ok(ScriptPlayer {
                id,
                name,
                health,
                score,
                frags,
                deaths,
                translation,
                rotation,
                scale,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "ScriptPlayer",
                message: Some("expected table".to_string()),
            })
        }
    }
}

impl<'lua> IntoLua<'lua> for ScriptPlayer {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<mlua::Value<'lua>> {
        let table = lua.create_table()?;
        table.set("id", self.id)?;
        if let Some(name) = self.name {
            table.set("name", name)?;
        }
        if let Some(health) = self.health {
            table.set("health", health)?;
        }
        if let Some(score) = self.score {
            table.set("score", score)?;
        }
        if let Some(frags) = self.frags {
            table.set("frags", frags)?;
        }
        if let Some(deaths) = self.deaths {
            table.set("deaths", deaths)?;
        }
        if let Some(translation) = self.translation {
            table.set("translation", translation)?;
        }
        if let Some(rotation) = self.rotation {
            table.set("rotation", rotation)?;
        }
        if let Some(scale) = self.scale {
            table.set("scale", scale)?;
        }
        Ok(mlua::Value::Table(table))
    }
}

#[derive(Clone, Debug)]
pub struct ScriptMonster {
    pub id: u8,
    pub kind: MonsterKind,
    pub health: Option<i16>,
    pub translation: Option<ScriptVec3>,
    pub rotation: Option<ScriptQuat>,
    pub scale: Option<ScriptVec3>,
}

impl Default for ScriptMonster {
    fn default() -> Self {
        ScriptMonster {
            id: 0,
            kind: MonsterKind::GruntPlasma,
            health: Some(100),
            translation: Some(ScriptVec3::default()),
            rotation: Some(ScriptQuat::default()),
            scale: Some(ScriptVec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            }),
        }
    }
}

impl ScriptMonster {
    pub fn from_components(
        transform: &Transform,
        health_component: &Health,
        monster: &Monster,
    ) -> Self {
        ScriptMonster {
            id: 0,
            kind: monster.kind.clone(),
            health: Some(health_component.current()),
            translation: Some(transform.translation.into()),
            rotation: Some(transform.rotation.into()),
            scale: Some(transform.scale.into()),
        }
    }

    pub fn update_components(
        &self,
        transform: &mut Transform,
        health_component: &mut Health,
        _monster: &mut Monster,
    ) {
        if let Some(translation) = &self.translation {
            transform.translation = translation.into();
        }
        if let Some(rotation) = &self.rotation {
            transform.rotation = rotation.into();
        }
        if let Some(scale) = &self.scale {
            transform.scale = scale.into();
        }
        if let Some(health) = self.health {
            health_component.set_current(health);
        }
    }
}

impl<'lua> FromLua<'lua> for ScriptMonster {
    fn from_lua(value: mlua::Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        if let Some(table) = value.as_table() {
            let id = table.get("id")?;
            let kind = table.get("kind")?;

            let health = if let Ok(health) = table.get("health") {
                Some(health)
            } else {
                None
            };

            let translation = if let Ok(translation) = table.get("translation") {
                Some(translation)
            } else {
                None
            };

            let rotation = if let Ok(rotation) = table.get("rotation") {
                Some(rotation)
            } else {
                None
            };

            let scale = if let Ok(scale) = table.get("scale") {
                Some(scale)
            } else {
                None
            };

            Ok(ScriptMonster {
                id,
                kind,
                health,
                translation,
                rotation,
                scale,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "ScriptMonster",
                message: Some("expected table".to_string()),
            })
        }
    }
}

impl<'lua> IntoLua<'lua> for ScriptMonster {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<mlua::Value<'lua>> {
        let table = lua.create_table()?;
        table.set("id", self.id)?;
        table.set("kind", self.kind)?;
        if let Some(health) = self.health {
            table.set("health", health)?;
        }
        if let Some(translation) = self.translation {
            table.set("translation", translation)?;
        }
        if let Some(rotation) = self.rotation {
            table.set("rotation", rotation)?;
        }
        if let Some(scale) = self.scale {
            table.set("scale", scale)?;
        }
        Ok(mlua::Value::Table(table))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScriptVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Into<Vec3> for ScriptVec3 {
    fn into(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

impl Into<Vec3> for &ScriptVec3 {
    fn into(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

impl From<Vec3> for ScriptVec3 {
    fn from(vec: Vec3) -> Self {
        ScriptVec3 {
            x: vec.x,
            y: vec.y,
            z: vec.z,
        }
    }
}

impl<'lua> FromLua<'lua> for ScriptVec3 {
    fn from_lua(value: mlua::Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        if let Some(table) = value.as_table() {
            Ok(ScriptVec3 {
                x: table.get("x")?,
                y: table.get("y")?,
                z: table.get("z")?,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "ScriptVec3",
                message: Some("expected table".to_string()),
            })
        }
    }
}
impl<'lua> IntoLua<'lua> for ScriptVec3 {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<mlua::Value<'lua>> {
        let table = lua.create_table()?;
        table.set("x", self.x)?;
        table.set("y", self.y)?;
        table.set("z", self.z)?;
        Ok(mlua::Value::Table(table))
    }
}

#[derive(Clone, Debug)]
pub struct ScriptSpawnMonster {
    pub kind: MonsterKind,
    pub scale: Option<ScriptVec3>,
    pub health: Option<i16>,
}

impl Default for ScriptSpawnMonster {
    fn default() -> Self {
        ScriptSpawnMonster {
            kind: MonsterKind::GruntPlasma,
            scale: Some(ScriptVec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            }),
            health: Some(100),
        }
    }
}

impl FromLua<'_> for ScriptSpawnMonster {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> LuaResult<Self> {
        if let Some(table) = value.as_table() {
            let kind = table.get("kind")?;
            let scale = if let Ok(scale) = table.get("scale") {
                Some(scale)
            } else {
                None
            };
            let health = if let Ok(health) = table.get("health") {
                Some(health)
            } else {
                None
            };
            Ok(ScriptSpawnMonster {
                kind,
                scale,
                health,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "ScriptSpawnMonster",
                message: Some("expected table".to_string()),
            })
        }
    }
}

impl IntoLua<'_> for ScriptSpawnMonster {
    fn into_lua(self, lua: &Lua) -> LuaResult<mlua::Value> {
        let table = lua.create_table()?;
        table.set("kind", self.kind)?;
        if let Some(scale) = self.scale {
            table.set("scale", scale)?;
        }
        if let Some(health) = self.health {
            table.set("health", health)?;
        }
        Ok(mlua::Value::Table(table))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScriptQuat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Into<Quat> for ScriptQuat {
    fn into(self) -> Quat {
        Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}

impl Into<Quat> for &ScriptQuat {
    fn into(self) -> Quat {
        Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}

impl From<Quat> for ScriptQuat {
    fn from(quat: Quat) -> Self {
        ScriptQuat {
            x: quat.x,
            y: quat.y,
            z: quat.z,
            w: quat.w,
        }
    }
}

impl<'lua> FromLua<'lua> for ScriptQuat {
    fn from_lua(value: mlua::Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        if let Some(table) = value.as_table() {
            Ok(ScriptQuat {
                x: table.get("x")?,
                y: table.get("y")?,
                z: table.get("z")?,
                w: table.get("w")?,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "ScriptQuat",
                message: Some("expected table".to_string()),
            })
        }
    }
}

impl<'lua> IntoLua<'lua> for ScriptQuat {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<mlua::Value<'lua>> {
        let table = lua.create_table()?;
        table.set("x", self.x)?;
        table.set("y", self.y)?;
        table.set("z", self.z)?;
        table.set("w", self.w)?;
        Ok(mlua::Value::Table(table))
    }
}
