use core_ids::EntityId;
use entity_state::{EntityAuthoringService, EntityDefinition, EntityState};
use gameplay_mechanics::{
    CatalogVersion, IntrinsicSourceBinding, IntrinsicSourcesComponent, MechanicsScalar,
    OperationId, SourceInstanceId, StatService, StatValue, StatsComponent,
};

use super::*;

#[test]
fn starter_catalog_compiles_with_one_activation_actions_and_exact_party() {
    let rules = starter_ruleset().expect("starter rules compile");
    assert_eq!(rules.party().members.len(), 3);
    assert_eq!(rules.actors().len(), 8);
    assert!(rules
        .actions()
        .values()
        .all(|action| action.activation_cost == 1));
    assert!(matches!(
        rules.actions()[&RoguelikeId::parse("move").unwrap()].effect,
        ActionEffectDefinition::Movement { steps: 1 }
    ));
    assert_eq!(
        rules.actions()[&RoguelikeId::parse("aimed-shot").unwrap()].target,
        ActionTargetCandidate::HostileCell
    );
    assert_eq!(
        rules.actions()[&RoguelikeId::parse("rusty-blade").unwrap()].target,
        ActionTargetCandidate::HostilePartySquare
    );
    assert!(rules
        .actions()
        .values()
        .all(|action| !action.origin.source_path.is_empty()));
}

#[test]
fn strict_candidate_and_semantic_compiler_reject_unknown_or_multi_effect_actions() {
    let unknown = STARTER_JSON.replace(
        "\"schemaVersion\": 1,",
        "\"schemaVersion\": 1, \"unknown\": true,",
    );
    assert!(serde_json::from_str::<RoguelikeRulesCandidate>(&unknown).is_err());

    let mut candidate = starter_candidate().unwrap();
    candidate.actions[0].attack = candidate.actions[1].attack.clone();
    let package = package_for_test(candidate);
    let error = RoguelikeRuleset::compile(vec![package]).unwrap_err();
    assert!(format!("{error}").contains("exactly one movement or attack"));

    let mut candidate = starter_candidate().unwrap();
    candidate
        .actions
        .iter_mut()
        .find(|action| action.id.as_str() == "rusty-blade")
        .unwrap()
        .target = ActionTargetCandidate::HostileCell;
    let error = RoguelikeRuleset::compile(vec![package_for_test(candidate)]).unwrap_err();
    assert!(format!("{error}").contains("incompatible targets"));
}

#[test]
fn compiler_requires_exact_definition_provenance_and_valid_roll_policy() {
    let candidate = starter_candidate().unwrap();
    let package = package_for_test_omitting(candidate, Some("action:move"));
    let error = RoguelikeRuleset::compile(vec![package]).unwrap_err();
    assert!(format!("{error}").contains("no exact package provenance"));

    let mut candidate = starter_candidate().unwrap();
    candidate.roll_policy.kind = RollPolicyKindCandidate::Static;
    candidate.roll_policy.seed = Some(99);
    candidate.roll_policy.rolls = vec![StaticRollCandidate {
        d20: 20,
        damage: vec![6],
    }];
    let package = package_for_test(candidate);
    let error = RoguelikeRuleset::compile(vec![package]).unwrap_err();
    assert!(format!("{error}").contains("static policy"));
}

#[test]
fn registered_components_are_durable_and_reject_noncanonical_wire_data() {
    let registry = roguelike_component_registry().unwrap();
    let entity = EntityId::new(7);
    let mut state = EntityState::from_definitions_with_registry(
        registry,
        [EntityDefinition::new(entity, "actor")],
    )
    .unwrap();
    assert!(state
        .component_revision::<AbilityScoresComponent>(entity)
        .is_ok());

    let decoded = serde_json::from_value::<AbilityScoresComponent>(serde_json::json!({
        "scores": [
            {"id": "might", "score": 12},
            {"id": "might", "score": 14}
        ]
    }))
    .unwrap();
    assert!(AbilityScoresComponent::new(decoded.scores().to_vec()).is_err());
    let revision = state
        .component_revision::<AbilityScoresComponent>(entity)
        .unwrap();
    assert!(EntityAuthoringService
        .attach_component(&mut state, revision, entity, decoded)
        .is_err());
}

#[test]
fn feat_source_is_evaluated_by_the_named_engine_stat_service() {
    let rules = starter_ruleset().unwrap();
    let registry = roguelike_component_registry().unwrap();
    let entity = EntityId::new(101);
    let mut state = EntityState::from_definitions_with_registry(
        registry,
        [EntityDefinition::new(entity, "Brann")],
    )
    .unwrap();
    attach(
        &mut state,
        entity,
        StatsComponent::new(
            CatalogVersion::parse("rusty-roguelike.v1").unwrap(),
            vec![StatValue::new(
                defense_stat_id(&RoguelikeId::parse("armor").unwrap()),
                MechanicsScalar::new(10).unwrap(),
            )],
        )
        .unwrap(),
    );
    attach(
        &mut state,
        entity,
        IntrinsicSourcesComponent::new(
            CatalogVersion::parse("rusty-roguelike.v1").unwrap(),
            vec![IntrinsicSourceBinding::new(
                SourceInstanceId::parse("feat.hold-the-line").unwrap(),
                feat_source_id(&RoguelikeId::parse("hold-the-line").unwrap()),
            )],
        )
        .unwrap(),
    );
    let evaluation = StatService::evaluate(
        &state,
        rules.mechanics(),
        entity,
        &defense_stat_id(&RoguelikeId::parse("armor").unwrap()),
        &OperationId::parse("test.evaluate-feat").unwrap(),
        &[],
    )
    .unwrap();
    assert_eq!(evaluation.base.get(), 10);
    assert_eq!(evaluation.value.get(), 11);
    assert_eq!(evaluation.decisions.len(), 1);
}

fn attach<T: entity_state::EntityComponent>(
    state: &mut EntityState,
    entity: EntityId,
    component: T,
) {
    let revision = state.component_revision::<T>(entity).unwrap();
    EntityAuthoringService
        .attach_component(state, revision, entity, component)
        .unwrap();
}

fn package_for_test(candidate: RoguelikeRulesCandidate) -> gameplay_rules::AdmittedRulePackage {
    package_for_test_omitting(candidate, None)
}

fn package_for_test_omitting(
    candidate: RoguelikeRulesCandidate,
    omitted_subject: Option<&str>,
) -> gameplay_rules::AdmittedRulePackage {
    let source_id = gameplay_rules::RuleSourceId::parse("test").unwrap();
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
        if omitted_subject.is_some_and(|subject| subject == format!("{kind}:{id}")) {
            continue;
        }
        provenance.push(
            gameplay_rules::RuleProvenance::new(
                gameplay_rules::RuleSubjectId::parse(format!("{kind}:{id}")).unwrap(),
                source_id.clone(),
                None,
                None,
            )
            .unwrap(),
        );
    }
    admit_roguelike_candidate(
        RoguelikePackageEnvelope {
            domain: gameplay_rules::RuleDomainId::parse("rusty-roguelike").unwrap(),
            package: gameplay_rules::RulePackageId::parse("test").unwrap(),
            version: gameplay_rules::RuleVersion::new(1).unwrap(),
            dependencies: vec![],
            sources: vec![gameplay_rules::RuleSource::new(source_id, "test.json").unwrap()],
            provenance,
        },
        candidate,
    )
    .unwrap()
}

const STARTER_JSON: &str = include_str!("../../../../content/rules/starter.json");
