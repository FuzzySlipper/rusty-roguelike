use rusty_engine::gameplay_rules::{self, decode_rule_package};

#[cfg(test)]
use rusty_engine::gameplay_rules::{
    RuleDomainId, RulePackageId, RuleProvenance, RuleSource, RuleSourceId, RuleSubjectId,
    RuleVersion,
};

use super::*;

const STARTER_DOMAIN: &str = "rusty-roguelike";
const STARTER_PACKAGE: &str = "starter";
const STARTER_VERSION: u64 = 1;

/// Committed package artifact materialized by the gameplay authoring
/// workspace (`gameplay/scripts/materialize.mjs`). Its payload is
/// semantically identical to the retired `rust/content/rules/starter.json`.
const STARTER_PACKAGE_JSON: &str =
    include_str!("../../../../../data/gameplay/rusty-roguelike-starter.package.json");

/// Source records carried by the artifact: section id -> repo-relative
/// TypeScript catalog path, as recorded by the materializer. Used by the
/// opposition-subset test helper to rebuild a consistent envelope.
#[cfg(test)]
const STARTER_SOURCES: &[(&str, &str)] = &[
    ("abilities", "gameplay/src/catalogs/abilities.ts"),
    ("defenses", "gameplay/src/catalogs/defenses.ts"),
    ("damageTypes", "gameplay/src/catalogs/damageTypes.ts"),
    ("actions", "gameplay/src/catalogs/actions.ts"),
    ("feats", "gameplay/src/catalogs/feats.ts"),
    ("classes", "gameplay/src/catalogs/classes.ts"),
    ("items", "gameplay/src/catalogs/items.ts"),
    ("actors", "gameplay/src/catalogs/actors.ts"),
    ("party", "gameplay/src/catalogs/party.ts"),
];

/// Decodes the committed artifact into an admitted Engine rules package.
/// The artifact is the full gameplay-rules envelope (not just the payload);
/// provenance and source paths are carried by the envelope itself.
pub fn starter_rule_package() -> Result<gameplay_rules::AdmittedRulePackage, RoguelikeCompileError>
{
    let package = decode_rule_package(STARTER_PACKAGE_JSON.as_bytes())
        .map_err(RoguelikeCompileError::Package)?;
    let identity = package.identity();
    if identity.domain().as_str() != STARTER_DOMAIN
        || identity.package().as_str() != STARTER_PACKAGE
        || identity.version().get() != STARTER_VERSION
    {
        return Err(RoguelikeCompileError::InvalidPayload(format!(
            "materialized starter package identity mismatch: expected {STARTER_DOMAIN}/{STARTER_PACKAGE}@{STARTER_VERSION}, got {identity}"
        )));
    }
    Ok(package)
}

/// The starter candidate decoded from the materialized artifact payload.
pub fn starter_candidate() -> Result<RoguelikeRulesCandidate, RoguelikeCompileError> {
    serde_json::from_value(starter_rule_package()?.payload().clone())
        .map_err(|error| RoguelikeCompileError::InvalidPayload(error.to_string()))
}

pub fn starter_ruleset() -> Result<RoguelikeRuleset, RoguelikeCompileError> {
    RoguelikeRuleset::compile(vec![starter_rule_package()?])
}

/// Rebuilds a starter package envelope around a (possibly reduced) candidate
/// with the TypeScript catalog sources and provenance computed by subject.
/// Used by the opposition-subset test helper; the full artifact path decodes
/// the envelope directly instead.
#[cfg(test)]
fn starter_rule_package_for_candidate(
    candidate: RoguelikeRulesCandidate,
) -> Result<gameplay_rules::AdmittedRulePackage, RoguelikeCompileError> {
    let sources = STARTER_SOURCES
        .iter()
        .map(|(source_id, path)| {
            let source_id =
                RuleSourceId::parse(*source_id).map_err(RoguelikeCompileError::Package)?;
            RuleSource::new(source_id, *path).map_err(RoguelikeCompileError::Package)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut provenance = Vec::new();
    for (kind, section, id) in candidate
        .abilities
        .iter()
        .map(|value| ("ability", "abilities", &value.id))
        .chain(
            candidate
                .defenses
                .iter()
                .map(|value| ("defense", "defenses", &value.id)),
        )
        .chain(
            candidate
                .damage_types
                .iter()
                .map(|value| ("damage-type", "damageTypes", &value.id)),
        )
        .chain(
            candidate
                .actions
                .iter()
                .map(|value| ("action", "actions", &value.id)),
        )
        .chain(
            candidate
                .feats
                .iter()
                .map(|value| ("feat", "feats", &value.id)),
        )
        .chain(
            candidate
                .classes
                .iter()
                .map(|value| ("class", "classes", &value.id)),
        )
        .chain(
            candidate
                .items
                .iter()
                .map(|value| ("item", "items", &value.id)),
        )
        .chain(
            candidate
                .actors
                .iter()
                .map(|value| ("actor", "actors", &value.id)),
        )
        .chain(std::iter::once(("party", "party", &candidate.party.id)))
    {
        provenance.push(
            RuleProvenance::new(
                RuleSubjectId::parse(format!("{kind}:{id}"))
                    .map_err(RoguelikeCompileError::Package)?,
                RuleSourceId::parse(section).map_err(RoguelikeCompileError::Package)?,
                None,
                None,
            )
            .map_err(RoguelikeCompileError::Package)?,
        );
    }
    admit_roguelike_candidate(
        RoguelikePackageEnvelope {
            domain: RuleDomainId::parse(STARTER_DOMAIN).map_err(RoguelikeCompileError::Package)?,
            package: RulePackageId::parse(STARTER_PACKAGE)
                .map_err(RoguelikeCompileError::Package)?,
            version: RuleVersion::new(STARTER_VERSION).map_err(RoguelikeCompileError::Package)?,
            dependencies: vec![],
            sources,
            provenance,
        },
        candidate,
    )
    .map_err(RoguelikeCompileError::Package)
}

#[cfg(test)]
pub(crate) fn starter_ruleset_with_opposition(
    opposition_entity_ids: &[u64],
) -> Result<RoguelikeRuleset, RoguelikeCompileError> {
    let mut candidate = starter_candidate()?;
    candidate.actors.retain(|actor| {
        actor.side == ActorSideCandidate::Party || opposition_entity_ids.contains(&actor.entity_id)
    });
    RoguelikeRuleset::compile(vec![starter_rule_package_for_candidate(candidate)?])
}
