use rusty_engine::core_ids::EntityId;
use rusty_engine::gameplay_mechanics::{
    self, MechanicsScalar, OperationId, SourceInstanceId, SourceInstanceIdentity, TrackService,
    TrackSetPolicy, TrackSetRequest,
};
use rusty_engine::gameplay_rules;

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
        0
    );
    assert!(preparation.preparation.as_ref().unwrap().ready);
    assert!(preparation.party.iter().all(|member| {
        member.level == 1
            && member.class_level == 1
            && !member.abilities.is_empty()
            && !member.defenses.is_empty()
            && !member.feats.is_empty()
            && member.loadout.capacity.used > 0
            && member
                .loadout
                .equipment_slots
                .iter()
                .filter(|slot| slot.equipped.is_some())
                .count()
                == usize::try_from(member.loadout.capacity.used).unwrap()
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
    assert!(decision.can_wait);
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

    let wait: SessionCommandDto = serde_json::from_value(serde_json::json!({
        "kind": "wait",
        "actorEntityId": 102,
        "expectedRevision": 4
    }))
    .unwrap();
    assert!(matches!(
        wait,
        SessionCommandDto::Wait {
            actor_entity_id: 102,
            expected_revision: 4
        }
    ));
    assert!(
        serde_json::from_value::<SessionCommandDto>(serde_json::json!({
            "kind": "wait",
            "actorEntityId": 102,
            "expectedRevision": 4,
            "browserDelay": 0
        }))
        .is_err()
    );
}

#[test]
fn complete_save_reopens_preparation_exploration_and_active_combat_exactly() {
    let mut session = GameSession::new(
        WorldState::new(
            generate_authored_floor(SEED).unwrap(),
            starter_ruleset().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let preparation = session.encode_save().unwrap();
    let reopened = GameSession::decode_save(&preparation).unwrap();
    assert_eq!(reopened.view().unwrap(), session.view().unwrap());
    assert_eq!(reopened.encode_save().unwrap(), preparation);

    complete_preparation(&mut session);
    let exploration = session.encode_save().unwrap();
    let reopened = GameSession::decode_save(&exploration).unwrap();
    assert_eq!(reopened.view().unwrap(), session.view().unwrap());
    assert_eq!(reopened.encode_save().unwrap(), exploration);

    route_to_first_encounter(&mut session);
    let combat = session.view().unwrap();
    assert!(combat
        .order
        .iter()
        .any(|activation| activation.side == TurnSide::Opposition));
    let encoded = session.encode_save().unwrap();
    let reopened = GameSession::decode_save(&encoded).unwrap();
    assert_eq!(reopened.view().unwrap(), combat);
    assert_eq!(reopened.encode_save().unwrap(), encoded);
}

#[test]
fn complete_save_rejects_unknown_incompatible_stale_and_impossible_facts() {
    let session = GameSession::new(
        WorldState::new(
            generate_authored_floor(SEED).unwrap(),
            starter_ruleset().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let canonical = session.encode_save().unwrap();
    let value: serde_json::Value = serde_json::from_str(&canonical).unwrap();

    let mut unknown = value.clone();
    unknown["floor"]["bounds"]["browserHint"] = serde_json::json!(true);
    assert_eq!(
        GameSession::decode_save(&serde_json::to_string(&unknown).unwrap())
            .err()
            .unwrap()
            .code(),
        "session_save_noncanonical"
    );

    let mut disconnected = value.clone();
    disconnected["floor"]["walkableCells"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_eq!(
        GameSession::decode_save(&serde_json::to_string(&disconnected).unwrap())
            .err()
            .unwrap()
            .code(),
        "session_save_floor_mismatch"
    );

    let mut stale = value.clone();
    stale["session"]["revision"] = serde_json::json!(1);
    assert_eq!(
        GameSession::decode_save(&serde_json::to_string(&stale).unwrap())
            .err()
            .unwrap()
            .code(),
        "session_save_log_invalid"
    );

    let mut unbounded_revision = value.clone();
    unbounded_revision["session"]["revision"] = serde_json::json!(u64::MAX);
    assert_eq!(
        GameSession::decode_save(&serde_json::to_string(&unbounded_revision).unwrap())
            .err()
            .unwrap()
            .code(),
        "session_save_bounds_invalid"
    );

    let mut missing = value.clone();
    missing["session"].as_object_mut().unwrap().remove("round");
    assert_eq!(
        GameSession::decode_save(&serde_json::to_string(&missing).unwrap())
            .err()
            .unwrap()
            .code(),
        "session_save_decode"
    );

    let mut plausible_but_unreachable = value.clone();
    plausible_but_unreachable["session"]["targetCursors"]["201"] = serde_json::json!(0);
    assert_eq!(
        GameSession::decode_save(&serde_json::to_string(&plausible_but_unreachable).unwrap())
            .err()
            .unwrap()
            .code(),
        "session_save_replay_invalid"
    );

    let mut impossible = value;
    let tracks = impossible["entityState"]["registeredComponents"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|component| {
            component["typeId"]
                .as_str()
                .is_some_and(|id| id.contains("tracks"))
        })
        .unwrap();
    let party_tracks = tracks["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["entity"] == serde_json::json!(101))
        .unwrap();
    party_tracks["value"]["values"][0]["current"] = serde_json::json!(999);
    assert!(matches!(
        GameSession::decode_save(&serde_json::to_string(&impossible).unwrap())
            .err()
            .unwrap()
            .code(),
        "session_save_world_invalid" | "session_save_damage_history_invalid"
    ));
    assert_eq!(session.encode_save().unwrap(), canonical);
}

#[test]
fn complete_save_reopens_a_terminal_expedition() {
    let mut session = GameSession::new(
        WorldState::new(
            generate_authored_floor(SEED).unwrap(),
            starter_ruleset().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    complete_preparation(&mut session);
    autoplay_to_terminal(&mut session);
    assert_eq!(
        session.view().unwrap().outcome,
        crate::SessionOutcome::Victory
    );
    let encoded = session.encode_save().unwrap();
    let reopened = GameSession::decode_save(&encoded).unwrap();
    assert_eq!(reopened.view().unwrap(), session.view().unwrap());
    assert_eq!(reopened.encode_save().unwrap(), encoded);
}

#[test]
fn preparation_loadout_is_engine_backed_typed_and_atomic() {
    let mut session = GameSession::new(
        WorldState::new(
            generate_authored_floor(SEED).unwrap(),
            starter_ruleset().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let initial = session.view().unwrap();
    let stash = &initial.preparation.as_ref().unwrap().stash;
    assert_eq!(stash.capacity.used, 0);
    assert!(initial.preparation.as_ref().unwrap().ready);
    let owner = initial.party[0].entity_id;
    let armor = initial.party[0]
        .loadout
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
            from_owner_entity_id: owner,
            to_owner_entity_id: owner,
            destination_slot_id: Some("focus".to_owned()),
        })
        .expect_err("armor cannot occupy a focus slot");
    assert_eq!(invalid.code(), "session_loadout_slot_invalid");
    assert_eq!(session.view().unwrap(), initial);

    let moved = session
        .command(SessionCommand::MoveLoadoutItem {
            expected_revision: initial.revision,
            item_entity_id: armor.entity_id,
            from_owner_entity_id: owner,
            to_owner_entity_id: stash.owner_entity_id,
            destination_slot_id: None,
        })
        .unwrap();
    assert_eq!(moved.preparation.as_ref().unwrap().stash.capacity.used, 1);
    assert!(!moved.preparation.as_ref().unwrap().ready);
    assert_eq!(
        moved.party[0]
            .defenses
            .iter()
            .find(|defense| defense.defense_id.as_str() == "armor")
            .unwrap()
            .value,
        before_armor - 2
    );
    assert!(moved.party[0]
        .loadout
        .equipment_slots
        .iter()
        .any(|slot| slot.slot_id == "body" && slot.equipped.is_none()));

    let customized = session.encode_save().unwrap();
    let reopened = GameSession::decode_save(&customized).unwrap();
    assert_eq!(reopened.view().unwrap(), moved);
    assert_eq!(reopened.encode_save().unwrap(), customized);

    let stable = session.view().unwrap();
    let stale = session
        .command(SessionCommand::MoveLoadoutItem {
            expected_revision: initial.revision,
            item_entity_id: armor.entity_id,
            from_owner_entity_id: stash.owner_entity_id,
            to_owner_entity_id: owner,
            destination_slot_id: armor.equipment_slot_id.clone(),
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

    let restored = session
        .command(SessionCommand::MoveLoadoutItem {
            expected_revision: stable.revision,
            item_entity_id: armor.entity_id,
            from_owner_entity_id: stash.owner_entity_id,
            to_owner_entity_id: owner,
            destination_slot_id: armor.equipment_slot_id,
        })
        .unwrap();
    assert!(restored.preparation.as_ref().unwrap().ready);
    assert_eq!(
        restored.preparation.as_ref().unwrap().stash.capacity.used,
        0
    );
    assert_eq!(
        restored.party[0]
            .defenses
            .iter()
            .find(|defense| defense.defense_id.as_str() == "armor")
            .unwrap()
            .value,
        before_armor
    );
}

#[test]
fn initiative_order_and_single_party_action_settle_to_the_next_decision() {
    let world = WorldState::new(open_arena(), two_enemy_rules()).unwrap();
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
        TurnReceipt::OppositionMoved {
            actor_entity_id: 202
        }
    )));
    assert_ne!(
        session.world.enemy_position(EntityId::new(202)).unwrap(),
        session.world.party_position().unwrap()
    );
}

#[test]
fn wait_consumes_one_activation_settles_opposition_and_replays_exactly() {
    let world = WorldState::new(open_arena(), two_enemy_rules()).unwrap();
    let mut preparation = GameSession::new(world.fork().unwrap()).unwrap();
    let initial_preparation = preparation.view().unwrap();
    let rejected = preparation
        .command(SessionCommand::Wait {
            actor_entity_id: 102,
            expected_revision: initial_preparation.revision,
        })
        .expect_err("wait is unavailable during preparation");
    assert_eq!(rejected.code(), "session_preparation_active");
    assert_eq!(preparation.view().unwrap(), initial_preparation);

    let mut session = prepared_session(world);
    let initial = session.view().unwrap();
    let initial_save: serde_json::Value =
        serde_json::from_str(&session.encode_save().unwrap()).unwrap();
    let stale = session
        .command(SessionCommand::Wait {
            actor_entity_id: initial.current.as_ref().unwrap().entity_id,
            expected_revision: initial.revision - 1,
        })
        .expect_err("stale wait rejects before staging");
    assert_eq!(stale.code(), "session_revision_stale");
    assert_eq!(session.view().unwrap(), initial);

    let waited = session
        .command(SessionCommand::Wait {
            actor_entity_id: 102,
            expected_revision: initial.revision,
        })
        .unwrap();
    assert_eq!(waited.revision, initial.revision + 1);
    assert_eq!(waited.current.as_ref().unwrap().entity_id, 103);
    assert_eq!(
        waited.latest_receipts,
        vec![TurnReceipt::PartyWaited {
            actor_entity_id: 102
        }]
    );
    assert_eq!(waited.world, initial.world);
    let waited_save: serde_json::Value =
        serde_json::from_str(&session.encode_save().unwrap()).unwrap();
    assert_eq!(
        waited_save["session"]["nextRoll"],
        initial_save["session"]["nextRoll"]
    );

    let settled = session
        .command(SessionCommand::Wait {
            actor_entity_id: 103,
            expected_revision: waited.revision,
        })
        .unwrap();
    assert_eq!(settled.revision, waited.revision + 1);
    assert_eq!(settled.current.as_ref().unwrap().entity_id, 101);
    assert!(matches!(
        settled.latest_receipts.first(),
        Some(TurnReceipt::PartyWaited {
            actor_entity_id: 103
        })
    ));
    assert!(settled.latest_receipts.iter().any(|receipt| matches!(
        receipt,
        TurnReceipt::OppositionMoved { .. }
            | TurnReceipt::OppositionPassed { .. }
            | TurnReceipt::OppositionAttacked { .. }
    )));
    let mut persistent = GameSession::new(
        WorldState::new(
            generate_authored_floor(SEED).unwrap(),
            starter_ruleset().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    complete_preparation(&mut persistent);
    let persistent_view = persistent.view().unwrap();
    persistent
        .command(SessionCommand::Wait {
            actor_entity_id: persistent_view.current.as_ref().unwrap().entity_id,
            expected_revision: persistent_view.revision,
        })
        .unwrap();
    let persistent_save = persistent.encode_save().unwrap();
    let reopened = GameSession::decode_save(&persistent_save).unwrap();
    assert_eq!(reopened.view().unwrap(), persistent.view().unwrap());
    assert_eq!(reopened.encode_save().unwrap(), persistent_save);

    let opposition_cursor = session
        .order
        .iter()
        .position(|slot| slot.side == TurnSide::Opposition)
        .unwrap();
    session.cursor = opposition_cursor;
    let opposition_view = session.view().unwrap();
    let rejected = session
        .command(SessionCommand::Wait {
            actor_entity_id: opposition_view.current.as_ref().unwrap().entity_id,
            expected_revision: opposition_view.revision,
        })
        .expect_err("opposition cannot submit a party wait");
    assert_eq!(rejected.code(), "session_actor_not_current");
    assert_eq!(session.view().unwrap(), opposition_view);

    incapacitate(&mut session, 201);
    incapacitate(&mut session, 202);
    session.refresh_outcome().unwrap();
    let terminal = session.view().unwrap();
    let rejected = session
        .command(SessionCommand::Wait {
            actor_entity_id: 101,
            expected_revision: terminal.revision,
        })
        .expect_err("terminal wait rejects before staging");
    assert_eq!(rejected.code(), "session_terminal");
    assert_eq!(session.view().unwrap(), terminal);
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
fn ranged_opposition_moves_to_a_free_adjacent_cell_before_attacking() {
    let floor = open_arena();
    let rules = single_enemy_rules();
    let seeded = WorldState::new(floor.clone(), rules).unwrap();
    let mut durable = seeded.durable_state().unwrap();
    durable.enemies[0].world = EnemyWorldComponent::new(
        floor.floor_id.clone(),
        crate::WorldCell { x: 2, y: 4 },
        EnemyParticipation::Participating,
    )
    .unwrap();
    let mut session =
        prepared_session(WorldState::restore(floor, single_enemy_rules(), durable).unwrap());

    turn_right(&mut session);
    let moved = turn_right(&mut session);
    assert!(moved.latest_receipts.iter().any(|receipt| matches!(
        receipt,
        TurnReceipt::OppositionMoved {
            actor_entity_id: 202
        }
    )));
    assert!(!moved.latest_receipts.iter().any(|receipt| matches!(
        receipt,
        TurnReceipt::OppositionAttacked {
            actor_entity_id: 202,
            ..
        }
    )));
    assert_eq!(
        session.world.enemy_position(EntityId::new(202)).unwrap(),
        crate::WorldCell { x: 2, y: 3 }
    );
    assert_ne!(
        session.world.enemy_position(EntityId::new(202)).unwrap(),
        session.world.party_position().unwrap()
    );
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

fn route_to_first_encounter(session: &mut GameSession) {
    const ROUTE: &str = "
      right right right right forward forward right right forward forward
      right right right right right right right right right backward
      right right right right right right right right right right
      right right right right right right right right right right
      right right right right right right backward
      right right right right right right right right right right
      right right forward
    ";
    for token in ROUTE.split_whitespace() {
        let current = session.view().unwrap();
        if current
            .order
            .iter()
            .any(|activation| activation.side == TurnSide::Opposition)
        {
            return;
        }
        if !current.world.visible_actors.is_empty() {
            for _ in 0..4 {
                let advanced = turn_right(session);
                if advanced
                    .order
                    .iter()
                    .any(|activation| activation.side == TurnSide::Opposition)
                {
                    return;
                }
            }
            panic!("visible opposition did not join the next round");
        }
        let step = match token {
            "forward" => RelativeStep::Forward,
            "backward" => RelativeStep::Backward,
            "left" => RelativeStep::Left,
            "right" => RelativeStep::Right,
            _ => unreachable!("fixed route token"),
        };
        let view = session.view().unwrap();
        let actor = view.current.unwrap().entity_id;
        let next = match session.command(SessionCommand::Step {
            actor_entity_id: actor,
            expected_revision: view.revision,
            step,
        }) {
            Ok(next) => next,
            Err(error)
                if error.code() == "session_party_step_rejected"
                    && error.detail().contains("world_step_occupied") =>
            {
                let rotations = match step {
                    RelativeStep::Forward => 0,
                    RelativeStep::Right => 1,
                    RelativeStep::Backward => 2,
                    RelativeStep::Left => 3,
                };
                for _ in 0..rotations {
                    let turned = turn_right(session);
                    if turned
                        .order
                        .iter()
                        .any(|activation| activation.side == TurnSide::Opposition)
                    {
                        return;
                    }
                }
                for _ in 0..4 {
                    let advanced = turn_right(session);
                    if advanced
                        .order
                        .iter()
                        .any(|activation| activation.side == TurnSide::Opposition)
                    {
                        return;
                    }
                }
                panic!("occupied route cell did not reveal its actor");
            }
            Err(error) => panic!("fixed route step failed: {error}"),
        };
        if next
            .order
            .iter()
            .any(|activation| activation.side == TurnSide::Opposition)
        {
            return;
        }
    }
    panic!("fixed route did not reveal an encounter");
}

fn autoplay_to_terminal(session: &mut GameSession) {
    for _ in 0..1_000 {
        let view = session.view().unwrap();
        if view.outcome != crate::SessionOutcome::Ongoing {
            return;
        }
        let decision = view.decision.unwrap();
        if let Some(action) = decision
            .actions
            .iter()
            .find(|action| !action.legal_target_entity_ids.is_empty())
        {
            session
                .command(SessionCommand::UseAction {
                    actor_entity_id: decision.actor_entity_id,
                    expected_revision: decision.expected_revision,
                    action_id: action.action_id.clone(),
                    target_entity_id: action.legal_target_entity_ids[0],
                })
                .unwrap();
            continue;
        }
        let durable = session.world.durable_state().unwrap();
        let party = durable.party.position();
        let facing = durable.party.facing();
        let goals = durable
            .enemies
            .iter()
            .filter(|enemy| {
                session
                    .world
                    .entities()
                    .component::<gameplay_mechanics::TracksComponent>(EntityId::new(
                        enemy.entity_id,
                    ))
                    .unwrap()
                    .and_then(|tracks| tracks.current(&crate::vitality_track_id()))
                    .is_some_and(|value| value.get() > 0)
            })
            .map(|enemy| enemy.world.position())
            .collect::<BTreeSet<_>>();
        if let Some(adjacent) = goals.iter().find(|goal| {
            party.x.abs_diff(goal.x) + party.y.abs_diff(goal.y) == 1
                && view.world.visible_actors.is_empty()
        }) {
            let desired = match (adjacent.x - party.x, adjacent.y - party.y) {
                (0, -1) => crate::Facing::North,
                (1, 0) => crate::Facing::East,
                (0, 1) => crate::Facing::South,
                (-1, 0) => crate::Facing::West,
                _ => unreachable!("adjacent cardinal goal"),
            };
            if facing != desired {
                session
                    .command(SessionCommand::TurnRight {
                        actor_entity_id: decision.actor_entity_id,
                        expected_revision: decision.expected_revision,
                    })
                    .unwrap();
                continue;
            }
        }
        let step = decision.legal_steps.iter().copied().min_by_key(|step| {
            navigation_distance(
                relative_destination(party, facing, *step),
                &goals,
                session.world.floor(),
            )
        });
        if let Some(step) = step {
            session
                .command(SessionCommand::Step {
                    actor_entity_id: decision.actor_entity_id,
                    expected_revision: decision.expected_revision,
                    step,
                })
                .unwrap();
        } else {
            session
                .command(SessionCommand::TurnRight {
                    actor_entity_id: decision.actor_entity_id,
                    expected_revision: decision.expected_revision,
                })
                .unwrap();
        }
    }
    let view = session.view().unwrap();
    let durable = session.world.durable_state().unwrap();
    let alive = durable
        .enemies
        .iter()
        .filter_map(|enemy| {
            session
                .world
                .entities()
                .component::<gameplay_mechanics::TracksComponent>(EntityId::new(enemy.entity_id))
                .unwrap()
                .and_then(|tracks| tracks.current(&crate::vitality_track_id()))
                .filter(|value| value.get() > 0)
                .map(|value| (enemy.entity_id, enemy.world.position(), value.get()))
        })
        .collect::<Vec<_>>();
    panic!(
        "autoplay stalled at round {}, party {:?}, facing {:?}, visible {:?}, alive {:?}",
        view.round,
        durable.party.position(),
        durable.party.facing(),
        view.world.visible_actors,
        alive
    );
}

fn relative_destination(
    origin: crate::WorldCell,
    facing: crate::Facing,
    step: RelativeStep,
) -> crate::WorldCell {
    let forward = facing.forward();
    let right = facing.right_axis();
    let delta = match step {
        RelativeStep::Forward => forward,
        RelativeStep::Backward => (-forward.0, -forward.1),
        RelativeStep::Right => right,
        RelativeStep::Left => (-right.0, -right.1),
    };
    crate::WorldCell {
        x: origin.x + delta.0,
        y: origin.y + delta.1,
    }
}

fn navigation_distance(
    start: crate::WorldCell,
    goals: &BTreeSet<crate::WorldCell>,
    floor: &crate::GeneratedFloor,
) -> usize {
    let walkable = floor
        .walkable_cells
        .iter()
        .map(crate::WorldCell::from)
        .collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::from([start]);
    let mut queue = VecDeque::from([(start, 0_usize)]);
    while let Some((cell, distance)) = queue.pop_front() {
        if goals.contains(&cell) {
            return distance;
        }
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let next = crate::WorldCell {
                x: cell.x + dx,
                y: cell.y + dy,
            };
            if walkable.contains(&next) && visited.insert(next) {
                queue.push_back((next, distance + 1));
            }
        }
    }
    usize::MAX
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
    candidate.actors.retain(|actor| {
        actor.side == crate::ActorSideCandidate::Party || matches!(actor.entity_id, 201 | 202)
    });
    candidate
        .actors
        .iter_mut()
        .find(|actor| actor.entity_id == 201)
        .unwrap()
        .actions
        .retain(|action| action.as_str() == "move");
    RoguelikeRuleset::compile(vec![package_for_test(candidate)]).unwrap()
}

fn two_enemy_rules() -> RoguelikeRuleset {
    crate::rules::starter_ruleset_with_opposition(&[201, 202]).unwrap()
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
use std::collections::{BTreeSet, VecDeque};
