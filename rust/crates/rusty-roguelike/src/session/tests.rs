use core_ids::EntityId;
use gameplay_mechanics::{
    MechanicsScalar, OperationId, SourceInstanceId, SourceInstanceIdentity, TrackService,
    TrackSetPolicy, TrackSetRequest,
};

use crate::{
    admit_roguelike_candidate, generate_authored_floor, starter_candidate, starter_ruleset,
    EnemyParticipation, EnemyWorldComponent, FloorBounds, FloorCell, FloorFeature,
    FloorFeatureKind, RelativeStep, RoguelikePackageEnvelope, RoguelikeRulesCandidate,
    RoguelikeRuleset, RollPolicyCandidate, RollPolicyKindCandidate, SessionCommand,
    SessionCommandDto, SessionPhase, StaticRollCandidate, TurnReceipt, TurnSide, WorldState,
};

use super::GameSession;

const SEED: u64 = 5_201;

#[test]
fn view_projects_party_status_inventory_and_the_exact_current_decision() {
    let session =
        GameSession::new(WorldState::new(open_arena(), starter_ruleset().unwrap()).unwrap())
            .unwrap();
    let preparation = session.view().unwrap();
    assert_eq!(preparation.phase, SessionPhase::Preparation);
    assert!(preparation.current.is_none());
    assert_eq!(
        preparation
            .preparation
            .as_ref()
            .unwrap()
            .stash
            .capacity
            .used,
        7
    );
    assert!(preparation.party.iter().all(|member| {
        member.level == 1
            && member.class_level == 1
            && !member.abilities.is_empty()
            && !member.defenses.is_empty()
            && !member.feats.is_empty()
            && member.loadout.capacity.used == 0
    }));
    let mut session = session;
    complete_preparation(&mut session);
    let view = session.view().unwrap();
    let current = view.current.as_ref().unwrap();
    let decision = view.decision.as_ref().unwrap();

    assert_eq!(view.party.len(), 3);
    assert!(view.party.iter().all(|member| {
        member.current_vitality <= member.maximum_vitality
            && member.conscious == (member.current_vitality > 0)
            && member.loadout.capacity.used > 0
            && member
                .loadout
                .equipment_slots
                .iter()
                .filter(|slot| slot.equipped.is_some())
                .count()
                == usize::try_from(member.loadout.capacity.used).unwrap()
    }));
    assert_eq!(decision.actor_entity_id, current.entity_id);
    assert_eq!(decision.expected_revision, view.revision);
    assert!(!decision.legal_steps.is_empty());
    assert!(decision.can_turn);
    assert!(decision.actions.iter().any(|action| {
        action.action_id.as_str() == "aimed-shot" && action.legal_target_entity_ids.contains(&202)
    }));
}

#[test]
fn session_command_dto_is_closed_and_preserves_the_typed_action() {
    let canonical = serde_json::json!({
        "kind": "useAction",
        "actorEntityId": 102,
        "expectedRevision": 4,
        "actionId": "aimed-shot",
        "targetEntityId": 202
    });
    let decoded: SessionCommandDto = serde_json::from_value(canonical).unwrap();
    assert!(matches!(
        decoded,
        SessionCommandDto::UseAction {
            actor_entity_id: 102,
            expected_revision: 4,
            target_entity_id: 202,
            ref action_id,
        } if action_id.as_str() == "aimed-shot"
    ));

    let forged = serde_json::json!({
        "kind": "turnLeft",
        "actorEntityId": 102,
        "expectedRevision": 4,
        "ignoredLegality": true
    });
    assert!(serde_json::from_value::<SessionCommandDto>(forged).is_err());

    let move_item: SessionCommandDto = serde_json::from_value(serde_json::json!({
        "kind": "moveLoadoutItem",
        "expectedRevision": 0,
        "itemEntityId": 205,
        "fromOwnerEntityId": 204,
        "toOwnerEntityId": 101,
        "destinationSlotId": "body"
    }))
    .unwrap();
    assert!(matches!(
        move_item,
        SessionCommandDto::MoveLoadoutItem {
            expected_revision: 0,
            item_entity_id: 205,
            from_owner_entity_id: 204,
            to_owner_entity_id: 101,
            ref destination_slot_id,
        } if destination_slot_id.as_deref() == Some("body")
    ));
    assert!(
        serde_json::from_value::<SessionCommandDto>(serde_json::json!({
            "kind": "beginExpedition",
            "expectedRevision": 0,
            "browserReady": true
        }))
        .is_err()
    );
}

#[test]
fn preparation_loadout_is_engine_backed_typed_and_atomic() {
    let mut session =
        GameSession::new(WorldState::new(open_arena(), starter_ruleset().unwrap()).unwrap())
            .unwrap();
    let initial = session.view().unwrap();
    let stash = &initial.preparation.as_ref().unwrap().stash;
    let armor = stash
        .inventory_slots
        .iter()
        .flatten()
        .find(|item| item.item_id.as_str() == "scale-mail")
        .unwrap()
        .clone();
    let before_armor = initial.party[0]
        .defenses
        .iter()
        .find(|defense| defense.defense_id.as_str() == "armor")
        .unwrap()
        .value;

    let invalid = session
        .command(SessionCommand::MoveLoadoutItem {
            expected_revision: initial.revision,
            item_entity_id: armor.entity_id,
            from_owner_entity_id: stash.owner_entity_id,
            to_owner_entity_id: 101,
            destination_slot_id: Some("focus".to_owned()),
        })
        .expect_err("armor cannot occupy a focus slot");
    assert_eq!(invalid.code(), "session_loadout_slot_invalid");
    assert_eq!(session.view().unwrap(), initial);

    let moved = session
        .command(SessionCommand::MoveLoadoutItem {
            expected_revision: initial.revision,
            item_entity_id: armor.entity_id,
            from_owner_entity_id: stash.owner_entity_id,
            to_owner_entity_id: 101,
            destination_slot_id: armor.equipment_slot_id.clone(),
        })
        .unwrap();
    assert_eq!(moved.preparation.as_ref().unwrap().stash.capacity.used, 6);
    assert_eq!(
        moved.party[0]
            .defenses
            .iter()
            .find(|defense| defense.defense_id.as_str() == "armor")
            .unwrap()
            .value,
        before_armor + 2
    );
    assert!(moved.party[0]
        .loadout
        .equipment_slots
        .iter()
        .any(|slot| slot.slot_id == "body" && slot.equipped.is_some()));

    let stable = session.view().unwrap();
    let stale = session
        .command(SessionCommand::MoveLoadoutItem {
            expected_revision: initial.revision,
            item_entity_id: armor.entity_id,
            from_owner_entity_id: 101,
            to_owner_entity_id: 101,
            destination_slot_id: None,
        })
        .expect_err("stale loadout commands reject before mutation");
    assert_eq!(stale.code(), "session_revision_stale");
    assert_eq!(session.view().unwrap(), stable);

    let incomplete = session
        .command(SessionCommand::BeginExpedition {
            expected_revision: stable.revision,
        })
        .expect_err("the shared stash must be equipped first");
    assert_eq!(incomplete.code(), "session_preparation_incomplete");
    assert_eq!(session.view().unwrap(), stable);
}

#[test]
fn initiative_order_and_single_party_action_settle_to_the_next_decision() {
    let world = WorldState::new(open_arena(), starter_ruleset().unwrap()).unwrap();
    let before = world.durable_state().unwrap().party.position();
    let mut session = prepared_session(world);
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
        .command(SessionCommand::Step {
            actor_entity_id: 102,
            expected_revision: initial.revision,
            step: RelativeStep::Forward,
        })
        .unwrap();
    assert_eq!(next.revision, initial.revision + 1);
    assert_eq!(next.current.as_ref().unwrap().entity_id, 103);
    assert_eq!(
        session.world().durable_state().unwrap().party.position().y,
        before.y - 1
    );
    assert!(matches!(
        next.latest_receipts.first(),
        Some(TurnReceipt::PartyMoved {
            actor_entity_id: 102,
            ..
        })
    ));
    let after_mira = turn_right(&mut session);
    assert_eq!(after_mira.current.as_ref().unwrap().entity_id, 101);
    assert!(after_mira.latest_receipts.iter().any(|receipt| matches!(
        receipt,
        TurnReceipt::OppositionAttacked {
            actor_entity_id: 202,
            ..
        }
    )));
}

#[test]
fn adjacent_no_legal_opposition_passes_and_newly_seen_actor_joins_next_round() {
    let floor = open_arena();
    let rules = rules_without_goblin_attack();
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
    let world = WorldState::restore(floor, rules_without_goblin_attack(), durable).unwrap();
    let mut session = prepared_session(world);
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
    let mut session = prepared_session(world);
    let before = session.view().unwrap();
    let error = session
        .command(SessionCommand::UseAction {
            actor_entity_id: 102,
            expected_revision: before.revision,
            action_id: crate::RoguelikeId::parse("aimed-shot").unwrap(),
            target_entity_id: 202,
        })
        .expect_err("two damage dice cannot satisfy a one-die action");
    assert_eq!(error.code(), "session_static_roll_incompatible");
    assert_eq!(session.view().unwrap(), before);

    let mut session =
        prepared_session(WorldState::new(open_arena(), starter_ruleset().unwrap()).unwrap());
    let before_attack = session.view().unwrap();
    let resolved = session
        .command(SessionCommand::UseAction {
            actor_entity_id: 102,
            expected_revision: before_attack.revision,
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
        .command(SessionCommand::TurnLeft {
            actor_entity_id: 103,
            expected_revision: stable.revision - 1,
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
            d20: 1,
            damage: vec![1],
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
    let mut session = prepared_session(
        WorldState::restore(floor, static_rules_single_enemy(rolls), durable).unwrap(),
    );
    let attack = crate::RoguelikeId::parse("aimed-shot").unwrap();
    session
        .command(SessionCommand::UseAction {
            actor_entity_id: 102,
            expected_revision: session.view().unwrap().revision,
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
        .command(SessionCommand::UseAction {
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

#[test]
fn party_square_targeting_rotates_fairly_and_logs_the_complete_member_resolution() {
    let mut session = single_enemy_party_square_session();
    turn_right(&mut session);
    let first = turn_left(&mut session);
    let (first_target, first_eligible) = opposition_target(&first);
    assert_eq!(first_target, 101);
    assert_eq!(first_eligible, 3);
    let first_receipt = first
        .latest_receipts
        .iter()
        .find(|receipt| matches!(receipt, TurnReceipt::OppositionAttacked { .. }))
        .unwrap();
    assert!(matches!(
        first_receipt,
        TurnReceipt::OppositionAttacked {
            actor_entity_id: 202,
            action_id,
            target,
            damage_rolls,
            requested_damage,
            applied_damage,
            ..
        } if action_id.as_str() == "ember-shot"
            && target.selection_policy == crate::PartyMemberSelectionPolicy::RoundRobinLiving
            && damage_rolls.len() == 1
            && applied_damage <= requested_damage
    ));

    turn_right(&mut session);
    turn_left(&mut session);
    let second = turn_right(&mut session);
    assert_eq!(opposition_target(&second).0, 102);
}

#[test]
fn party_square_targeting_skips_incapacitated_members_without_browser_choice() {
    let mut session = single_enemy_party_square_session();
    incapacitate(&mut session, 101);
    let after_kestrel = turn_right(&mut session);
    assert!(!after_kestrel.order.iter().any(|slot| slot.entity_id == 101));
    let attacked = turn_left(&mut session);
    assert_eq!(opposition_target(&attacked), (102, 2));
}

fn turn_right(session: &mut GameSession) -> crate::SessionView {
    let view = session.view().unwrap();
    session
        .command(SessionCommand::TurnRight {
            actor_entity_id: view.current.unwrap().entity_id,
            expected_revision: view.revision,
        })
        .unwrap()
}

fn turn_left(session: &mut GameSession) -> crate::SessionView {
    let view = session.view().unwrap();
    session
        .command(SessionCommand::TurnLeft {
            actor_entity_id: view.current.unwrap().entity_id,
            expected_revision: view.revision,
        })
        .unwrap()
}

fn opposition_target(view: &crate::SessionView) -> (u64, u8) {
    view.latest_receipts
        .iter()
        .find_map(|receipt| match receipt {
            TurnReceipt::OppositionAttacked { target, .. } => Some((
                target.selected_member_entity_id,
                target.eligible_member_count,
            )),
            _ => None,
        })
        .expect("automatic opposition attack receipt")
}

fn incapacitate(session: &mut GameSession, entity_id: u64) {
    let operation = OperationId::parse(format!("test.incapacitate.{entity_id}")).unwrap();
    let catalog = session.world.rules().mechanics().clone();
    TrackService::set_under_policy(
        session.world.entities_mut(),
        &catalog,
        TrackSetRequest {
            operation: operation.clone(),
            source: SourceInstanceIdentity::Request {
                operation,
                instance: SourceInstanceId::parse("test.incapacitate").unwrap(),
            },
            entity: EntityId::new(entity_id),
            track: crate::vitality_track_id(),
            value: MechanicsScalar::zero(),
            policy: TrackSetPolicy::RejectOutOfBounds,
            expected_revision: None,
        },
    )
    .unwrap();
}

fn prepared_session(world: WorldState) -> GameSession {
    let mut session = GameSession::new(world).unwrap();
    complete_preparation(&mut session);
    session
}

fn complete_preparation(session: &mut GameSession) {
    let initial = session.view().unwrap();
    let stash = initial.preparation.unwrap().stash;
    for item in stash.inventory_slots.into_iter().flatten() {
        let owner = session
            .world
            .rules()
            .party()
            .members
            .iter()
            .map(|actor_id| &session.world.rules().actors()[actor_id])
            .find(|actor| actor.items.contains(&item.item_id))
            .unwrap()
            .entity_id;
        let view = session.view().unwrap();
        session
            .command(SessionCommand::MoveLoadoutItem {
                expected_revision: view.revision,
                item_entity_id: item.entity_id,
                from_owner_entity_id: stash.owner_entity_id,
                to_owner_entity_id: owner,
                destination_slot_id: item.equipment_slot_id,
            })
            .unwrap();
    }
    let view = session.view().unwrap();
    assert!(view.preparation.as_ref().unwrap().ready);
    session
        .command(SessionCommand::BeginExpedition {
            expected_revision: view.revision,
        })
        .unwrap();
}

fn single_enemy_party_square_session() -> GameSession {
    let floor = open_arena();
    let rules = single_enemy_rules();
    let seeded = WorldState::new(floor.clone(), rules).unwrap();
    let mut durable = seeded.durable_state().unwrap();
    durable.enemies[0].world = EnemyWorldComponent::new(
        floor.floor_id.clone(),
        crate::WorldCell { x: 2, y: 1 },
        EnemyParticipation::Participating,
    )
    .unwrap();
    prepared_session(WorldState::restore(floor, single_enemy_rules(), durable).unwrap())
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

fn single_enemy_rules() -> RoguelikeRuleset {
    let mut candidate = starter_candidate().unwrap();
    candidate
        .actors
        .retain(|actor| actor.side == crate::ActorSideCandidate::Party || actor.entity_id == 202);
    RoguelikeRuleset::compile(vec![package_for_test(candidate)]).unwrap()
}

fn rules_without_goblin_attack() -> RoguelikeRuleset {
    let mut candidate = starter_candidate().unwrap();
    candidate
        .actors
        .iter_mut()
        .find(|actor| actor.entity_id == 201)
        .unwrap()
        .actions
        .retain(|action| action.as_str() == "move");
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
