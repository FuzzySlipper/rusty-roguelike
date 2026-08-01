use std::collections::BTreeMap;

use gameplay_mechanics::MechanicsCatalog;
use gameplay_rules::RulePackageIdentity;

use super::{
    ActionTargetCandidate, ActorSideCandidate, EquipmentSlotCandidate, RoguelikeId,
    RollPolicyCandidate,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionOrigin {
    pub package: RulePackageIdentity,
    pub source_path: String,
    pub line: Option<u64>,
    pub column: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityDefinition {
    pub id: RoguelikeId,
    pub minimum: i16,
    pub maximum: i16,
    pub origin: DefinitionOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefenseDefinition {
    pub id: RoguelikeId,
    pub base: i16,
    pub abilities: Vec<RoguelikeId>,
    pub origin: DefinitionOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageDefinition {
    pub kind: RoguelikeId,
    pub dice: u8,
    pub sides: u16,
    pub bonus: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionEffectDefinition {
    Movement {
        steps: u8,
    },
    Attack {
        ability: RoguelikeId,
        defense: RoguelikeId,
        damage: DamageDefinition,
        range: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionDefinition {
    pub id: RoguelikeId,
    pub name: String,
    pub tags: Vec<RoguelikeId>,
    pub target: ActionTargetCandidate,
    pub effect: ActionEffectDefinition,
    pub activation_cost: u8,
    pub origin: DefinitionOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatModifierDefinition {
    pub defense: RoguelikeId,
    pub amount: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatDefinition {
    pub id: RoguelikeId,
    pub name: String,
    pub description: String,
    pub modifiers: Vec<StatModifierDefinition>,
    pub origin: DefinitionOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassLevelDefinition {
    pub level: u8,
    pub actions: Vec<RoguelikeId>,
    pub feats: Vec<RoguelikeId>,
    pub action_slot_increase: u8,
    pub feat_slot_increase: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDefinition {
    pub id: RoguelikeId,
    pub name: String,
    pub levels: Vec<ClassLevelDefinition>,
    pub origin: DefinitionOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDefinition {
    pub id: RoguelikeId,
    pub name: String,
    pub slot: Option<EquipmentSlotCandidate>,
    pub grants_action: Option<RoguelikeId>,
    pub modifiers: Vec<StatModifierDefinition>,
    pub origin: DefinitionOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityScoreDefinition {
    pub ability: RoguelikeId,
    pub score: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorDefinition {
    pub id: RoguelikeId,
    pub entity_id: u64,
    pub name: String,
    pub title: String,
    pub side: ActorSideCandidate,
    pub level: u8,
    pub experience: u32,
    pub vitality: u16,
    pub inventory_capacity: u8,
    pub class: RoguelikeId,
    pub class_level: u8,
    pub abilities: Vec<AbilityScoreDefinition>,
    pub actions: Vec<RoguelikeId>,
    pub feats: Vec<RoguelikeId>,
    pub items: Vec<RoguelikeId>,
    pub origin: DefinitionOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyDefinition {
    pub id: RoguelikeId,
    pub entity_id: u64,
    pub members: Vec<RoguelikeId>,
    pub origin: DefinitionOrigin,
}

#[derive(Debug, Clone)]
pub struct RoguelikeRuleset {
    pub(super) fingerprint: String,
    pub(super) roll_policy: RollPolicyCandidate,
    pub(super) abilities: BTreeMap<RoguelikeId, AbilityDefinition>,
    pub(super) defenses: BTreeMap<RoguelikeId, DefenseDefinition>,
    pub(super) damage_types: BTreeMap<RoguelikeId, DefinitionOrigin>,
    pub(super) actions: BTreeMap<RoguelikeId, ActionDefinition>,
    pub(super) feats: BTreeMap<RoguelikeId, FeatDefinition>,
    pub(super) classes: BTreeMap<RoguelikeId, ClassDefinition>,
    pub(super) items: BTreeMap<RoguelikeId, ItemDefinition>,
    pub(super) actors: BTreeMap<RoguelikeId, ActorDefinition>,
    pub(super) party: PartyDefinition,
    pub(super) mechanics: MechanicsCatalog,
}

impl RoguelikeRuleset {
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub const fn roll_policy(&self) -> &RollPolicyCandidate {
        &self.roll_policy
    }

    pub const fn abilities(&self) -> &BTreeMap<RoguelikeId, AbilityDefinition> {
        &self.abilities
    }

    pub const fn defenses(&self) -> &BTreeMap<RoguelikeId, DefenseDefinition> {
        &self.defenses
    }

    pub const fn damage_types(&self) -> &BTreeMap<RoguelikeId, DefinitionOrigin> {
        &self.damage_types
    }

    pub const fn actions(&self) -> &BTreeMap<RoguelikeId, ActionDefinition> {
        &self.actions
    }

    pub const fn feats(&self) -> &BTreeMap<RoguelikeId, FeatDefinition> {
        &self.feats
    }

    pub const fn classes(&self) -> &BTreeMap<RoguelikeId, ClassDefinition> {
        &self.classes
    }

    pub const fn items(&self) -> &BTreeMap<RoguelikeId, ItemDefinition> {
        &self.items
    }

    pub const fn actors(&self) -> &BTreeMap<RoguelikeId, ActorDefinition> {
        &self.actors
    }

    pub const fn party(&self) -> &PartyDefinition {
        &self.party
    }

    pub const fn mechanics(&self) -> &MechanicsCatalog {
        &self.mechanics
    }
}
