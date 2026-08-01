use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{RelativeStep, RoguelikeId, WorldView};

pub const SESSION_VIEW_SCHEMA_VERSION: u32 = 1;
pub const MAX_SESSION_ACTIVATIONS: usize = 64;
pub const MAX_SESSION_RECEIPTS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum TurnSide {
    Party,
    Opposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum SessionOutcome {
    Ongoing,
    Victory,
    Defeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum PartyMemberSelectionPolicy {
    RoundRobinLiving,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PartySquareTargetReceipt {
    #[ts(type = "number")]
    pub selected_member_entity_id: u64,
    pub selection_policy: PartyMemberSelectionPolicy,
    pub eligible_member_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartyCommand {
    Step {
        actor_entity_id: u64,
        expected_revision: u64,
        step: RelativeStep,
    },
    TurnLeft {
        actor_entity_id: u64,
        expected_revision: u64,
    },
    TurnRight {
        actor_entity_id: u64,
        expected_revision: u64,
    },
    UseAction {
        actor_entity_id: u64,
        expected_revision: u64,
        action_id: RoguelikeId,
        target_entity_id: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, TS)]
#[ts(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionCommandDto {
    Step {
        #[ts(type = "number")]
        actor_entity_id: u64,
        #[ts(type = "number")]
        expected_revision: u64,
        step: RelativeStep,
    },
    TurnLeft {
        #[ts(type = "number")]
        actor_entity_id: u64,
        #[ts(type = "number")]
        expected_revision: u64,
    },
    TurnRight {
        #[ts(type = "number")]
        actor_entity_id: u64,
        #[ts(type = "number")]
        expected_revision: u64,
    },
    UseAction {
        #[ts(type = "number")]
        actor_entity_id: u64,
        #[ts(type = "number")]
        expected_revision: u64,
        action_id: RoguelikeId,
        #[ts(type = "number")]
        target_entity_id: u64,
    },
}

impl<'de> Deserialize<'de> for SessionCommandDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(
            deny_unknown_fields,
            tag = "kind",
            rename_all = "camelCase",
            rename_all_fields = "camelCase"
        )]
        enum StrictSessionCommandDto {
            Step {
                actor_entity_id: u64,
                expected_revision: u64,
                step: RelativeStep,
            },
            TurnLeft {
                actor_entity_id: u64,
                expected_revision: u64,
            },
            TurnRight {
                actor_entity_id: u64,
                expected_revision: u64,
            },
            UseAction {
                actor_entity_id: u64,
                expected_revision: u64,
                action_id: RoguelikeId,
                target_entity_id: u64,
            },
        }

        Ok(match StrictSessionCommandDto::deserialize(deserializer)? {
            StrictSessionCommandDto::Step {
                actor_entity_id,
                expected_revision,
                step,
            } => Self::Step {
                actor_entity_id,
                expected_revision,
                step,
            },
            StrictSessionCommandDto::TurnLeft {
                actor_entity_id,
                expected_revision,
            } => Self::TurnLeft {
                actor_entity_id,
                expected_revision,
            },
            StrictSessionCommandDto::TurnRight {
                actor_entity_id,
                expected_revision,
            } => Self::TurnRight {
                actor_entity_id,
                expected_revision,
            },
            StrictSessionCommandDto::UseAction {
                actor_entity_id,
                expected_revision,
                action_id,
                target_entity_id,
            } => Self::UseAction {
                actor_entity_id,
                expected_revision,
                action_id,
                target_entity_id,
            },
        })
    }
}

impl From<SessionCommandDto> for PartyCommand {
    fn from(value: SessionCommandDto) -> Self {
        match value {
            SessionCommandDto::Step {
                actor_entity_id,
                expected_revision,
                step,
            } => Self::Step {
                actor_entity_id,
                expected_revision,
                step,
            },
            SessionCommandDto::TurnLeft {
                actor_entity_id,
                expected_revision,
            } => Self::TurnLeft {
                actor_entity_id,
                expected_revision,
            },
            SessionCommandDto::TurnRight {
                actor_entity_id,
                expected_revision,
            } => Self::TurnRight {
                actor_entity_id,
                expected_revision,
            },
            SessionCommandDto::UseAction {
                actor_entity_id,
                expected_revision,
                action_id,
                target_entity_id,
            } => Self::UseAction {
                actor_entity_id,
                expected_revision,
                action_id,
                target_entity_id,
            },
        }
    }
}

impl PartyCommand {
    pub const fn actor_entity_id(&self) -> u64 {
        match self {
            Self::Step {
                actor_entity_id, ..
            }
            | Self::TurnLeft {
                actor_entity_id, ..
            }
            | Self::TurnRight {
                actor_entity_id, ..
            }
            | Self::UseAction {
                actor_entity_id, ..
            } => *actor_entity_id,
        }
    }

    pub const fn expected_revision(&self) -> u64 {
        match self {
            Self::Step {
                expected_revision, ..
            }
            | Self::TurnLeft {
                expected_revision, ..
            }
            | Self::TurnRight {
                expected_revision, ..
            }
            | Self::UseAction {
                expected_revision, ..
            } => *expected_revision,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActivationView {
    #[ts(type = "number")]
    pub entity_id: u64,
    pub actor_id: RoguelikeId,
    pub name: String,
    pub side: TurnSide,
    pub initiative: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PartyMemberStatusView {
    #[ts(type = "number")]
    pub entity_id: u64,
    pub actor_id: RoguelikeId,
    pub name: String,
    pub current_vitality: u16,
    pub maximum_vitality: u16,
    pub conscious: bool,
    pub carried_items: Vec<CarriedItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CarriedItemView {
    pub item_id: RoguelikeId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LegalActionView {
    pub action_id: RoguelikeId,
    pub name: String,
    #[ts(type = "Array<number>")]
    pub legal_target_entity_ids: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PartyDecisionView {
    #[ts(type = "number")]
    pub actor_entity_id: u64,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub legal_steps: Vec<RelativeStep>,
    pub can_turn: bool,
    pub actions: Vec<LegalActionView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum PartyTurnDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TurnReceipt {
    PartyMoved {
        #[ts(type = "number")]
        actor_entity_id: u64,
        step: RelativeStep,
    },
    PartyTurned {
        #[ts(type = "number")]
        actor_entity_id: u64,
        direction: PartyTurnDirection,
    },
    PartyAttacked {
        #[ts(type = "number")]
        actor_entity_id: u64,
        #[ts(type = "number")]
        target_entity_id: u64,
        action_id: RoguelikeId,
        d20: u8,
        ability_modifier: i16,
        attack_total: i16,
        defense: i16,
        hit: bool,
        damage_rolls: Vec<u16>,
        damage_bonus: i16,
        requested_damage: u16,
        applied_damage: u16,
    },
    OppositionAttacked {
        #[ts(type = "number")]
        actor_entity_id: u64,
        action_id: RoguelikeId,
        target: PartySquareTargetReceipt,
        d20: u8,
        ability_modifier: i16,
        attack_total: i16,
        defense: i16,
        hit: bool,
        damage_rolls: Vec<u16>,
        damage_bonus: i16,
        requested_damage: u16,
        applied_damage: u16,
    },
    OppositionMoved {
        #[ts(type = "number")]
        actor_entity_id: u64,
    },
    OppositionPassed {
        #[ts(type = "number")]
        actor_entity_id: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SessionView {
    pub schema_version: u32,
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub round: u64,
    pub outcome: SessionOutcome,
    pub current: Option<ActivationView>,
    pub order: Vec<ActivationView>,
    pub party: Vec<PartyMemberStatusView>,
    pub decision: Option<PartyDecisionView>,
    pub latest_receipts: Vec<TurnReceipt>,
    pub world: WorldView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionError {
    code: &'static str,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SessionErrorDto {
    pub code: String,
    pub detail: String,
}

impl From<&SessionError> for SessionErrorDto {
    fn from(value: &SessionError) -> Self {
        Self {
            code: value.code().to_owned(),
            detail: value.detail().to_owned(),
        }
    }
}

impl SessionError {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub const fn code(&self) -> &'static str {
        self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for SessionError {}
