use rusty_engine::entity_state::{
    ComponentCodec, ComponentRegistration, ComponentRegistrationError, ComponentRegistry,
    ComponentTypeId, EntityComponent,
};
use rusty_engine::gameplay_mechanics::gameplay_component_registry;
use rusty_engine::gameplay_rules::MAX_SAFE_JSON_INTEGER;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::RoguelikeId;

pub const ABILITY_SCORES_COMPONENT_TYPE_ID: &str = "rusty-roguelike.ability-scores";
pub const ACTOR_BUILD_COMPONENT_TYPE_ID: &str = "rusty-roguelike.actor-build";
pub const COLLAPSED_PARTY_COMPONENT_TYPE_ID: &str = "rusty-roguelike.collapsed-party";

const MAX_COMPONENT_ENTRIES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AbilityScore {
    id: RoguelikeId,
    score: i16,
}

impl AbilityScore {
    pub const fn new(id: RoguelikeId, score: i16) -> Self {
        Self { id, score }
    }

    pub const fn id(&self) -> &RoguelikeId {
        &self.id
    }

    pub const fn score(&self) -> i16 {
        self.score
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AbilityScoresComponent {
    scores: Vec<AbilityScore>,
}

impl AbilityScoresComponent {
    pub const LABEL: &'static str = "AbilityScoresComponent";

    pub fn new(mut scores: Vec<AbilityScore>) -> Result<Self, RoguelikeComponentDataError> {
        scores.sort_by(|left, right| left.id.cmp(&right.id));
        validate_unique(&scores, "scores", |entry| entry.id.as_str())?;
        if scores.iter().any(|entry| !(1..=30).contains(&entry.score)) {
            return Err(RoguelikeComponentDataError::InvalidAbilityScore);
        }
        Ok(Self { scores })
    }

    pub fn scores(&self) -> &[AbilityScore] {
        &self.scores
    }
}

impl EntityComponent for AbilityScoresComponent {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActorBuildComponent {
    class: RoguelikeId,
    class_level: u8,
    actions: Vec<RoguelikeId>,
    feats: Vec<RoguelikeId>,
    items: Vec<RoguelikeId>,
}

impl ActorBuildComponent {
    pub const LABEL: &'static str = "ActorBuildComponent";

    pub fn new(
        class: RoguelikeId,
        class_level: u8,
        mut actions: Vec<RoguelikeId>,
        mut feats: Vec<RoguelikeId>,
        mut items: Vec<RoguelikeId>,
    ) -> Result<Self, RoguelikeComponentDataError> {
        actions.sort();
        feats.sort();
        items.sort();
        validate_unique(&actions, "actions", RoguelikeId::as_str)?;
        validate_unique(&feats, "feats", RoguelikeId::as_str)?;
        validate_unique(&items, "items", RoguelikeId::as_str)?;
        if !(1..=20).contains(&class_level) {
            return Err(RoguelikeComponentDataError::InvalidClassLevel);
        }
        Ok(Self {
            class,
            class_level,
            actions,
            feats,
            items,
        })
    }

    pub const fn class(&self) -> &RoguelikeId {
        &self.class
    }

    pub const fn class_level(&self) -> u8 {
        self.class_level
    }

    pub fn actions(&self) -> &[RoguelikeId] {
        &self.actions
    }

    pub fn feats(&self) -> &[RoguelikeId] {
        &self.feats
    }

    pub fn items(&self) -> &[RoguelikeId] {
        &self.items
    }
}

impl EntityComponent for ActorBuildComponent {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CollapsedPartyComponent {
    member_entity_ids: Vec<u64>,
}

impl CollapsedPartyComponent {
    pub const LABEL: &'static str = "CollapsedPartyComponent";

    pub fn new(mut member_entity_ids: Vec<u64>) -> Result<Self, RoguelikeComponentDataError> {
        member_entity_ids.sort_unstable();
        if !(2..=4).contains(&member_entity_ids.len()) {
            return Err(RoguelikeComponentDataError::InvalidPartySize {
                actual: member_entity_ids.len(),
            });
        }
        if member_entity_ids.contains(&0)
            || member_entity_ids
                .iter()
                .any(|entity| *entity > MAX_SAFE_JSON_INTEGER)
            || member_entity_ids.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(RoguelikeComponentDataError::DuplicateOrZeroMember);
        }
        Ok(Self { member_entity_ids })
    }

    pub fn member_entity_ids(&self) -> &[u64] {
        &self.member_entity_ids
    }
}

impl EntityComponent for CollapsedPartyComponent {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoguelikeComponentDataError {
    QuotaExceeded {
        field: &'static str,
    },
    DuplicateIdentity {
        field: &'static str,
        identity: String,
    },
    InvalidClassLevel,
    InvalidAbilityScore,
    InvalidPartySize {
        actual: usize,
    },
    DuplicateOrZeroMember,
}

impl std::fmt::Display for RoguelikeComponentDataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid Roguelike component data: {self:?}")
    }
}

impl std::error::Error for RoguelikeComponentDataError {}

pub fn roguelike_component_registry() -> Result<ComponentRegistry, ComponentRegistrationError> {
    let mut registry = gameplay_component_registry()?;
    register_roguelike_components(&mut registry)?;
    Ok(registry)
}

pub fn register_roguelike_components(
    registry: &mut ComponentRegistry,
) -> Result<(), ComponentRegistrationError> {
    let mut staged = registry.clone();
    staged.register(durable_registration::<AbilityScoresComponent>(
        ABILITY_SCORES_COMPONENT_TYPE_ID,
        "rusty-roguelike.ability-scores-json",
        |component| {
            validate_canonical(component.scores(), |entry| entry.id.as_str())?;
            if component
                .scores()
                .iter()
                .any(|entry| !(1..=30).contains(&entry.score()))
            {
                return Err("ability score is outside 1..=30".to_owned());
            }
            Ok(())
        },
    ))?;
    staged.register(durable_registration::<ActorBuildComponent>(
        ACTOR_BUILD_COMPONENT_TYPE_ID,
        "rusty-roguelike.actor-build-json",
        |component| {
            if !(1..=20).contains(&component.class_level()) {
                return Err("class level is outside 1..=20".to_owned());
            }
            validate_canonical(component.actions(), RoguelikeId::as_str)?;
            validate_canonical(component.feats(), RoguelikeId::as_str)?;
            validate_canonical(component.items(), RoguelikeId::as_str)
        },
    ))?;
    staged.register(durable_registration::<CollapsedPartyComponent>(
        COLLAPSED_PARTY_COMPONENT_TYPE_ID,
        "rusty-roguelike.collapsed-party-json",
        |component| {
            if !(2..=4).contains(&component.member_entity_ids().len())
                || component.member_entity_ids().contains(&0)
                || component
                    .member_entity_ids()
                    .iter()
                    .any(|entity| *entity > MAX_SAFE_JSON_INTEGER)
                || component
                    .member_entity_ids()
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
            {
                return Err("party member entity ids are not canonical".to_owned());
            }
            Ok(())
        },
    ))?;
    *registry = staged;
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
        |value| serde_json::to_value(value).expect("component serialization is infallible"),
        |value| serde_json::from_value(value).map_err(|error| error.to_string()),
    )
    .expect("fixed Roguelike component codec is valid");
    ComponentRegistration::durable(
        ComponentTypeId::parse(type_id).expect("fixed Roguelike component id is valid"),
        codec,
    )
    .with_validator(validator)
}

fn validate_unique<T>(
    values: &[T],
    field: &'static str,
    identity: impl Fn(&T) -> &str,
) -> Result<(), RoguelikeComponentDataError> {
    if values.len() > MAX_COMPONENT_ENTRIES {
        return Err(RoguelikeComponentDataError::QuotaExceeded { field });
    }
    if let Some(pair) = values
        .windows(2)
        .find(|pair| identity(&pair[0]) == identity(&pair[1]))
    {
        return Err(RoguelikeComponentDataError::DuplicateIdentity {
            field,
            identity: identity(&pair[1]).to_owned(),
        });
    }
    Ok(())
}

fn validate_canonical<T>(values: &[T], identity: impl Fn(&T) -> &str) -> Result<(), String> {
    if values.len() > MAX_COMPONENT_ENTRIES {
        return Err("component entry quota exceeded".to_owned());
    }
    if values
        .windows(2)
        .any(|pair| identity(&pair[0]) >= identity(&pair[1]))
    {
        return Err("component entries are not canonical".to_owned());
    }
    Ok(())
}
