use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{RelativeStep, RoguelikeId, WorldView};

pub const SESSION_VIEW_SCHEMA_VERSION: u32 = 6;
pub const MAX_SESSION_ACTIVATIONS: usize = 64;
pub const MAX_SESSION_RECEIPTS: usize = 256;
pub const MAX_SESSION_LOG_ENTRIES: usize = 4_096;

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
pub enum SessionPhase {
    Preparation,
    Expedition,
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
pub enum SessionCommand {
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
    Wait {
        actor_entity_id: u64,
        expected_revision: u64,
    },
    UseAction {
        actor_entity_id: u64,
        expected_revision: u64,
        action_id: RoguelikeId,
        target_entity_id: u64,
    },
    MoveLoadoutItem {
        expected_revision: u64,
        item_entity_id: u64,
        from_owner_entity_id: u64,
        to_owner_entity_id: u64,
        destination_slot_id: Option<String>,
    },
    BeginExpedition {
        expected_revision: u64,
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
    Wait {
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
    MoveLoadoutItem {
        #[ts(type = "number")]
        expected_revision: u64,
        #[ts(type = "number")]
        item_entity_id: u64,
        #[ts(type = "number")]
        from_owner_entity_id: u64,
        #[ts(type = "number")]
        to_owner_entity_id: u64,
        destination_slot_id: Option<String>,
    },
    BeginExpedition {
        #[ts(type = "number")]
        expected_revision: u64,
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
            Wait {
                actor_entity_id: u64,
                expected_revision: u64,
            },
            UseAction {
                actor_entity_id: u64,
                expected_revision: u64,
                action_id: RoguelikeId,
                target_entity_id: u64,
            },
            MoveLoadoutItem {
                expected_revision: u64,
                item_entity_id: u64,
                from_owner_entity_id: u64,
                to_owner_entity_id: u64,
                destination_slot_id: Option<String>,
            },
            BeginExpedition {
                expected_revision: u64,
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
            StrictSessionCommandDto::Wait {
                actor_entity_id,
                expected_revision,
            } => Self::Wait {
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
            StrictSessionCommandDto::MoveLoadoutItem {
                expected_revision,
                item_entity_id,
                from_owner_entity_id,
                to_owner_entity_id,
                destination_slot_id,
            } => Self::MoveLoadoutItem {
                expected_revision,
                item_entity_id,
                from_owner_entity_id,
                to_owner_entity_id,
                destination_slot_id,
            },
            StrictSessionCommandDto::BeginExpedition { expected_revision } => {
                Self::BeginExpedition { expected_revision }
            }
        })
    }
}

impl From<SessionCommandDto> for SessionCommand {
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
            SessionCommandDto::Wait {
                actor_entity_id,
                expected_revision,
            } => Self::Wait {
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
            SessionCommandDto::MoveLoadoutItem {
                expected_revision,
                item_entity_id,
                from_owner_entity_id,
                to_owner_entity_id,
                destination_slot_id,
            } => Self::MoveLoadoutItem {
                expected_revision,
                item_entity_id,
                from_owner_entity_id,
                to_owner_entity_id,
                destination_slot_id,
            },
            SessionCommandDto::BeginExpedition { expected_revision } => {
                Self::BeginExpedition { expected_revision }
            }
        }
    }
}

impl SessionCommand {
    pub const fn actor_entity_id(&self) -> Option<u64> {
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
            | Self::Wait {
                actor_entity_id, ..
            }
            | Self::UseAction {
                actor_entity_id, ..
            } => Some(*actor_entity_id),
            Self::MoveLoadoutItem { .. } | Self::BeginExpedition { .. } => None,
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
            | Self::Wait {
                expected_revision, ..
            }
            | Self::UseAction {
                expected_revision, ..
            }
            | Self::MoveLoadoutItem {
                expected_revision, ..
            }
            | Self::BeginExpedition { expected_revision } => *expected_revision,
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
    pub title: String,
    pub level: u8,
    pub experience: u32,
    pub class_id: RoguelikeId,
    pub class_name: String,
    pub class_level: u8,
    pub current_vitality: u16,
    pub maximum_vitality: u16,
    pub conscious: bool,
    pub abilities: Vec<AbilityReadoutView>,
    pub defenses: Vec<DefenseReadoutView>,
    pub feats: Vec<FeatReadoutView>,
    pub actions: Vec<CharacterActionView>,
    pub loadout: LoadoutView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AbilityReadoutView {
    pub ability_id: RoguelikeId,
    pub score: i16,
    pub modifier: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DefenseReadoutView {
    pub defense_id: RoguelikeId,
    pub value: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FeatReadoutView {
    pub feat_id: RoguelikeId,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CharacterActionView {
    pub action_id: RoguelikeId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LoadoutItemView {
    #[ts(type = "number")]
    pub entity_id: u64,
    pub item_id: RoguelikeId,
    pub name: String,
    pub equipment_slot_id: Option<String>,
    pub equipped_slot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LoadoutCapacityView {
    #[ts(type = "number")]
    pub used: u64,
    #[ts(type = "number")]
    pub maximum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EquipmentSlotView {
    pub slot_id: String,
    pub label: String,
    pub equipped: Option<LoadoutItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LoadoutView {
    #[ts(type = "number")]
    pub owner_entity_id: u64,
    pub inventory_slots: Vec<Option<LoadoutItemView>>,
    pub equipment_slots: Vec<EquipmentSlotView>,
    pub capacity: LoadoutCapacityView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PreparationView {
    pub stash: LoadoutView,
    pub ready: bool,
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
    pub can_wait: bool,
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
    LoadoutMoved {
        #[ts(type = "number")]
        item_entity_id: u64,
        #[ts(type = "number")]
        from_owner_entity_id: u64,
        #[ts(type = "number")]
        to_owner_entity_id: u64,
        destination_slot_id: Option<String>,
    },
    ExpeditionBegan,
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
    PartyWaited {
        #[ts(type = "number")]
        actor_entity_id: u64,
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
pub struct SessionLogEntry {
    #[ts(type = "number")]
    pub id: u64,
    #[ts(type = "number")]
    pub revision: u64,
    pub receipt: TurnReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SessionView {
    pub schema_version: u32,
    #[ts(type = "number")]
    pub revision: u64,
    pub phase: SessionPhase,
    #[ts(type = "number")]
    pub round: u64,
    pub outcome: SessionOutcome,
    pub current: Option<ActivationView>,
    pub order: Vec<ActivationView>,
    pub party: Vec<PartyMemberStatusView>,
    pub preparation: Option<PreparationView>,
    pub decision: Option<PartyDecisionView>,
    pub latest_receipts: Vec<TurnReceipt>,
    pub log: Vec<SessionLogEntry>,
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
