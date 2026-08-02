use std::collections::{BTreeMap, BTreeSet};

use core_ids::EntityId;
use entity_state::EntityStateSnapshot;
use serde::{Deserialize, Serialize};

use crate::{
    generate_authored_floor, starter_ruleset, vitality_track_id, ActionEffectDefinition,
    ActorBuildComponent, ActorSideCandidate, GeneratedFloor, RoguelikeId, WorldState,
    RUSTY_ENGINE_REVISION, RUSTY_PROCGEN_REVISION,
};

use super::roll::RollSource;
use super::runtime::{error, GameSession, TurnSlot};
use super::{
    PartyTurnDirection, SessionCommand, SessionError, SessionLogEntry, SessionOutcome,
    SessionPhase, TurnReceipt, TurnSide, MAX_SESSION_ACTIVATIONS, MAX_SESSION_LOG_ENTRIES,
    MAX_SESSION_RECEIPTS,
};

pub const GAME_SAVE_SCHEMA_VERSION: u32 = 4;
const MAX_GAME_SAVE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SaveEnvelope {
    schema_version: u32,
    rusty_engine_revision: String,
    rusty_procgen_revision: String,
    ruleset_fingerprint: String,
    floor: GeneratedFloor,
    entity_state: EntityStateSnapshot,
    session: SessionDurableState,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SaveEnvelopeRef<'a> {
    schema_version: u32,
    rusty_engine_revision: &'a str,
    rusty_procgen_revision: &'a str,
    ruleset_fingerprint: &'a str,
    floor: &'a GeneratedFloor,
    entity_state: EntityStateSnapshot,
    session: SessionDurableState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SessionDurableState {
    revision: u64,
    round: u64,
    phase: SessionPhase,
    outcome: SessionOutcome,
    order: Vec<DurableTurnSlot>,
    cursor: usize,
    next_roll: u64,
    target_cursors: BTreeMap<u64, usize>,
    latest_receipts: Vec<TurnReceipt>,
    log: Vec<SessionLogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DurableTurnSlot {
    entity_id: u64,
    actor_id: RoguelikeId,
    name: String,
    side: TurnSide,
    initiative: i16,
}

impl GameSession {
    pub fn encode_save(&self) -> Result<String, SessionError> {
        let save = SaveEnvelopeRef {
            schema_version: GAME_SAVE_SCHEMA_VERSION,
            rusty_engine_revision: RUSTY_ENGINE_REVISION,
            rusty_procgen_revision: RUSTY_PROCGEN_REVISION,
            ruleset_fingerprint: self.world.rules().fingerprint(),
            floor: self.world.floor(),
            entity_state: self.world.entity_snapshot(),
            session: self.durable_session(),
        };
        serde_json::to_string_pretty(&save)
            .map_err(|detail| error("session_save_encode", detail.to_string()))
    }

    pub fn decode_save(input: &str) -> Result<Self, SessionError> {
        if input.len() > MAX_GAME_SAVE_BYTES {
            return Err(error(
                "session_save_too_large",
                format!("save exceeds {MAX_GAME_SAVE_BYTES} bytes"),
            ));
        }
        let raw: serde_json::Value = serde_json::from_str(input)
            .map_err(|detail| error("session_save_decode", detail.to_string()))?;
        let save: SaveEnvelope = serde_json::from_value(raw.clone())
            .map_err(|detail| error("session_save_decode", detail.to_string()))?;
        if save.schema_version != GAME_SAVE_SCHEMA_VERSION {
            return Err(error(
                "session_save_schema_unsupported",
                format!("unsupported save schema {}", save.schema_version),
            ));
        }
        if save.rusty_engine_revision != RUSTY_ENGINE_REVISION {
            return Err(error(
                "session_save_engine_mismatch",
                "save uses a different Rusty Engine revision",
            ));
        }
        if save.rusty_procgen_revision != RUSTY_PROCGEN_REVISION {
            return Err(error(
                "session_save_procgen_mismatch",
                "save uses a different Rusty Procgen revision",
            ));
        }
        let regenerated = generate_authored_floor(save.floor.provenance.seed)
            .map_err(|detail| error("session_save_floor_invalid", detail.to_string()))?;
        if regenerated != save.floor {
            return Err(error(
                "session_save_floor_mismatch",
                "saved floor and Procgen provenance do not reproduce exactly",
            ));
        }
        let rules = starter_ruleset()
            .map_err(|detail| error("session_save_rules_invalid", detail.to_string()))?;
        if save.ruleset_fingerprint != rules.fingerprint() {
            return Err(error(
                "session_save_rules_mismatch",
                "save uses a different compiled ruleset",
            ));
        }
        // Serde's closed structs reject unknown typed fields. This exact normalized
        // round-trip also closes nested donor DTOs which predate deny_unknown_fields.
        let canonical = serde_json::to_value(SaveEnvelopeRef {
            schema_version: save.schema_version,
            rusty_engine_revision: &save.rusty_engine_revision,
            rusty_procgen_revision: &save.rusty_procgen_revision,
            ruleset_fingerprint: &save.ruleset_fingerprint,
            floor: &save.floor,
            entity_state: save.entity_state.clone(),
            session: save.session.clone(),
        })
        .map_err(|detail| error("session_save_decode", detail.to_string()))?;
        if canonical != raw {
            return Err(error(
                "session_save_noncanonical",
                "save contains unknown, omitted, or noncanonical facts",
            ));
        }
        let floor = save.floor;
        let entity_state = save.entity_state;
        let durable_session = save.session;
        let world =
            WorldState::restore_snapshot(floor.clone(), rules.clone(), entity_state.clone())
                .map_err(|detail| error("session_save_world_invalid", detail.to_string()))?;
        let restored = Self::restore_session(world, durable_session.clone())?;
        validate_replay(floor, rules, &entity_state, &durable_session, &restored)?;
        Ok(restored)
    }

    fn durable_session(&self) -> SessionDurableState {
        SessionDurableState {
            revision: self.revision,
            round: self.round,
            phase: self.phase,
            outcome: self.outcome,
            order: self.order.iter().map(DurableTurnSlot::from).collect(),
            cursor: self.cursor,
            next_roll: self.roll.next_roll(),
            target_cursors: self.target_cursors.clone(),
            latest_receipts: self.latest_receipts.clone(),
            log: self.log.clone(),
        }
    }

    fn restore_session(world: WorldState, save: SessionDurableState) -> Result<Self, SessionError> {
        validate_session_shape(&world, &save)?;
        let roll = RollSource::restore(world.rules().roll_policy(), save.next_roll)?;
        let next_log_id = save.log.last().map_or(Ok(1), |entry| {
            entry
                .id
                .checked_add(1)
                .ok_or_else(|| error("session_log_identity_overflow", "log identity overflowed"))
        })?;
        let mut session = Self {
            world,
            roll,
            order: vec![],
            cursor: save.cursor,
            round: save.round,
            revision: save.revision,
            phase: save.phase,
            outcome: save.outcome,
            latest_receipts: save.latest_receipts,
            log: save.log,
            next_log_id,
            target_cursors: save.target_cursors,
        };
        session.rebuild_order()?;
        let derived = session
            .order
            .iter()
            .map(DurableTurnSlot::from)
            .collect::<Vec<_>>();
        if derived != save.order {
            return Err(error(
                "session_save_order_invalid",
                "saved activation order disagrees with current world state",
            ));
        }
        let outcome = session.outcome;
        session.refresh_outcome()?;
        if session.outcome != outcome {
            return Err(error(
                "session_save_outcome_invalid",
                "saved outcome disagrees with current vitality",
            ));
        }
        session.view()?;
        Ok(session)
    }
}

fn validate_replay(
    floor: GeneratedFloor,
    rules: crate::RoguelikeRuleset,
    expected_entities: &EntityStateSnapshot,
    expected_session: &SessionDurableState,
    restored: &GameSession,
) -> Result<(), SessionError> {
    let mut replay = GameSession::new(
        WorldState::new(floor, rules)
            .map_err(|detail| error("session_save_replay_invalid", detail.to_string()))?,
    )?;
    for revision in 1..=expected_session.revision {
        let receipts = expected_session
            .log
            .iter()
            .filter(|entry| entry.revision == revision)
            .map(|entry| entry.receipt.clone())
            .collect::<Vec<_>>();
        let first = receipts.first().ok_or_else(|| {
            error(
                "session_save_replay_invalid",
                format!("revision {revision} has no initiating receipt"),
            )
        })?;
        let command = command_from_receipt(first, replay.revision)?;
        let view = replay.command(command)?;
        if view.latest_receipts != receipts {
            return Err(error(
                "session_save_replay_invalid",
                format!("revision {revision} receipts do not replay exactly"),
            ));
        }
    }
    if replay.world.entity_snapshot() != *expected_entities
        || replay.durable_session() != *expected_session
        || replay.view()? != restored.view()?
    {
        return Err(error(
            "session_save_replay_invalid",
            "saved snapshot and lifecycle facts do not match deterministic replay",
        ));
    }
    Ok(())
}

fn command_from_receipt(
    receipt: &TurnReceipt,
    expected_revision: u64,
) -> Result<SessionCommand, SessionError> {
    match receipt {
        TurnReceipt::LoadoutMoved {
            item_entity_id,
            from_owner_entity_id,
            to_owner_entity_id,
            destination_slot_id,
        } => Ok(SessionCommand::MoveLoadoutItem {
            expected_revision,
            item_entity_id: *item_entity_id,
            from_owner_entity_id: *from_owner_entity_id,
            to_owner_entity_id: *to_owner_entity_id,
            destination_slot_id: destination_slot_id.clone(),
        }),
        TurnReceipt::ExpeditionBegan => Ok(SessionCommand::BeginExpedition { expected_revision }),
        TurnReceipt::PartyMoved {
            actor_entity_id,
            step,
        } => Ok(SessionCommand::Step {
            actor_entity_id: *actor_entity_id,
            expected_revision,
            step: *step,
        }),
        TurnReceipt::PartyTurned {
            actor_entity_id,
            direction,
        } => Ok(match direction {
            PartyTurnDirection::Left => SessionCommand::TurnLeft {
                actor_entity_id: *actor_entity_id,
                expected_revision,
            },
            PartyTurnDirection::Right => SessionCommand::TurnRight {
                actor_entity_id: *actor_entity_id,
                expected_revision,
            },
        }),
        TurnReceipt::PartyWaited { actor_entity_id } => Ok(SessionCommand::Wait {
            actor_entity_id: *actor_entity_id,
            expected_revision,
        }),
        TurnReceipt::PartyAttacked {
            actor_entity_id,
            target_entity_id,
            action_id,
            ..
        } => Ok(SessionCommand::UseAction {
            actor_entity_id: *actor_entity_id,
            expected_revision,
            action_id: action_id.clone(),
            target_entity_id: *target_entity_id,
        }),
        TurnReceipt::OppositionAttacked { .. }
        | TurnReceipt::OppositionMoved { .. }
        | TurnReceipt::OppositionPassed { .. } => Err(error(
            "session_save_replay_invalid",
            "a revision begins with an automatic opposition receipt",
        )),
    }
}

impl From<&TurnSlot> for DurableTurnSlot {
    fn from(value: &TurnSlot) -> Self {
        Self {
            entity_id: value.entity.raw(),
            actor_id: value.actor_id.clone(),
            name: value.name.clone(),
            side: value.side,
            initiative: value.initiative,
        }
    }
}

fn validate_session_shape(
    world: &WorldState,
    save: &SessionDurableState,
) -> Result<(), SessionError> {
    if save.round == 0
        || save.revision > MAX_SESSION_LOG_ENTRIES as u64
        || save.order.len() > MAX_SESSION_ACTIVATIONS
        || save.latest_receipts.len() > MAX_SESSION_RECEIPTS
        || save.log.len() > MAX_SESSION_LOG_ENTRIES
        || save.round > save.revision.saturating_add(1)
    {
        return Err(error(
            "session_save_bounds_invalid",
            "saved session counters or collections exceed their bounds",
        ));
    }
    if (save.phase == SessionPhase::Preparation
        && (save.outcome != SessionOutcome::Ongoing || save.cursor != 0 || save.round != 1))
        || (save.phase == SessionPhase::Expedition
            && save.outcome == SessionOutcome::Ongoing
            && (save.order.is_empty() || save.cursor >= save.order.len()))
    {
        return Err(error(
            "session_save_phase_invalid",
            "saved phase, outcome, and activation cursor are inconsistent",
        ));
    }
    let party = world
        .rules()
        .party()
        .members
        .iter()
        .map(|id| world.rules().actors()[id].entity_id)
        .collect::<BTreeSet<_>>();
    let opposition = world
        .rules()
        .actors()
        .values()
        .filter(|actor| actor.side == ActorSideCandidate::Opposition)
        .map(|actor| actor.entity_id)
        .collect::<BTreeSet<_>>();
    if save
        .target_cursors
        .iter()
        .any(|(entity, cursor)| !opposition.contains(entity) || *cursor >= party.len())
    {
        return Err(error(
            "session_save_target_cursor_invalid",
            "saved target cursors do not belong to compiled opposition",
        ));
    }
    validate_log(world, save, &party, &opposition)
}

fn validate_log(
    world: &WorldState,
    save: &SessionDurableState,
    party: &BTreeSet<u64>,
    opposition: &BTreeSet<u64>,
) -> Result<(), SessionError> {
    let mut expected_id = 1_u64;
    let mut seen_revisions = BTreeSet::new();
    let mut attack_count = 0_u64;
    let mut expedition_began = 0_usize;
    let mut applied_damage = BTreeMap::<u64, u64>::new();
    for entry in &save.log {
        if entry.id != expected_id || entry.revision == 0 || entry.revision > save.revision {
            return Err(error(
                "session_save_log_invalid",
                "saved log identities and revisions are not canonical",
            ));
        }
        expected_id = expected_id
            .checked_add(1)
            .ok_or_else(|| error("session_log_identity_overflow", "log identity overflowed"))?;
        seen_revisions.insert(entry.revision);
        validate_receipt(world, &entry.receipt, party, opposition)?;
        match entry.receipt {
            TurnReceipt::PartyAttacked {
                target_entity_id,
                applied_damage: damage,
                ..
            } => {
                attack_count += 1;
                *applied_damage.entry(target_entity_id).or_default() += u64::from(damage);
            }
            TurnReceipt::OppositionAttacked {
                ref target,
                applied_damage: damage,
                ..
            } => {
                attack_count += 1;
                *applied_damage
                    .entry(target.selected_member_entity_id)
                    .or_default() += u64::from(damage);
            }
            TurnReceipt::ExpeditionBegan => expedition_began += 1,
            _ => {}
        }
    }
    let expected_revisions = (1..=save.revision).collect::<BTreeSet<_>>();
    if seen_revisions != expected_revisions || attack_count != save.next_roll {
        return Err(error(
            "session_save_log_invalid",
            "saved log does not account for every command and random roll",
        ));
    }
    let latest = save
        .log
        .iter()
        .filter(|entry| entry.revision == save.revision)
        .map(|entry| entry.receipt.clone())
        .collect::<Vec<_>>();
    if latest != save.latest_receipts
        || (save.phase == SessionPhase::Preparation && expedition_began != 0)
        || (save.phase == SessionPhase::Expedition && expedition_began != 1)
    {
        return Err(error(
            "session_save_log_invalid",
            "saved latest receipts or phase transition log are inconsistent",
        ));
    }
    for actor in world.rules().actors().values() {
        let current = world
            .entities()
            .component::<gameplay_mechanics::TracksComponent>(EntityId::new(actor.entity_id))
            .map_err(|detail| error("session_save_tracks_invalid", detail.to_string()))?
            .and_then(|tracks| tracks.current(&vitality_track_id()))
            .ok_or_else(|| error("session_save_tracks_invalid", "vitality track is missing"))?
            .get();
        let expected = u64::from(actor.vitality).saturating_sub(
            applied_damage
                .get(&actor.entity_id)
                .copied()
                .unwrap_or_default(),
        );
        if current != i64::try_from(expected).expect("u16 vitality fits i64") {
            return Err(error(
                "session_save_damage_history_invalid",
                format!(
                    "entity {} vitality disagrees with the durable log",
                    actor.entity_id
                ),
            ));
        }
    }
    Ok(())
}

fn validate_receipt(
    world: &WorldState,
    receipt: &TurnReceipt,
    party: &BTreeSet<u64>,
    opposition: &BTreeSet<u64>,
) -> Result<(), SessionError> {
    let actor_valid = match receipt {
        TurnReceipt::PartyMoved {
            actor_entity_id, ..
        }
        | TurnReceipt::PartyTurned {
            actor_entity_id, ..
        }
        | TurnReceipt::PartyWaited {
            actor_entity_id, ..
        }
        | TurnReceipt::PartyAttacked {
            actor_entity_id, ..
        } => party.contains(actor_entity_id),
        TurnReceipt::OppositionMoved {
            actor_entity_id, ..
        }
        | TurnReceipt::OppositionPassed {
            actor_entity_id, ..
        }
        | TurnReceipt::OppositionAttacked {
            actor_entity_id, ..
        } => opposition.contains(actor_entity_id),
        TurnReceipt::LoadoutMoved {
            item_entity_id,
            from_owner_entity_id,
            to_owner_entity_id,
            ..
        } => {
            let item = EntityId::new(*item_entity_id);
            world
                .entities()
                .component::<gameplay_mechanics::ItemComponent>(item)
                .ok()
                .flatten()
                .is_some()
                && (party.contains(from_owner_entity_id)
                    || *from_owner_entity_id == world.stash_entity().raw())
                && (party.contains(to_owner_entity_id)
                    || *to_owner_entity_id == world.stash_entity().raw())
        }
        TurnReceipt::ExpeditionBegan => true,
    };
    if !actor_valid {
        return Err(error(
            "session_save_receipt_invalid",
            "saved receipt references an impossible actor, item, or owner",
        ));
    }
    match receipt {
        TurnReceipt::PartyAttacked {
            actor_entity_id,
            target_entity_id,
            action_id,
            d20,
            ability_modifier,
            attack_total,
            defense,
            hit,
            damage_rolls,
            damage_bonus,
            requested_damage,
            applied_damage,
        } => validate_attack_receipt(
            world,
            *actor_entity_id,
            *target_entity_id,
            action_id,
            *d20,
            *ability_modifier,
            *attack_total,
            *defense,
            *hit,
            damage_rolls,
            *damage_bonus,
            *requested_damage,
            *applied_damage,
        )?,
        TurnReceipt::OppositionAttacked {
            actor_entity_id,
            action_id,
            target,
            d20,
            ability_modifier,
            attack_total,
            defense,
            hit,
            damage_rolls,
            damage_bonus,
            requested_damage,
            applied_damage,
        } => validate_attack_receipt(
            world,
            *actor_entity_id,
            target.selected_member_entity_id,
            action_id,
            *d20,
            *ability_modifier,
            *attack_total,
            *defense,
            *hit,
            damage_rolls,
            *damage_bonus,
            *requested_damage,
            *applied_damage,
        )?,
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_attack_receipt(
    world: &WorldState,
    actor_entity_id: u64,
    _target_entity_id: u64,
    action_id: &RoguelikeId,
    d20: u8,
    ability_modifier: i16,
    attack_total: i16,
    defense: i16,
    hit: bool,
    damage_rolls: &[u16],
    damage_bonus: i16,
    requested_damage: u16,
    applied_damage: u16,
) -> Result<(), SessionError> {
    let actor = world
        .rules()
        .actors()
        .values()
        .find(|actor| actor.entity_id == actor_entity_id)
        .ok_or_else(|| error("session_save_receipt_invalid", "receipt actor is unknown"))?;
    let build = world
        .entities()
        .component::<ActorBuildComponent>(EntityId::new(actor_entity_id))
        .map_err(|detail| error("session_save_receipt_invalid", detail.to_string()))?
        .ok_or_else(|| error("session_save_receipt_invalid", "receipt actor has no build"))?;
    let action = world
        .rules()
        .actions()
        .get(action_id)
        .filter(|_| build.actions().contains(action_id))
        .ok_or_else(|| {
            error(
                "session_save_receipt_invalid",
                format!("{} cannot use action {action_id}", actor.id),
            )
        })?;
    let ActionEffectDefinition::Attack { damage, .. } = &action.effect else {
        return Err(error(
            "session_save_receipt_invalid",
            "attack receipt names a non-attack action",
        ));
    };
    let rolled = damage_rolls
        .iter()
        .map(|value| i64::from(*value))
        .sum::<i64>();
    let computed_damage = (rolled + i64::from(damage_bonus)).max(0);
    let requested = if hit {
        u16::try_from(computed_damage).ok()
    } else {
        Some(0)
    };
    if !(1..=20).contains(&d20)
        || attack_total != i16::from(d20).saturating_add(ability_modifier)
        || hit != (attack_total >= defense)
        || damage_rolls.len() != usize::from(damage.dice)
        || damage_rolls
            .iter()
            .any(|value| *value == 0 || *value > damage.sides)
        || damage_bonus != damage.bonus
        || requested != Some(requested_damage)
        || applied_damage > requested_damage
    {
        return Err(error(
            "session_save_receipt_invalid",
            "saved attack receipt contains impossible resolution facts",
        ));
    }
    Ok(())
}
