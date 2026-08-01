use serde::{Deserialize, Serialize};

use crate::{RelativeStep, RoguelikeId, WorldView};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TurnSide {
    Party,
    Opposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionOutcome {
    Ongoing,
    Victory,
    Defeat,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActivationView {
    pub entity_id: u64,
    pub actor_id: RoguelikeId,
    pub name: String,
    pub side: TurnSide,
    pub initiative: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TurnReceipt {
    PartyMoved {
        actor_entity_id: u64,
    },
    PartyTurned {
        actor_entity_id: u64,
    },
    PartyAttacked {
        actor_entity_id: u64,
        target_entity_id: u64,
        action_id: RoguelikeId,
        d20: u8,
        attack_total: i16,
        defense: i16,
        hit: bool,
        damage: u16,
    },
    OppositionMoved {
        actor_entity_id: u64,
    },
    OppositionPassed {
        actor_entity_id: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SessionView {
    pub revision: u64,
    pub round: u64,
    pub outcome: SessionOutcome,
    pub current: Option<ActivationView>,
    pub order: Vec<ActivationView>,
    pub latest_receipts: Vec<TurnReceipt>,
    pub world: WorldView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionError {
    code: &'static str,
    detail: String,
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
