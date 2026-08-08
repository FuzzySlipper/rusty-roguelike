use rusty_engine::gameplay_rules::{
    self, RuleDomainId, RulePackageId, RuleProvenance, RuleSource, RuleSourceId, RuleSubjectId,
    RuleVersion,
};

use super::*;

const STARTER_RULES_JSON: &str = include_str!("../../../../content/rules/starter.json");
const STARTER_SOURCE_PATH: &str = "rust/content/rules/starter.json";

pub fn starter_candidate() -> Result<RoguelikeRulesCandidate, serde_json::Error> {
    serde_json::from_str(STARTER_RULES_JSON)
}

pub fn starter_rule_package() -> Result<gameplay_rules::AdmittedRulePackage, RoguelikeCompileError>
{
    let candidate = starter_candidate()
        .map_err(|error| RoguelikeCompileError::InvalidPayload(error.to_string()))?;
    starter_rule_package_for_candidate(candidate)
}

fn starter_rule_package_for_candidate(
    candidate: RoguelikeRulesCandidate,
) -> Result<gameplay_rules::AdmittedRulePackage, RoguelikeCompileError> {
    let source_id = RuleSourceId::parse("starter-rules").map_err(RoguelikeCompileError::Package)?;
    let source = RuleSource::new(source_id.clone(), STARTER_SOURCE_PATH)
        .map_err(RoguelikeCompileError::Package)?;
    let mut provenance = Vec::new();
    for (kind, id) in candidate
        .abilities
        .iter()
        .map(|value| ("ability", &value.id))
        .chain(
            candidate
                .defenses
                .iter()
                .map(|value| ("defense", &value.id)),
        )
        .chain(
            candidate
                .damage_types
                .iter()
                .map(|value| ("damage-type", &value.id)),
        )
        .chain(candidate.actions.iter().map(|value| ("action", &value.id)))
        .chain(candidate.feats.iter().map(|value| ("feat", &value.id)))
        .chain(candidate.classes.iter().map(|value| ("class", &value.id)))
        .chain(candidate.items.iter().map(|value| ("item", &value.id)))
        .chain(candidate.actors.iter().map(|value| ("actor", &value.id)))
        .chain(std::iter::once(("party", &candidate.party.id)))
    {
        provenance.push(
            RuleProvenance::new(
                RuleSubjectId::parse(format!("{kind}:{id}"))
                    .map_err(RoguelikeCompileError::Package)?,
                source_id.clone(),
                None,
                None,
            )
            .map_err(RoguelikeCompileError::Package)?,
        );
    }
    admit_roguelike_candidate(
        RoguelikePackageEnvelope {
            domain: RuleDomainId::parse("rusty-roguelike")
                .map_err(RoguelikeCompileError::Package)?,
            package: RulePackageId::parse("starter").map_err(RoguelikeCompileError::Package)?,
            version: RuleVersion::new(1).map_err(RoguelikeCompileError::Package)?,
            dependencies: vec![],
            sources: vec![source],
            provenance,
        },
        candidate,
    )
    .map_err(RoguelikeCompileError::Package)
}

pub fn starter_ruleset() -> Result<RoguelikeRuleset, RoguelikeCompileError> {
    RoguelikeRuleset::compile(vec![starter_rule_package()?])
}

#[cfg(test)]
pub(crate) fn starter_ruleset_with_opposition(
    opposition_entity_ids: &[u64],
) -> Result<RoguelikeRuleset, RoguelikeCompileError> {
    let mut candidate = starter_candidate()
        .map_err(|error| RoguelikeCompileError::InvalidPayload(error.to_string()))?;
    candidate.actors.retain(|actor| {
        actor.side == ActorSideCandidate::Party || opposition_entity_ids.contains(&actor.entity_id)
    });
    RoguelikeRuleset::compile(vec![starter_rule_package_for_candidate(candidate)?])
}
