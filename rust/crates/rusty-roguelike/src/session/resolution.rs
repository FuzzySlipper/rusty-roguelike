use core_ids::EntityId;
use gameplay_mechanics::{
    DamageKindId, DamagePart, DamageRequest, DamageService, MechanicsScalar, OperationId,
    SourceInstanceId, SourceInstanceIdentity, StatService,
};

use crate::{
    defense_stat_id, vitality_track_id, ActionEffectDefinition, ActorDefinition,
    ActorSideCandidate, RoguelikeId,
};

use super::runtime::{error, GameSession};
use super::{PartyCommand, SessionError, TurnReceipt};

impl GameSession {
    pub(super) fn resolve_party_command(
        &mut self,
        command: PartyCommand,
    ) -> Result<(), SessionError> {
        let actor = self.actor(command.actor_entity_id())?.clone();
        match command {
            PartyCommand::Step { step, .. } => {
                self.require_movement_action(&actor)?;
                self.world
                    .step(step)
                    .map_err(|detail| error("session_party_step_rejected", detail.to_string()))?;
                self.latest_receipts.push(TurnReceipt::PartyMoved {
                    actor_entity_id: actor.entity_id,
                });
            }
            PartyCommand::TurnLeft { .. } => {
                self.require_movement_action(&actor)?;
                self.world
                    .turn_left()
                    .map_err(|detail| error("session_party_turn_rejected", detail.to_string()))?;
                self.latest_receipts.push(TurnReceipt::PartyTurned {
                    actor_entity_id: actor.entity_id,
                });
            }
            PartyCommand::TurnRight { .. } => {
                self.require_movement_action(&actor)?;
                self.world
                    .turn_right()
                    .map_err(|detail| error("session_party_turn_rejected", detail.to_string()))?;
                self.latest_receipts.push(TurnReceipt::PartyTurned {
                    actor_entity_id: actor.entity_id,
                });
            }
            PartyCommand::UseAction {
                action_id,
                target_entity_id,
                ..
            } => self.resolve_party_action(&actor, &action_id, EntityId::new(target_entity_id))?,
        }
        Ok(())
    }

    fn resolve_party_action(
        &mut self,
        actor: &ActorDefinition,
        action_id: &RoguelikeId,
        target: EntityId,
    ) -> Result<(), SessionError> {
        if !actor.actions.contains(action_id) {
            return Err(error(
                "session_action_not_owned",
                "the current actor does not own the selected action",
            ));
        }
        let action = self
            .world
            .rules()
            .actions()
            .get(action_id)
            .ok_or_else(|| error("session_action_unknown", action_id.to_string()))?
            .clone();
        if action.activation_cost != 1 {
            return Err(error(
                "session_action_cost_invalid",
                "Roguelike actions must consume exactly one activation",
            ));
        }
        let ActionEffectDefinition::Attack {
            ability,
            defense,
            damage,
            range,
        } = action.effect
        else {
            return Err(error(
                "session_action_requires_step",
                "movement actions are issued through a one-cell movement command",
            ));
        };
        let target_actor = self.actor(target.raw())?.clone();
        if target_actor.side != ActorSideCandidate::Opposition
            || !self
                .world
                .participating_enemies()
                .map_err(|detail| error("session_world_read", detail.to_string()))?
                .contains(&target)
            || !self.is_alive(target)?
        {
            return Err(error(
                "session_target_not_legal",
                "the target is not a live participating opponent",
            ));
        }
        if !self
            .world
            .view()
            .map_err(|detail| error("session_world_view", detail.to_string()))?
            .visible_actors
            .iter()
            .any(|visible| visible.entity_id == target.raw())
        {
            return Err(error(
                "session_target_not_visible",
                "the target is outside the Rust world visibility projection",
            ));
        }
        let party_position = self
            .world
            .party_position()
            .map_err(|detail| error("session_world_read", detail.to_string()))?;
        let target_position = self
            .world
            .enemy_position(target)
            .map_err(|detail| error("session_world_read", detail.to_string()))?;
        if self
            .world
            .clear_distance(party_position, target_position)
            .is_none_or(|distance| distance > u32::from(range))
        {
            return Err(error(
                "session_target_out_of_range",
                "the target is outside the selected action's clear range",
            ));
        }

        let roll = self.roll.attack(damage.dice, damage.sides)?;
        let ability_score = actor
            .abilities
            .iter()
            .find(|score| score.ability == ability)
            .ok_or_else(|| error("session_ability_missing", ability.to_string()))?
            .score;
        let attack_total = i16::from(roll.d20) + ability_modifier(ability_score);
        let operation = OperationId::parse(format!(
            "turn.{}.{}.{}.{}",
            self.round, self.revision, actor.entity_id, target_actor.entity_id
        ))
        .map_err(|detail| error("session_operation_invalid", detail.to_string()))?;
        let catalog = self.world.rules().mechanics().clone();
        let defense_value = StatService::evaluate(
            self.world.entities(),
            &catalog,
            target,
            &defense_stat_id(&defense),
            &operation,
            &[],
        )
        .map_err(|detail| error("session_defense_evaluation_failed", detail.to_string()))?
        .value
        .get();
        let defense_value = i16::try_from(defense_value)
            .map_err(|_| error("session_defense_out_of_range", "defense does not fit i16"))?;
        let hit = attack_total >= defense_value;
        let rolled_damage = roll
            .damage
            .iter()
            .map(|value| i64::from(*value))
            .sum::<i64>()
            .saturating_add(i64::from(damage.bonus))
            .max(0);
        let requested_damage = if hit { rolled_damage } else { 0 };
        let applied_damage = if requested_damage > 0 {
            let instance = SourceInstanceId::parse(format!("action.{action_id}"))
                .map_err(|detail| error("session_source_invalid", detail.to_string()))?;
            let source = SourceInstanceIdentity::Request {
                operation: operation.clone(),
                instance,
            };
            let receipt = DamageService::apply(
                self.world.entities_mut(),
                &catalog,
                DamageRequest {
                    operation,
                    source,
                    actor: Some(EntityId::new(actor.entity_id)),
                    target,
                    target_track: vitality_track_id(),
                    parts: vec![DamagePart {
                        amount: MechanicsScalar::new(requested_damage).map_err(|detail| {
                            error("session_damage_invalid", detail.to_string())
                        })?,
                        kind: DamageKindId::parse(format!("damage.{}", damage.kind)).map_err(
                            |detail| error("session_damage_kind_invalid", detail.to_string()),
                        )?,
                    }],
                    request_sources: vec![],
                    expected_tracks_revision: None,
                },
            )
            .map_err(|detail| error("session_damage_failed", detail.to_string()))?;
            receipt
                .parts
                .iter()
                .map(|part| part.applied.get())
                .sum::<i64>()
        } else {
            0
        };
        self.latest_receipts.push(TurnReceipt::PartyAttacked {
            actor_entity_id: actor.entity_id,
            target_entity_id: target.raw(),
            action_id: action_id.clone(),
            d20: roll.d20,
            attack_total,
            defense: defense_value,
            hit,
            damage: u16::try_from(applied_damage).unwrap_or(u16::MAX),
        });
        Ok(())
    }

    pub(super) fn resolve_opposition(&mut self, entity: EntityId) -> Result<(), SessionError> {
        let party = self
            .world
            .party_position()
            .map_err(|detail| error("session_world_read", detail.to_string()))?;
        let origin = self
            .world
            .enemy_position(entity)
            .map_err(|detail| error("session_world_read", detail.to_string()))?;
        let moved = if self
            .world
            .clear_distance(origin, party)
            .is_some_and(|distance| distance > 1)
        {
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
            // Enemy-to-party-member target selection lands in #6490. Until then an
            // adjacent enemy has no authored legal action, and its activation is
            // explicitly consumed so the deterministic round cannot deadlock.
            TurnReceipt::OppositionPassed {
                actor_entity_id: entity.raw(),
            }
        });
        Ok(())
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

    fn actor(&self, entity_id: u64) -> Result<&ActorDefinition, SessionError> {
        self.world
            .rules()
            .actors()
            .values()
            .find(|actor| actor.entity_id == entity_id)
            .ok_or_else(|| error("session_actor_unknown", entity_id.to_string()))
    }
}

fn ability_modifier(score: i16) -> i16 {
    (score - 10).div_euclid(2)
}
