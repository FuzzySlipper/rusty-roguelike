use gameplay_rules::{
    admit_rule_package, AdmittedRulePackage, RuleDomainId, RulePackageCandidate,
    RulePackageDependency, RulePackageError, RulePackageId, RuleProvenance, RuleSource,
    RuleVersion,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{
    RoguelikeId, MAX_ROGUELIKE_ACTION_TAGS, MAX_ROGUELIKE_AUTHORED_TEXT_BYTES,
    MAX_ROGUELIKE_DAMAGE_DICE, MAX_ROGUELIKE_DAMAGE_DIE_SIDES, MAX_ROGUELIKE_DEFINITIONS_PER_KIND,
    MAX_ROGUELIKE_ID_BYTES, MAX_ROGUELIKE_STATIC_ROLLS,
};

pub const ROGUELIKE_CANDIDATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RoguelikeRulesCandidate {
    pub schema_version: u32,
    pub roll_policy: RollPolicyCandidate,
    pub abilities: Vec<AbilityCandidate>,
    pub defenses: Vec<DefenseCandidate>,
    pub damage_types: Vec<DamageTypeCandidate>,
    pub actions: Vec<ActionCandidate>,
    pub feats: Vec<FeatCandidate>,
    pub classes: Vec<ClassCandidate>,
    pub items: Vec<ItemCandidate>,
    pub actors: Vec<ActorCandidate>,
    pub party: PartyCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum RollPolicyKindCandidate {
    Seeded,
    Static,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RollPolicyCandidate {
    pub kind: RollPolicyKindCandidate,
    #[ts(type = "number")]
    pub seed: Option<u64>,
    pub rolls: Vec<StaticRollCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct StaticRollCandidate {
    pub d20: u8,
    pub damage: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AbilityCandidate {
    pub id: RoguelikeId,
    pub minimum: i16,
    pub maximum: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DefenseCandidate {
    pub id: RoguelikeId,
    pub base: i16,
    pub abilities: Vec<RoguelikeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DamageTypeCandidate {
    pub id: RoguelikeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DamageCandidate {
    pub kind: RoguelikeId,
    pub dice: u8,
    pub sides: u16,
    pub bonus: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ActionTargetCandidate {
    SelfOnly,
    HostileCell,
    HostilePartySquare,
    AllyCell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MovementCandidate {
    pub steps: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AttackCandidate {
    pub ability: RoguelikeId,
    pub defense: RoguelikeId,
    pub damage: DamageCandidate,
    pub range: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActionCandidate {
    pub id: RoguelikeId,
    pub name: String,
    pub tags: Vec<RoguelikeId>,
    pub target: ActionTargetCandidate,
    pub movement: Option<MovementCandidate>,
    pub attack: Option<AttackCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct StatModifierCandidate {
    pub defense: RoguelikeId,
    pub amount: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FeatCandidate {
    pub id: RoguelikeId,
    pub name: String,
    pub description: String,
    pub modifiers: Vec<StatModifierCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ClassLevelCandidate {
    pub level: u8,
    pub actions: Vec<RoguelikeId>,
    pub feats: Vec<RoguelikeId>,
    pub action_slot_increase: u8,
    pub feat_slot_increase: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ClassCandidate {
    pub id: RoguelikeId,
    pub name: String,
    pub levels: Vec<ClassLevelCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum EquipmentSlotCandidate {
    Body,
    Weapon,
    Focus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ItemCandidate {
    pub id: RoguelikeId,
    pub name: String,
    pub slot: Option<EquipmentSlotCandidate>,
    pub grants_action: Option<RoguelikeId>,
    pub modifiers: Vec<StatModifierCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ActorSideCandidate {
    Party,
    Opposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AbilityScoreCandidate {
    pub ability: RoguelikeId,
    pub score: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActorCandidate {
    pub id: RoguelikeId,
    #[ts(type = "number")]
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
    pub abilities: Vec<AbilityScoreCandidate>,
    pub actions: Vec<RoguelikeId>,
    pub feats: Vec<RoguelikeId>,
    pub items: Vec<RoguelikeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PartyCandidate {
    pub id: RoguelikeId,
    #[ts(type = "number")]
    pub entity_id: u64,
    pub members: Vec<RoguelikeId>,
}

#[derive(Debug, Clone)]
pub struct RoguelikePackageEnvelope {
    pub domain: RuleDomainId,
    pub package: RulePackageId,
    pub version: RuleVersion,
    pub dependencies: Vec<RulePackageDependency>,
    pub sources: Vec<RuleSource>,
    pub provenance: Vec<RuleProvenance>,
}

pub fn admit_roguelike_candidate(
    envelope: RoguelikePackageEnvelope,
    candidate: RoguelikeRulesCandidate,
) -> Result<AdmittedRulePackage, RulePackageError> {
    let payload =
        serde_json::to_value(candidate).map_err(|error| RulePackageError::MalformedJson {
            path: "$/payload".to_owned(),
            offset: 0,
            reason: error.to_string(),
        })?;
    admit_rule_package(RulePackageCandidate::new(
        envelope.domain,
        envelope.package,
        envelope.version,
        envelope.dependencies,
        envelope.sources,
        envelope.provenance,
        payload,
    ))
}

pub fn generated_candidate_typescript() -> String {
    let declarations = [
        RoguelikeId::decl(),
        RoguelikeRulesCandidate::decl(),
        RollPolicyKindCandidate::decl(),
        RollPolicyCandidate::decl(),
        StaticRollCandidate::decl(),
        AbilityCandidate::decl(),
        DefenseCandidate::decl(),
        DamageTypeCandidate::decl(),
        DamageCandidate::decl(),
        ActionTargetCandidate::decl(),
        MovementCandidate::decl(),
        AttackCandidate::decl(),
        ActionCandidate::decl(),
        StatModifierCandidate::decl(),
        FeatCandidate::decl(),
        ClassLevelCandidate::decl(),
        ClassCandidate::decl(),
        EquipmentSlotCandidate::decl(),
        ItemCandidate::decl(),
        ActorSideCandidate::decl(),
        AbilityScoreCandidate::decl(),
        ActorCandidate::decl(),
        PartyCandidate::decl(),
    ]
    .into_iter()
    .map(|declaration| format!("export {declaration}"))
    .collect::<Vec<_>>()
    .join("\n\n");

    format!(
        "export const ROGUELIKE_CANDIDATE_SCHEMA_VERSION = {ROGUELIKE_CANDIDATE_SCHEMA_VERSION} as const;\n\
export const ROGUELIKE_ID_PATTERN = {pattern:?} as const;\n\
export const ROGUELIKE_LIMITS = Object.freeze({{\n\
  maxIdBytes: {MAX_ROGUELIKE_ID_BYTES},\n\
  maxDefinitionsPerKind: {MAX_ROGUELIKE_DEFINITIONS_PER_KIND},\n\
  maxAuthoredTextBytes: {MAX_ROGUELIKE_AUTHORED_TEXT_BYTES},\n\
  maxActionTags: {MAX_ROGUELIKE_ACTION_TAGS},\n\
  maxDamageDice: {MAX_ROGUELIKE_DAMAGE_DICE},\n\
  maxDamageDieSides: {MAX_ROGUELIKE_DAMAGE_DIE_SIDES},\n\
  maxStaticRolls: {MAX_ROGUELIKE_STATIC_ROLLS},\n\
}} as const);\n\n\
{declarations}\n",
        pattern = super::ROGUELIKE_ID_PATTERN,
    )
}
