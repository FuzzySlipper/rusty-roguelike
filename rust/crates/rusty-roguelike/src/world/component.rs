use entity_state::{
    ComponentCodec, ComponentRegistration, ComponentRegistrationError, ComponentRegistry,
    ComponentTypeId, EntityComponent,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::roguelike_component_registry;

use super::{
    EnemyParticipation, Facing, WorldCell, MAX_DISCOVERED_CELLS, MAX_WORLD_FLOOR_ID_BYTES,
};

pub const PARTY_EXPLORATION_COMPONENT_TYPE_ID: &str = "rusty-roguelike.party-exploration";
pub const ENEMY_WORLD_COMPONENT_TYPE_ID: &str = "rusty-roguelike.enemy-world";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PartyExplorationComponent {
    floor_id: String,
    position: WorldCell,
    facing: Facing,
    discovered: Vec<WorldCell>,
}

impl PartyExplorationComponent {
    pub const LABEL: &'static str = "PartyExplorationComponent";

    pub fn new(
        floor_id: String,
        position: WorldCell,
        facing: Facing,
        mut discovered: Vec<WorldCell>,
    ) -> Result<Self, String> {
        discovered.sort();
        if floor_id.is_empty() || floor_id.len() > MAX_WORLD_FLOOR_ID_BYTES {
            return Err("floor identity is invalid".to_owned());
        }
        if discovered.len() > MAX_DISCOVERED_CELLS
            || discovered.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err("discovered cells are not unique and bounded".to_owned());
        }
        Ok(Self {
            floor_id,
            position,
            facing,
            discovered,
        })
    }

    pub fn floor_id(&self) -> &str {
        &self.floor_id
    }

    pub const fn position(&self) -> WorldCell {
        self.position
    }

    pub const fn facing(&self) -> Facing {
        self.facing
    }

    pub fn discovered(&self) -> &[WorldCell] {
        &self.discovered
    }

    pub(super) fn with_pose(&self, position: WorldCell, facing: Facing) -> Self {
        let mut value = self.clone();
        value.position = position;
        value.facing = facing;
        value
    }

    pub(super) fn with_discovered(&self, discovered: Vec<WorldCell>) -> Self {
        let mut value = self.clone();
        value.discovered = discovered;
        value
    }
}

impl EntityComponent for PartyExplorationComponent {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EnemyWorldComponent {
    floor_id: String,
    position: WorldCell,
    participation: EnemyParticipation,
}

impl EnemyWorldComponent {
    pub const LABEL: &'static str = "EnemyWorldComponent";

    pub fn new(
        floor_id: String,
        position: WorldCell,
        participation: EnemyParticipation,
    ) -> Result<Self, String> {
        if floor_id.is_empty() || floor_id.len() > MAX_WORLD_FLOOR_ID_BYTES {
            return Err("floor identity is invalid".to_owned());
        }
        Ok(Self {
            floor_id,
            position,
            participation,
        })
    }

    pub fn floor_id(&self) -> &str {
        &self.floor_id
    }

    pub const fn position(&self) -> WorldCell {
        self.position
    }

    pub const fn participation(&self) -> EnemyParticipation {
        self.participation
    }

    pub(super) fn participating(&self) -> Self {
        let mut value = self.clone();
        value.participation = EnemyParticipation::Participating;
        value
    }

    pub(super) fn with_position(&self, position: WorldCell) -> Self {
        let mut value = self.clone();
        value.position = position;
        value
    }
}

impl EntityComponent for EnemyWorldComponent {}

pub fn roguelike_world_component_registry() -> Result<ComponentRegistry, ComponentRegistrationError>
{
    let mut registry = roguelike_component_registry()?;
    register_world_components(&mut registry)?;
    Ok(registry)
}

pub fn register_world_components(
    registry: &mut ComponentRegistry,
) -> Result<(), ComponentRegistrationError> {
    let mut staged = registry.clone();
    staged.register(durable_registration::<PartyExplorationComponent>(
        PARTY_EXPLORATION_COMPONENT_TYPE_ID,
        "rusty-roguelike.party-exploration-json",
        validate_party_component,
    ))?;
    staged.register(durable_registration::<EnemyWorldComponent>(
        ENEMY_WORLD_COMPONENT_TYPE_ID,
        "rusty-roguelike.enemy-world-json",
        |component| validate_floor_id(component.floor_id()),
    ))?;
    *registry = staged;
    Ok(())
}

fn validate_party_component(component: &PartyExplorationComponent) -> Result<(), String> {
    validate_floor_id(component.floor_id())?;
    if component.discovered().len() > MAX_DISCOVERED_CELLS
        || component
            .discovered()
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err("discovered cells are not canonical".to_owned());
    }
    Ok(())
}

fn validate_floor_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > MAX_WORLD_FLOOR_ID_BYTES
        || value.chars().any(char::is_control)
    {
        return Err("floor identity is invalid".to_owned());
    }
    Ok(())
}

fn durable_registration<T>(
    type_id: &'static str,
    codec_id: &'static str,
    validator: fn(&T) -> Result<(), String>,
) -> ComponentRegistration<T>
where
    T: EntityComponent + Serialize + DeserializeOwned,
{
    let codec = ComponentCodec::new(
        codec_id,
        1,
        |value| serde_json::to_value(value).expect("world component serialization is infallible"),
        |value| serde_json::from_value(value).map_err(|error| error.to_string()),
    )
    .expect("fixed world component codec is valid");
    ComponentRegistration::durable(
        ComponentTypeId::parse(type_id).expect("fixed world component id is valid"),
        codec,
    )
    .with_validator(validator)
}
