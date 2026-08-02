use std::collections::BTreeMap;

use core_ids::EntityId;
use gameplay_mechanics::TracksComponent;

use crate::{vitality_track_id, ActorDefinition, ActorSideCandidate, RoguelikeId, WorldState};

use super::roll::RollSource;
use super::{
    ActivationView, SessionCommand, SessionError, SessionLogEntry, SessionOutcome, SessionPhase,
    SessionView, TurnReceipt, TurnSide, MAX_SESSION_LOG_ENTRIES, SESSION_VIEW_SCHEMA_VERSION,
};

const MAX_AUTOMATIC_SETTLEMENTS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TurnSlot {
    pub entity: EntityId,
    pub actor_id: RoguelikeId,
    pub name: String,
    pub side: TurnSide,
    pub initiative: i16,
}

pub struct GameSession {
    pub(super) world: WorldState,
    pub(super) roll: RollSource,
    pub(super) order: Vec<TurnSlot>,
    pub(super) cursor: usize,
    pub(super) round: u64,
    pub(super) revision: u64,
    pub(super) phase: SessionPhase,
    pub(super) outcome: SessionOutcome,
    pub(super) latest_receipts: Vec<TurnReceipt>,
    pub(super) log: Vec<SessionLogEntry>,
    pub(super) next_log_id: u64,
    pub(super) target_cursors: BTreeMap<u64, usize>,
}

impl GameSession {
    pub fn new(world: WorldState) -> Result<Self, SessionError> {
        let roll = RollSource::new(world.rules().roll_policy())?;
        let mut session = Self {
            world,
            roll,
            order: vec![],
            cursor: 0,
            round: 1,
            revision: 0,
            phase: SessionPhase::Preparation,
            outcome: SessionOutcome::Ongoing,
            latest_receipts: vec![],
            log: vec![],
            next_log_id: 1,
            target_cursors: BTreeMap::new(),
        };
        session.initialize_canonical_loadout()?;
        session.rebuild_order()?;
        session.refresh_outcome()?;
        Ok(session)
    }

    pub fn command(&mut self, command: SessionCommand) -> Result<SessionView, SessionError> {
        if self.phase == SessionPhase::Expedition && self.outcome != SessionOutcome::Ongoing {
            return Err(error(
                "session_terminal",
                "a terminal session accepts no further commands",
            ));
        }
        if command.expected_revision() != self.revision {
            return Err(error(
                "session_revision_stale",
                "the command revision does not match current authoritative state",
            ));
        }
        let mut staged = self.fork()?;
        staged.latest_receipts.clear();
        match command {
            SessionCommand::MoveLoadoutItem {
                item_entity_id,
                from_owner_entity_id,
                to_owner_entity_id,
                destination_slot_id,
                ..
            } => staged.move_loadout_item(
                item_entity_id,
                from_owner_entity_id,
                to_owner_entity_id,
                destination_slot_id,
            )?,
            SessionCommand::BeginExpedition { .. } => staged.begin_expedition()?,
            command => {
                if staged.phase != SessionPhase::Expedition {
                    return Err(error(
                        "session_preparation_active",
                        "expedition commands are unavailable during preparation",
                    ));
                }
                let current = staged.current_slot().ok_or_else(|| {
                    error(
                        "session_no_party_activation",
                        "the session has no current activation",
                    )
                })?;
                if current.side != TurnSide::Party
                    || Some(current.entity.raw()) != command.actor_entity_id()
                {
                    return Err(error(
                        "session_actor_not_current",
                        "the command actor does not own the current party activation",
                    ));
                }
                staged.resolve_party_command(command)?;
                staged.prune_defeated_from_order()?;
                staged.advance()?;
                staged.refresh_outcome()?;
                staged.settle_automatic()?;
            }
        }
        staged.revision = staged
            .revision
            .checked_add(1)
            .ok_or_else(|| error("session_revision_overflow", "session revision overflowed"))?;
        staged.append_log()?;
        *self = staged;
        self.view()
    }

    pub fn view(&self) -> Result<SessionView, SessionError> {
        Ok(SessionView {
            schema_version: SESSION_VIEW_SCHEMA_VERSION,
            revision: self.revision,
            phase: self.phase,
            round: self.round,
            outcome: self.outcome,
            current: (self.phase == SessionPhase::Expedition
                && self.outcome == SessionOutcome::Ongoing)
                .then(|| self.current_slot().map(TurnSlot::view))
                .flatten(),
            order: if self.phase == SessionPhase::Expedition
                && self.outcome == SessionOutcome::Ongoing
            {
                self.order.iter().map(TurnSlot::view).collect()
            } else {
                vec![]
            },
            party: self.party_status()?,
            preparation: self.preparation_view()?,
            decision: self.party_decision()?,
            latest_receipts: self.latest_receipts.clone(),
            log: self.log.clone(),
            world: self
                .world
                .view()
                .map_err(|detail| error("session_world_view", detail.to_string()))?,
        })
    }

    pub fn world(&self) -> &WorldState {
        &self.world
    }

    fn fork(&self) -> Result<Self, SessionError> {
        Ok(Self {
            world: self
                .world
                .fork()
                .map_err(|detail| error("session_stage_world", detail.to_string()))?,
            roll: self.roll.clone(),
            order: self.order.clone(),
            cursor: self.cursor,
            round: self.round,
            revision: self.revision,
            phase: self.phase,
            outcome: self.outcome,
            latest_receipts: self.latest_receipts.clone(),
            log: self.log.clone(),
            next_log_id: self.next_log_id,
            target_cursors: self.target_cursors.clone(),
        })
    }

    fn append_log(&mut self) -> Result<(), SessionError> {
        if self.log.len() + self.latest_receipts.len() > MAX_SESSION_LOG_ENTRIES {
            return Err(error(
                "session_log_bound_exceeded",
                "the complete durable session log exceeded its fixed bound",
            ));
        }
        for receipt in &self.latest_receipts {
            self.log.push(SessionLogEntry {
                id: self.next_log_id,
                revision: self.revision,
                receipt: receipt.clone(),
            });
            self.next_log_id = self
                .next_log_id
                .checked_add(1)
                .ok_or_else(|| error("session_log_identity_overflow", "log identity overflowed"))?;
        }
        Ok(())
    }

    pub(super) fn current_slot(&self) -> Option<&TurnSlot> {
        self.order.get(self.cursor)
    }

    pub(super) fn advance(&mut self) -> Result<(), SessionError> {
        self.cursor += 1;
        if self.cursor >= self.order.len() {
            self.round = self
                .round
                .checked_add(1)
                .ok_or_else(|| error("session_round_overflow", "session round overflowed"))?;
            self.rebuild_order()?;
            self.cursor = 0;
        }
        Ok(())
    }

    fn prune_defeated_from_order(&mut self) -> Result<(), SessionError> {
        let Some(current) = self.current_slot().map(|slot| slot.entity) else {
            return Ok(());
        };
        let order = std::mem::take(&mut self.order);
        let mut survivors = Vec::with_capacity(order.len());
        for slot in order {
            if self.is_alive(slot.entity)? {
                survivors.push(slot);
            }
        }
        self.order = survivors;
        self.cursor = self
            .order
            .iter()
            .position(|slot| slot.entity == current)
            .ok_or_else(|| {
                error(
                    "session_current_actor_removed",
                    "the acting party member became unavailable during its own command",
                )
            })?;
        Ok(())
    }

    pub(super) fn settle_automatic(&mut self) -> Result<(), SessionError> {
        for _ in 0..MAX_AUTOMATIC_SETTLEMENTS {
            self.refresh_outcome()?;
            if self.outcome != SessionOutcome::Ongoing {
                return Ok(());
            }
            let Some(slot) = self.current_slot().cloned() else {
                return Err(error(
                    "session_activation_order_empty",
                    "ongoing session has no activation order",
                ));
            };
            if !self.is_alive(slot.entity)? {
                self.advance()?;
                continue;
            }
            if slot.side == TurnSide::Party {
                return Ok(());
            }
            self.resolve_opposition(slot.entity)?;
            self.prune_defeated_from_order()?;
            self.advance()?;
        }
        Err(error(
            "session_settlement_bound_exceeded",
            "automatic opposition settlement exceeded its fixed bound",
        ))
    }

    pub(super) fn rebuild_order(&mut self) -> Result<(), SessionError> {
        let participating = self
            .world
            .participating_enemies()
            .map_err(|detail| error("session_world_read", detail.to_string()))?;
        let mut order = Vec::new();
        for actor in self.world.rules().actors().values() {
            let entity = EntityId::new(actor.entity_id);
            if actor.side == ActorSideCandidate::Opposition && !participating.contains(&entity) {
                continue;
            }
            if !self.is_alive(entity)? {
                continue;
            }
            order.push(TurnSlot::from_actor(actor));
        }
        order.sort_by(|left, right| {
            right
                .initiative
                .cmp(&left.initiative)
                .then_with(|| left.entity.cmp(&right.entity))
        });
        self.order = order;
        Ok(())
    }

    pub(super) fn is_alive(&self, entity: EntityId) -> Result<bool, SessionError> {
        let tracks = self
            .world
            .entities()
            .component::<TracksComponent>(entity)
            .map_err(|detail| error("session_tracks_read", detail.to_string()))?
            .ok_or_else(|| error("session_tracks_missing", format!("entity {entity}")))?;
        Ok(tracks
            .current(&vitality_track_id())
            .is_some_and(|value| value.get() > 0))
    }

    pub(super) fn refresh_outcome(&mut self) -> Result<(), SessionError> {
        let mut party_alive = false;
        let mut opposition_alive = false;
        for actor in self.world.rules().actors().values() {
            if !self.is_alive(EntityId::new(actor.entity_id))? {
                continue;
            }
            match actor.side {
                ActorSideCandidate::Party => party_alive = true,
                ActorSideCandidate::Opposition => opposition_alive = true,
            }
        }
        self.outcome = if !party_alive {
            SessionOutcome::Defeat
        } else if !opposition_alive {
            SessionOutcome::Victory
        } else {
            SessionOutcome::Ongoing
        };
        Ok(())
    }
}

impl TurnSlot {
    fn from_actor(actor: &ActorDefinition) -> Self {
        Self {
            entity: EntityId::new(actor.entity_id),
            actor_id: actor.id.clone(),
            name: actor.name.clone(),
            side: match actor.side {
                ActorSideCandidate::Party => TurnSide::Party,
                ActorSideCandidate::Opposition => TurnSide::Opposition,
            },
            initiative: actor
                .abilities
                .iter()
                .find(|score| score.ability.as_str() == "finesse")
                .map(|score| score.score)
                .unwrap_or_default(),
        }
    }

    fn view(&self) -> ActivationView {
        ActivationView {
            entity_id: self.entity.raw(),
            actor_id: self.actor_id.clone(),
            name: self.name.clone(),
            side: self.side,
            initiative: self.initiative,
        }
    }
}

pub(super) fn error(code: &'static str, detail: impl Into<String>) -> SessionError {
    SessionError::new(code, detail)
}
