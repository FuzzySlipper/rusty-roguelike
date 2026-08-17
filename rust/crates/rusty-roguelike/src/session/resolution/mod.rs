mod policy;
mod transaction;

use rusty_engine::core_ids::EntityId;
use rusty_engine::gameplay_mechanics::OperationId;
use rusty_engine::gameplay_resolution::{
    AttemptStatus, CommitStatus, CorrelationId, PolicyFailure, ResolutionId, ResolutionIdentity,
    ResolutionMode, ResolutionPolicy, ResolutionReceipt, ResolutionRequest, ResolutionTraceSink,
    StandardResolver,
};

use crate::{
    ActionDefinition, ActionEffectDefinition, ActionTargetCandidate, ActorDefinition,
    ActorSideCandidate, DamageDefinition, RoguelikeId, WorldState,
};

use super::roll::{AttackRoll, RollSource};
use super::runtime::{error, GameSession};
use super::{
    LegalActionView, PartyDecisionView, PartyMemberSelectionPolicy, PartySquareTargetReceipt,
    PartyTurnDirection, SessionCommand, SessionError, SessionOutcome, SessionPhase, TurnReceipt,
    TurnSide,
};

use policy::RoguelikeResolutionPolicy;
use transaction::RoguelikeTransaction;

/// Who originated the attack attempt; player and opposition converge on the
/// same resolution policy through this intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RoguelikeIntentOrigin {
    Party,
    Opposition,
}

/// The raw attempt: a concrete actor, action, and target entity. Party-square
/// member selection is resolved downstream before this intent is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoguelikeIntent {
    pub actor_entity_id: u64,
    pub action_id: RoguelikeId,
    pub target_entity_id: u64,
    pub origin: RoguelikeIntentOrigin,
}

/// The admitted attempt after identity/ownership/availability validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoguelikeAdmittedIntent {
    pub actor: ActorDefinition,
    pub action: ActionDefinition,
    pub target_entity_id: u64,
    pub origin: RoguelikeIntentOrigin,
}

/// Immutable facts materialized during the gather phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoguelikeFacts {
    pub ability_score: i16,
    pub defense_value: i16,
    pub target: RoguelikeTargetFacts,
}

/// Immutable target facts for the legality checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoguelikeTargetFacts {
    pub side: ActorSideCandidate,
    pub alive: bool,
    pub visible: bool,
    pub participating: bool,
    pub distance: Option<u32>,
}

/// No conditional grammar yet; attack programs are a single operation node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikePredicate {}

/// The authored attack operation carried by the plan program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikeOperation {
    Attack {
        ability: RoguelikeId,
        defense: RoguelikeId,
        damage: DamageDefinition,
        range: u8,
    },
}

/// A staged damage effect; only applied by the transaction's commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikeEffect {
    Damage { target_entity_id: u64, amount: u16 },
}

/// The resolved attack facts carried from planning into the TurnReceipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoguelikeAttackResolved {
    pub d20: u8,
    pub ability_modifier: i16,
    pub attack_total: i16,
    pub defense: i16,
    pub hit: bool,
    pub damage_rolls: Vec<u16>,
    pub damage_bonus: i16,
    pub requested_damage: u16,
}

/// The event emitted for a planned attack; the receipt fields are projected
/// from it after the attempt commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoguelikeEvent {
    pub attack_resolved: RoguelikeAttackResolved,
}

/// Bounded caller-supplied roll evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoguelikeEvidence {
    pub id: String,
    pub value: i64,
}

/// No interceptors exist in the starter content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikeInterceptor {}

/// Policy explanation records for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikeTraceDetail {
    Decision { reason: String },
}

/// Suspension is unused by the roguelike attack policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikeSuspension {}

/// Typed attack rejections; each maps 1:1 to an existing SessionError code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikeRejection {
    UnknownActor {
        entity_id: u64,
    },
    UnknownTarget {
        entity_id: u64,
    },
    UnknownAction {
        action_id: String,
    },
    NotOwned {
        action_id: String,
    },
    EquipmentRequired {
        action_id: String,
    },
    ActivationCostInvalid {
        action_id: String,
    },
    PartyTargetModeInvalid {
        action_id: String,
    },
    OppositionTargetModeInvalid {
        action_id: String,
    },
    NotAnAttack {
        action_id: String,
    },
    TargetNotOpposition {
        entity_id: u64,
    },
    TargetNotParticipating {
        entity_id: u64,
    },
    TargetDead {
        entity_id: u64,
    },
    TargetNotVisible {
        entity_id: u64,
    },
    TargetOutOfRange {
        entity_id: u64,
    },
    MissingEvidence {
        id: String,
    },
    RollOutOfBounds {
        id: String,
        value: i64,
        minimum: i64,
        maximum: i64,
    },
}

/// Typed faults; plumbing failures carry the existing SessionError code they
/// must surface as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikeFault {
    Session { code: &'static str, detail: String },
}

/// Transaction commit failures map 1:1 to the existing damage error codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RoguelikeTransactionError {
    SourceInvalid { detail: String },
    DamageInvalid { detail: String },
    DamageKindInvalid { detail: String },
    DamageFailed { detail: String },
}

/// The attack facts a resolved attempt returns for TurnReceipt projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoguelikeResolutionOutcome {
    pub d20: u8,
    pub ability_modifier: i16,
    pub attack_total: i16,
    pub defense: i16,
    pub hit: bool,
    pub damage_rolls: Vec<u16>,
    pub damage_bonus: i16,
    pub requested_damage: u16,
    pub applied_damage: u16,
}

type RoguelikeResolutionReceipt = ResolutionReceipt<
    RoguelikeIntent,
    RoguelikeAdmittedIntent,
    RoguelikeFacts,
    RoguelikeEvidence,
    RoguelikeEffect,
    RoguelikeEvent,
    RoguelikeRejection,
    RoguelikeFault,
    RoguelikeSuspension,
    RoguelikeTraceDetail,
    RoguelikeTransactionError,
>;

/// Resolves one attack attempt through the Engine gameplay_resolution
/// lifecycle. Legality is checked by the policy first without consuming a
/// roll (parity with the previous manual flow), then the roll is drawn and
/// supplied as explicit evidence, and the attempt is planned and committed.
pub(super) fn resolve_roguelike_attack(
    world: &mut WorldState,
    roll: &mut RollSource,
    identity: ResolutionIdentity,
    operation: OperationId,
    intent: RoguelikeIntent,
) -> Result<RoguelikeResolutionOutcome, SessionError> {
    let snapshot = world
        .fork()
        .map_err(|detail| error("session_stage_world", detail.to_string()))?;
    let mut policy = RoguelikeResolutionPolicy::new(snapshot, operation.clone());
    let admitted = preflight(&mut policy, &intent)?;
    let ActionEffectDefinition::Attack { damage, .. } = &admitted.action.effect else {
        return Err(error(
            "session_action_requires_step",
            "attack resolution received a non-attack action",
        ));
    };
    let attack_roll = roll.attack(damage.dice, damage.sides)?;
    let evidence = attack_evidence(&admitted.action.id, damage, &attack_roll);
    let snapshot = world
        .fork()
        .map_err(|detail| error("session_stage_world", detail.to_string()))?;
    let mut policy = RoguelikeResolutionPolicy::new(snapshot, operation.clone());
    let mut transaction = RoguelikeTransaction::new(
        world,
        world.rules().mechanics().clone(),
        operation,
        intent.actor_entity_id,
        admitted.action.id.clone(),
        damage.kind.clone(),
    );
    let receipt = StandardResolver::default().resolve(
        &mut policy,
        &mut transaction,
        ResolutionRequest::new(identity, ResolutionMode::Apply, intent, evidence),
    );
    if !receipt.succeeded() {
        return Err(receipt_failure(&receipt));
    }
    let resolved = receipt
        .events()
        .first()
        .ok_or_else(|| {
            error(
                "session_resolution_invalid",
                "resolved attack produced no event",
            )
        })?
        .attack_resolved
        .clone();
    Ok(RoguelikeResolutionOutcome {
        d20: resolved.d20,
        ability_modifier: resolved.ability_modifier,
        attack_total: resolved.attack_total,
        defense: resolved.defense,
        hit: resolved.hit,
        damage_rolls: resolved.damage_rolls,
        damage_bonus: resolved.damage_bonus,
        requested_damage: resolved.requested_damage,
        applied_damage: transaction.applied_damage(),
    })
}

/// Runs admit/gather/check before any roll is drawn so illegal attempts fail
/// with the same SessionErrors without consuming a roll (roll parity).
fn preflight(
    policy: &mut RoguelikeResolutionPolicy,
    intent: &RoguelikeIntent,
) -> Result<RoguelikeAdmittedIntent, SessionError> {
    let mut sink = PreflightSink;
    let admitted = policy
        .admit(intent, &[], &mut sink)
        .map_err(policy_failure_error)?;
    let facts = policy
        .gather(&admitted, &[], &mut sink)
        .map_err(policy_failure_error)?;
    policy
        .check(&admitted, &facts, &[], &mut sink)
        .map_err(policy_failure_error)?;
    Ok(admitted)
}

struct PreflightSink;

impl ResolutionTraceSink<RoguelikeTraceDetail> for PreflightSink {
    fn record(&mut self, _detail: RoguelikeTraceDetail) {}
}

/// Builds the bounded roll evidence ids from an attack roll.
fn attack_evidence(
    action_id: &RoguelikeId,
    damage: &DamageDefinition,
    attack_roll: &AttackRoll,
) -> Vec<RoguelikeEvidence> {
    let mut evidence = vec![RoguelikeEvidence {
        id: evidence_d20_id(action_id),
        value: i64::from(attack_roll.d20),
    }];
    for (index, value) in attack_roll.damage.iter().enumerate() {
        evidence.push(RoguelikeEvidence {
            id: evidence_damage_id(action_id, u8::try_from(index).expect("dice count fits u8")),
            value: i64::from(*value),
        });
    }
    debug_assert_eq!(evidence.len(), 1 + usize::from(damage.dice));
    evidence
}

fn evidence_d20_id(action_id: &RoguelikeId) -> String {
    format!("attack.{action_id}.d20")
}

fn evidence_damage_id(action_id: &RoguelikeId, index: u8) -> String {
    format!("attack.{action_id}.damage.{index}")
}

fn policy_failure_error(
    failure: PolicyFailure<RoguelikeRejection, RoguelikeFault, RoguelikeSuspension>,
) -> SessionError {
    match failure {
        PolicyFailure::Rejected(rejection) => rejection_error(rejection),
        PolicyFailure::Fault(fault) => fault_error(fault),
        PolicyFailure::Suspended(_) => error(
            "session_resolution_suspended",
            "attack resolution does not suspend",
        ),
    }
}

fn receipt_failure(receipt: &RoguelikeResolutionReceipt) -> SessionError {
    match receipt.attempt().status() {
        AttemptStatus::Rejected(rejection) => rejection_error(rejection.clone()),
        AttemptStatus::Faulted(fault) => fault_error(fault.clone()),
        AttemptStatus::Suspended(_) => error(
            "session_resolution_suspended",
            "attack resolution does not suspend",
        ),
        AttemptStatus::LimitExceeded(limit) => {
            error("session_resolution_limit", format!("{limit:?}"))
        }
        AttemptStatus::ChildFailed => error(
            "session_resolution_child_failed",
            "attack resolution produced a failed child attempt",
        ),
        AttemptStatus::Planned => match receipt.commit() {
            CommitStatus::Failed(transaction_error) => {
                transaction_error_error(transaction_error.clone())
            }
            CommitStatus::NotAttempted => error(
                "session_resolution_invalid",
                "planned attack was not committed",
            ),
            CommitStatus::Previewed | CommitStatus::Applied => error(
                "session_resolution_invalid",
                "planned attack committed without an event",
            ),
        },
    }
}

fn rejection_error(rejection: RoguelikeRejection) -> SessionError {
    match rejection {
        RoguelikeRejection::UnknownActor { entity_id }
        | RoguelikeRejection::UnknownTarget { entity_id } => {
            error("session_actor_unknown", entity_id.to_string())
        }
        RoguelikeRejection::UnknownAction { action_id } => {
            error("session_action_unknown", action_id)
        }
        RoguelikeRejection::NotOwned { .. } => error(
            "session_action_not_owned",
            "the current actor does not own the selected action",
        ),
        RoguelikeRejection::EquipmentRequired { .. } => error(
            "session_action_equipment_required",
            "the selected action requires its granting item to be equipped",
        ),
        RoguelikeRejection::ActivationCostInvalid { .. } => error(
            "session_action_cost_invalid",
            "Roguelike actions must consume exactly one activation",
        ),
        RoguelikeRejection::PartyTargetModeInvalid { .. } => error(
            "session_party_target_mode_invalid",
            "party actions must target an enemy cell rather than the party square",
        ),
        RoguelikeRejection::OppositionTargetModeInvalid { .. } => error(
            "session_opposition_target_mode_invalid",
            "opposition actions must target the party square",
        ),
        RoguelikeRejection::NotAnAttack { .. } => error(
            "session_action_requires_step",
            "movement actions are issued through a one-cell movement command",
        ),
        RoguelikeRejection::TargetNotOpposition { .. }
        | RoguelikeRejection::TargetNotParticipating { .. }
        | RoguelikeRejection::TargetDead { .. } => error(
            "session_target_not_legal",
            "the target is not a live participating opponent",
        ),
        RoguelikeRejection::TargetNotVisible { .. } => error(
            "session_target_not_visible",
            "the target is outside the Rust world visibility projection",
        ),
        RoguelikeRejection::TargetOutOfRange { .. } => error(
            "session_target_out_of_range",
            "the target is outside the selected action's clear range",
        ),
        RoguelikeRejection::MissingEvidence { id } => {
            error("session_resolution_evidence_missing", id)
        }
        RoguelikeRejection::RollOutOfBounds {
            id,
            value,
            minimum,
            maximum,
        } => error(
            "session_static_roll_incompatible",
            format!("evidence {id} value {value} outside {minimum}..={maximum}"),
        ),
    }
}

fn fault_error(fault: RoguelikeFault) -> SessionError {
    match fault {
        RoguelikeFault::Session { code, detail } => error(code, detail),
    }
}

fn transaction_error_error(transaction_error: RoguelikeTransactionError) -> SessionError {
    match transaction_error {
        RoguelikeTransactionError::SourceInvalid { detail } => {
            error("session_source_invalid", detail)
        }
        RoguelikeTransactionError::DamageInvalid { detail } => {
            error("session_damage_invalid", detail)
        }
        RoguelikeTransactionError::DamageKindInvalid { detail } => {
            error("session_damage_kind_invalid", detail)
        }
        RoguelikeTransactionError::DamageFailed { detail } => {
            error("session_damage_failed", detail)
        }
    }
}

pub(super) fn ability_modifier(score: i16) -> i16 {
    (score - 10).div_euclid(2)
}

impl GameSession {
    pub(super) fn party_decision(&self) -> Result<Option<PartyDecisionView>, SessionError> {
        if self.phase != SessionPhase::Expedition || self.outcome != SessionOutcome::Ongoing {
            return Ok(None);
        }
        let Some(current) = self.current_slot() else {
            return Ok(None);
        };
        if current.side != TurnSide::Party {
            return Ok(None);
        }
        let actor = self.actor(current.entity.raw())?;
        let owns_movement = actor.actions.iter().any(|id| {
            self.world.rules().actions().get(id).is_some_and(|action| {
                action.activation_cost == 1
                    && action.target == ActionTargetCandidate::SelfOnly
                    && matches!(action.effect, ActionEffectDefinition::Movement { steps: 1 })
            })
        });
        let mut legal_steps = Vec::new();
        if owns_movement {
            for step in [
                crate::RelativeStep::Forward,
                crate::RelativeStep::Left,
                crate::RelativeStep::Right,
                crate::RelativeStep::Backward,
            ] {
                let mut staged = self
                    .world
                    .fork()
                    .map_err(|detail| error("session_stage_world", detail.to_string()))?;
                if staged.step(step).is_ok() {
                    legal_steps.push(step);
                }
            }
        }

        let visible = self
            .world
            .view()
            .map_err(|detail| error("session_world_view", detail.to_string()))?;
        let party_position = self
            .world
            .party_position()
            .map_err(|detail| error("session_world_read", detail.to_string()))?;
        let mut actions = Vec::new();
        for action_id in &actor.actions {
            if !self.actor_action_available(actor.entity_id, action_id)? {
                continue;
            }
            let Some(action) = self.world.rules().actions().get(action_id) else {
                continue;
            };
            let ActionEffectDefinition::Attack { range, .. } = action.effect else {
                continue;
            };
            if action.target != ActionTargetCandidate::HostileCell {
                continue;
            }
            let mut legal_target_entity_ids = visible
                .visible_actors
                .iter()
                .filter(|target| {
                    self.world
                        .enemy_position(EntityId::new(target.entity_id))
                        .ok()
                        .and_then(|position| self.world.clear_distance(party_position, position))
                        .is_some_and(|distance| distance <= u32::from(range))
                })
                .map(|target| target.entity_id)
                .collect::<Vec<_>>();
            legal_target_entity_ids.sort_unstable();
            actions.push(LegalActionView {
                action_id: action.id.clone(),
                name: action.name.clone(),
                legal_target_entity_ids,
            });
        }
        Ok(Some(PartyDecisionView {
            actor_entity_id: actor.entity_id,
            expected_revision: self.revision,
            legal_steps,
            can_turn: owns_movement,
            can_wait: true,
            actions,
        }))
    }

    pub(super) fn resolve_party_command(
        &mut self,
        command: SessionCommand,
    ) -> Result<(), SessionError> {
        let actor = self
            .actor(command.actor_entity_id().ok_or_else(|| {
                error(
                    "session_party_command_invalid",
                    "loadout commands are not party activation commands",
                )
            })?)?
            .clone();
        match command {
            SessionCommand::Step { step, .. } => {
                self.require_movement_action(&actor)?;
                self.world
                    .step(step)
                    .map_err(|detail| error("session_party_step_rejected", detail.to_string()))?;
                self.latest_receipts.push(TurnReceipt::PartyMoved {
                    actor_entity_id: actor.entity_id,
                    step,
                });
            }
            SessionCommand::TurnLeft { .. } => {
                self.require_movement_action(&actor)?;
                self.world
                    .turn_left()
                    .map_err(|detail| error("session_party_turn_rejected", detail.to_string()))?;
                self.latest_receipts.push(TurnReceipt::PartyTurned {
                    actor_entity_id: actor.entity_id,
                    direction: PartyTurnDirection::Left,
                });
            }
            SessionCommand::TurnRight { .. } => {
                self.require_movement_action(&actor)?;
                self.world
                    .turn_right()
                    .map_err(|detail| error("session_party_turn_rejected", detail.to_string()))?;
                self.latest_receipts.push(TurnReceipt::PartyTurned {
                    actor_entity_id: actor.entity_id,
                    direction: PartyTurnDirection::Right,
                });
            }
            SessionCommand::Wait { .. } => {
                self.latest_receipts.push(TurnReceipt::PartyWaited {
                    actor_entity_id: actor.entity_id,
                });
            }
            SessionCommand::UseAction {
                action_id,
                target_entity_id,
                ..
            } => self.resolve_party_action(&actor, &action_id, EntityId::new(target_entity_id))?,
            SessionCommand::MoveLoadoutItem { .. } | SessionCommand::BeginExpedition { .. } => {
                return Err(error(
                    "session_party_command_invalid",
                    "loadout commands are not party activation commands",
                ));
            }
        }
        Ok(())
    }

    fn resolve_party_action(
        &mut self,
        actor: &ActorDefinition,
        action_id: &RoguelikeId,
        target: EntityId,
    ) -> Result<(), SessionError> {
        let identity = self.next_resolution_identity()?;
        let operation = self.attack_operation(actor.entity_id, target.raw())?;
        let resolved = resolve_roguelike_attack(
            &mut self.world,
            &mut self.roll,
            identity,
            operation,
            RoguelikeIntent {
                actor_entity_id: actor.entity_id,
                action_id: action_id.clone(),
                target_entity_id: target.raw(),
                origin: RoguelikeIntentOrigin::Party,
            },
        )?;
        self.latest_receipts.push(TurnReceipt::PartyAttacked {
            actor_entity_id: actor.entity_id,
            target_entity_id: target.raw(),
            action_id: action_id.clone(),
            d20: resolved.d20,
            ability_modifier: resolved.ability_modifier,
            attack_total: resolved.attack_total,
            defense: resolved.defense,
            hit: resolved.hit,
            damage_rolls: resolved.damage_rolls,
            damage_bonus: resolved.damage_bonus,
            requested_damage: resolved.requested_damage,
            applied_damage: resolved.applied_damage,
        });
        Ok(())
    }

    fn attack_operation(
        &self,
        actor_entity_id: u64,
        target_entity_id: u64,
    ) -> Result<OperationId, SessionError> {
        OperationId::parse(format!(
            "turn.{}.{}.{}.{}",
            self.round, self.revision, actor_entity_id, target_entity_id
        ))
        .map_err(|detail| error("session_operation_invalid", detail.to_string()))
    }

    fn next_resolution_identity(&mut self) -> Result<ResolutionIdentity, SessionError> {
        let resolution = self.next_attempt;
        self.next_attempt = self.next_attempt.checked_add(1).ok_or_else(|| {
            error(
                "session_resolution_identity_overflow",
                "resolution identity overflowed",
            )
        })?;
        Ok(ResolutionIdentity::root(
            ResolutionId::new(resolution).map_err(|detail| {
                error("session_resolution_identity_invalid", format!("{detail:?}"))
            })?,
            CorrelationId::new(self.round).map_err(|detail| {
                error("session_resolution_identity_invalid", format!("{detail:?}"))
            })?,
        ))
    }

    pub(super) fn resolve_opposition(&mut self, entity: EntityId) -> Result<(), SessionError> {
        let actor = self.actor(entity.raw())?.clone();
        let party = self
            .world
            .party_position()
            .map_err(|detail| error("session_world_read", detail.to_string()))?;
        let origin = self
            .world
            .enemy_position(entity)
            .map_err(|detail| error("session_world_read", detail.to_string()))?;
        let distance = self.world.clear_distance(origin, party);
        let mut legal_attacks = actor
            .actions
            .iter()
            .filter_map(|id| {
                let action = self.world.rules().actions().get(id)?;
                let ActionEffectDefinition::Attack { range, .. } = action.effect else {
                    return None;
                };
                (action.target == ActionTargetCandidate::HostilePartySquare
                    && distance
                        .is_some_and(|distance| distance == 1 && distance <= u32::from(range)))
                .then(|| (id.clone(), action.clone()))
            })
            .collect::<Vec<_>>();
        legal_attacks.sort_by(|left, right| left.0.cmp(&right.0));
        if let Some((action_id, _)) = legal_attacks.into_iter().next() {
            let (target, target_receipt) = self.select_party_member(entity)?;
            let identity = self.next_resolution_identity()?;
            let operation = self.attack_operation(actor.entity_id, target.raw())?;
            let resolved = resolve_roguelike_attack(
                &mut self.world,
                &mut self.roll,
                identity,
                operation,
                RoguelikeIntent {
                    actor_entity_id: actor.entity_id,
                    action_id: action_id.clone(),
                    target_entity_id: target.raw(),
                    origin: RoguelikeIntentOrigin::Opposition,
                },
            )?;
            self.latest_receipts.push(TurnReceipt::OppositionAttacked {
                actor_entity_id: entity.raw(),
                action_id,
                target: target_receipt,
                d20: resolved.d20,
                ability_modifier: resolved.ability_modifier,
                attack_total: resolved.attack_total,
                defense: resolved.defense,
                hit: resolved.hit,
                damage_rolls: resolved.damage_rolls,
                damage_bonus: resolved.damage_bonus,
                requested_damage: resolved.requested_damage,
                applied_damage: resolved.applied_damage,
            });
            return Ok(());
        }

        let can_move = actor.actions.iter().any(|id| {
            self.world.rules().actions().get(id).is_some_and(|action| {
                action.target == ActionTargetCandidate::SelfOnly
                    && matches!(action.effect, ActionEffectDefinition::Movement { steps: 1 })
            })
        });
        let moved = if can_move {
            self.world
                .move_enemy_toward_party(entity)
                .map_err(|detail| error("session_opposition_move_failed", detail.to_string()))?
        } else {
            false
        };
        self.latest_receipts.push(if moved {
            TurnReceipt::OppositionMoved {
                actor_entity_id: entity.raw(),
            }
        } else {
            // An actor with neither a clear authored attack nor an available
            // Engine-routed step explicitly consumes its activation.
            TurnReceipt::OppositionPassed {
                actor_entity_id: entity.raw(),
            }
        });
        Ok(())
    }

    fn select_party_member(
        &mut self,
        attacker: EntityId,
    ) -> Result<(EntityId, PartySquareTargetReceipt), SessionError> {
        let members = self
            .world
            .rules()
            .party()
            .members
            .iter()
            .map(|id| self.world.rules().actors()[id].entity_id)
            .collect::<Vec<_>>();
        let mut eligible_member_count = 0;
        for entity in &members {
            if self.is_alive(EntityId::new(*entity))? {
                eligible_member_count += 1;
            }
        }
        if eligible_member_count == 0 {
            return Err(error(
                "session_party_target_unavailable",
                "no living party member can receive the party-square effect",
            ));
        }
        let start = self
            .target_cursors
            .get(&attacker.raw())
            .copied()
            .unwrap_or_default()
            % members.len();
        for offset in 0..members.len() {
            let index = (start + offset) % members.len();
            let entity = EntityId::new(members[index]);
            if self.is_alive(entity)? {
                self.target_cursors
                    .insert(attacker.raw(), (index + 1) % members.len());
                return Ok((
                    entity,
                    PartySquareTargetReceipt {
                        selected_member_entity_id: entity.raw(),
                        selection_policy: PartyMemberSelectionPolicy::RoundRobinLiving,
                        eligible_member_count: u8::try_from(eligible_member_count)
                            .expect("party size fits u8"),
                    },
                ));
            }
        }
        Err(error(
            "session_party_target_unavailable",
            "no living party member can receive the party-square effect",
        ))
    }

    fn require_movement_action(&self, actor: &ActorDefinition) -> Result<(), SessionError> {
        if actor.actions.iter().any(|id| {
            self.world.rules().actions().get(id).is_some_and(|action| {
                action.activation_cost == 1
                    && matches!(action.effect, ActionEffectDefinition::Movement { steps: 1 })
            })
        }) {
            Ok(())
        } else {
            Err(error(
                "session_movement_not_owned",
                "the current actor has no one-step movement action",
            ))
        }
    }

    pub(super) fn actor(&self, entity_id: u64) -> Result<&ActorDefinition, SessionError> {
        self.world
            .rules()
            .actors()
            .values()
            .find(|actor| actor.entity_id == entity_id)
            .ok_or_else(|| error("session_actor_unknown", entity_id.to_string()))
    }
}
