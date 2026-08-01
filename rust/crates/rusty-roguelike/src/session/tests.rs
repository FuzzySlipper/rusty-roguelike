use crate::{
    admit_roguelike_candidate, generate_authored_floor, starter_candidate, starter_ruleset,
    EnemyParticipation, EnemyWorldComponent, FloorBounds, FloorCell, FloorFeature,
    FloorFeatureKind, PartyCommand, RelativeStep, RoguelikePackageEnvelope,
    RoguelikeRulesCandidate, RoguelikeRuleset, RollPolicyCandidate, RollPolicyKindCandidate,
    StaticRollCandidate, TurnReceipt, TurnSide, WorldState,
};

use super::GameSession;

const SEED: u64 = 5_201;

#[test]
fn initiative_order_and_single_party_action_settle_to_the_next_decision() {
    let world = WorldState::new(open_arena(), starter_ruleset().unwrap()).unwrap();
    let before = world.durable_state().unwrap().party.position();
    let mut session = GameSession::new(world).unwrap();
    let initial = session.view().unwrap();

    assert_eq!(
        initial
            .order
            .iter()
            .map(|slot| (slot.entity_id, slot.side))
            .collect::<Vec<_>>(),
        vec![
            (102, TurnSide::Party),
            (103, TurnSide::Party),
            (202, TurnSide::Opposition),
            (101, TurnSide::Party),
        ]
    );
    assert_eq!(initial.current.as_ref().unwrap().entity_id, 102);

    let next = session
        .command(PartyCommand::Step {
            actor_entity_id: 102,
            expected_revision: 0,
            step: RelativeStep::Forward,
        })
        .unwrap();
    assert_eq!(next.revision, 1);
    assert_eq!(next.current.as_ref().unwrap().entity_id, 103);
    assert_eq!(
        session.world().durable_state().unwrap().party.position().y,
        before.y - 1
    );
    assert!(matches!(
        next.latest_receipts.first(),
        Some(TurnReceipt::PartyMoved {
            actor_entity_id: 102
        })
    ));
    let after_mira = turn_right(&mut session);
    assert_eq!(after_mira.current.as_ref().unwrap().entity_id, 101);
    assert!(after_mira.latest_receipts.iter().any(|receipt| matches!(
        receipt,
        TurnReceipt::OppositionMoved {
            actor_entity_id: 202
        } | TurnReceipt::OppositionPassed {
            actor_entity_id: 202
        }
    )));
}

#[test]
fn adjacent_no_legal_opposition_passes_and_newly_seen_actor_joins_next_round() {
    let floor = open_arena();
    let rules = starter_ruleset().unwrap();
    let seeded = WorldState::new(floor.clone(), rules).unwrap();
    let mut durable = seeded.durable_state().unwrap();
    durable.enemies[0].world = EnemyWorldComponent::new(
        floor.floor_id.clone(),
        crate::WorldCell { x: 2, y: 1 },
        EnemyParticipation::Participating,
    )
    .unwrap();
    durable.enemies[1].world = EnemyWorldComponent::new(
        floor.floor_id.clone(),
        crate::WorldCell { x: 2, y: 4 },
        EnemyParticipation::Dormant,
    )
    .unwrap();
    let world = WorldState::restore(floor, starter_ruleset().unwrap(), durable).unwrap();
    let mut session = GameSession::new(world).unwrap();
    assert!(!session
        .view()
        .unwrap()
        .order
        .iter()
        .any(|slot| slot.entity_id == 202));

    let after_kestrel = turn_right(&mut session);
    assert_eq!(after_kestrel.current.as_ref().unwrap().entity_id, 103);
    assert!(after_kestrel.latest_receipts.iter().any(|receipt| matches!(
        receipt,
        TurnReceipt::OppositionPassed {
            actor_entity_id: 201
        }
    )));
    let after_mira = turn_right(&mut session);
    assert_eq!(after_mira.current.as_ref().unwrap().entity_id, 101);
    assert!(after_mira
        .world
        .visible_actors
        .iter()
        .any(|actor| actor.entity_id == 202));
    let next_round = turn_right(&mut session);
    assert_eq!(next_round.round, 2);
    assert!(next_round.order.iter().any(|slot| slot.entity_id == 202));
}

#[test]
fn selected_attack_consumes_one_activation_and_failed_static_roll_is_atomic() {
    let rules = static_rules(vec![StaticRollCandidate {
        d20: 20,
        damage: vec![6, 6],
    }]);
    let world = WorldState::new(open_arena(), rules).unwrap();
    let mut session = GameSession::new(world).unwrap();
    let before = session.view().unwrap();
    let error = session
        .command(PartyCommand::UseAction {
            actor_entity_id: 102,
            expected_revision: 0,
            action_id: crate::RoguelikeId::parse("aimed-shot").unwrap(),
            target_entity_id: 202,
        })
        .expect_err("two damage dice cannot satisfy a one-die action");
    assert_eq!(error.code(), "session_static_roll_incompatible");
    assert_eq!(session.view().unwrap(), before);

    let mut session =
        GameSession::new(WorldState::new(open_arena(), starter_ruleset().unwrap()).unwrap())
            .unwrap();
    let resolved = session
        .command(PartyCommand::UseAction {
            actor_entity_id: 102,
            expected_revision: 0,
            action_id: crate::RoguelikeId::parse("aimed-shot").unwrap(),
            target_entity_id: 202,
        })
        .unwrap();
    assert_eq!(resolved.current.as_ref().unwrap().entity_id, 103);
    assert!(matches!(
        resolved.latest_receipts.first(),
        Some(TurnReceipt::PartyAttacked {
            actor_entity_id: 102,
            target_entity_id: 202,
            ..
        })
    ));

    let stable = session.view().unwrap();
    let stale = session
        .command(PartyCommand::TurnLeft {
            actor_entity_id: 103,
            expected_revision: 0,
        })
        .expect_err("stale revision must reject");
    assert_eq!(stale.code(), "session_revision_stale");
    assert_eq!(session.view().unwrap(), stable);
}

#[test]
fn defeated_participant_leaves_the_live_order_without_blocking_the_round() {
    let rolls = vec![
        StaticRollCandidate {
            d20: 20,
            damage: vec![8],
        },
        StaticRollCandidate {
            d20: 20,
            damage: vec![8],
        },
    ];
    let rules = static_rules_single_enemy(rolls.clone());
    let floor = open_arena();
    let seeded = WorldState::new(floor.clone(), rules).unwrap();
    let mut durable = seeded.durable_state().unwrap();
    durable.enemies[0].world = EnemyWorldComponent::new(
        floor.floor_id.clone(),
        crate::WorldCell { x: 2, y: 1 },
        EnemyParticipation::Participating,
    )
    .unwrap();
    let mut session = GameSession::new(
        WorldState::restore(floor, static_rules_single_enemy(rolls), durable).unwrap(),
    )
    .unwrap();
    let attack = crate::RoguelikeId::parse("aimed-shot").unwrap();
    session
        .command(PartyCommand::UseAction {
            actor_entity_id: 102,
            expected_revision: 0,
            action_id: attack.clone(),
            target_entity_id: 202,
        })
        .unwrap();
    turn_right(&mut session);
    turn_left(&mut session);
    let current = session.view().unwrap();
    assert_eq!(current.round, 2);
    assert_eq!(current.current.as_ref().unwrap().entity_id, 102);
    let after_defeat = session
        .command(PartyCommand::UseAction {
            actor_entity_id: 102,
            expected_revision: current.revision,
            action_id: attack,
            target_entity_id: 202,
        })
        .unwrap();
    assert!(!after_defeat.order.iter().any(|slot| slot.entity_id == 202));
    assert_eq!(after_defeat.outcome, crate::SessionOutcome::Victory);
    assert!(after_defeat.current.is_none());
}

fn turn_right(session: &mut GameSession) -> crate::SessionView {
    let view = session.view().unwrap();
    session
        .command(PartyCommand::TurnRight {
            actor_entity_id: view.current.unwrap().entity_id,
            expected_revision: view.revision,
        })
        .unwrap()
}

fn turn_left(session: &mut GameSession) -> crate::SessionView {
    let view = session.view().unwrap();
    session
        .command(PartyCommand::TurnLeft {
            actor_entity_id: view.current.unwrap().entity_id,
            expected_revision: view.revision,
        })
        .unwrap()
}

fn open_arena() -> crate::GeneratedFloor {
    let mut floor = generate_authored_floor(SEED).unwrap();
    floor.floor_id = "floor.session-arena".to_owned();
    floor.bounds = FloorBounds {
        min_x: 0,
        min_y: 0,
        width: 5,
        height: 5,
    };
    floor.walkable_cells = (0..5)
        .flat_map(|y| (0..5).map(move |x| FloorCell { x, y }))
        .collect();
    floor.regions.clear();
    floor.portals.clear();
    floor.features = vec![FloorFeature {
        id: "entry".to_owned(),
        source_node_id: "node.entry".to_owned(),
        kind: FloorFeatureKind::Entry,
        cell: FloorCell { x: 2, y: 2 },
    }];
    floor
}

fn static_rules(rolls: Vec<StaticRollCandidate>) -> RoguelikeRuleset {
    let mut candidate = starter_candidate().unwrap();
    candidate.roll_policy = RollPolicyCandidate {
        kind: RollPolicyKindCandidate::Static,
        seed: None,
        rolls,
    };
    RoguelikeRuleset::compile(vec![package_for_test(candidate)]).unwrap()
}

fn static_rules_single_enemy(rolls: Vec<StaticRollCandidate>) -> RoguelikeRuleset {
    let mut candidate = starter_candidate().unwrap();
    candidate
        .actors
        .retain(|actor| actor.side == crate::ActorSideCandidate::Party || actor.entity_id == 202);
    candidate.roll_policy = RollPolicyCandidate {
        kind: RollPolicyKindCandidate::Static,
        seed: None,
        rolls,
    };
    RoguelikeRuleset::compile(vec![package_for_test(candidate)]).unwrap()
}

fn package_for_test(candidate: RoguelikeRulesCandidate) -> gameplay_rules::AdmittedRulePackage {
    let source_id = gameplay_rules::RuleSourceId::parse("session-test").unwrap();
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
            package: gameplay_rules::RulePackageId::parse("session-test").unwrap(),
            version: gameplay_rules::RuleVersion::new(1).unwrap(),
            dependencies: vec![],
            sources: vec![gameplay_rules::RuleSource::new(source_id, "session-test.json").unwrap()],
            provenance,
        },
        candidate,
    )
    .unwrap()
}
