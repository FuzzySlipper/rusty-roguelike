use core_ids::EntityId;
use gameplay_mechanics::{
    DamageKindId, DamagePart, DamageRequest, DamageService, MechanicsScalar, OperationId,
    SourceInstanceId, SourceInstanceIdentity, StatService, TracksComponent,
};

use crate::{
    defense_stat_id, vitality_track_id, ActionDefinition, ActionEffectDefinition,
    ActionTargetCandidate, ActorBuildComponent, ActorDefinition, ActorSideCandidate, RoguelikeId,
};

use super::runtime::{error, GameSession};
use super::{
    CarriedItemView, LegalActionView, PartyCommand, PartyDecisionView, PartyMemberSelectionPolicy,
    PartyMemberStatusView, PartySquareTargetReceipt, PartyTurnDirection, SessionError,
    SessionOutcome, TurnReceipt, TurnSide,
};

struct ResolvedAttack {
    d20: u8,
    ability_modifier: i16,
    attack_total: i16,
    defense: i16,
    hit: bool,
    damage_rolls: Vec<u16>,
    damage_bonus: i16,
    requested_damage: u16,
    applied_damage: u16,
}

impl GameSession {
    pub(super) fn party_status(&self) -> Result<Vec<PartyMemberStatusView>, SessionError> {
        self.world
            .rules()
            .party()
            .members
            .iter()
            .map(|actor_id| {
                let actor = &self.world.rules().actors()[actor_id];
                let entity = EntityId::new(actor.entity_id);
                let tracks = self
                    .world
                    .entities()
                    .component::<TracksComponent>(entity)
                    .map_err(|detail| error("session_tracks_read", detail.to_string()))?
                    .ok_or_else(|| error("session_tracks_missing", format!("entity {entity}")))?;
                let current = tracks
                    .current(&vitality_track_id())
                    .ok_or_else(|| error("session_vitality_missing", format!("entity {entity}")))?
                    .get();
                let build = self
                    .world
                    .entities()
                    .component::<ActorBuildComponent>(entity)
                    .map_err(|detail| error("session_build_read", detail.to_string()))?
                    .ok_or_else(|| error("session_build_missing", format!("entity {entity}")))?;
                let operation =
                    OperationId::parse(format!("view.party.{}.{}", actor.entity_id, self.revision))
                        .map_err(|detail| error("session_operation_invalid", detail.to_string()))?;
                let maximum = StatService::evaluate(
                    self.world.entities(),
                    self.world.rules().mechanics(),
                    entity,
                    &crate::vitality_maximum_stat_id(),
                    &operation,
                    &[],
                )
                .map_err(|detail| error("session_vitality_evaluation_failed", detail.to_string()))?
                .value
                .get();
                Ok(PartyMemberStatusView {
                    entity_id: actor.entity_id,
                    actor_id: actor.id.clone(),
                    name: actor.name.clone(),
                    current_vitality: u16::try_from(current.max(0)).map_err(|_| {
                        error(
                            "session_vitality_out_of_range",
                            "current vitality exceeds u16",
                        )
                    })?,
                    maximum_vitality: u16::try_from(maximum.max(0)).map_err(|_| {
                        error(
                            "session_vitality_out_of_range",
                            "maximum vitality exceeds u16",
                        )
                    })?,
                    conscious: current > 0,
                    carried_items: build
                        .items()
                        .iter()
                        .map(|item_id| CarriedItemView {
                            item_id: item_id.clone(),
                            name: self.world.rules().items()[item_id].name.clone(),
                        })
                        .collect(),
                })
            })
            .collect()
    }

    pub(super) fn party_decision(&self) -> Result<Option<PartyDecisionView>, SessionError> {
        if self.outcome != SessionOutcome::Ongoing {
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
        let actions = actor
            .actions
            .iter()
            .filter_map(|action_id| {
                let action = self.world.rules().actions().get(action_id)?;
                let ActionEffectDefinition::Attack { range, .. } = action.effect else {
                    return None;
                };
                if action.target != ActionTargetCandidate::HostileCell {
                    return None;
                }
                let mut legal_target_entity_ids = visible
                    .visible_actors
                    .iter()
                    .filter(|target| {
                        self.world
                            .enemy_position(EntityId::new(target.entity_id))
                            .ok()
                            .and_then(|position| {
                                self.world.clear_distance(party_position, position)
                            })
                            .is_some_and(|distance| distance <= u32::from(range))
                    })
                    .map(|target| target.entity_id)
                    .collect::<Vec<_>>();
                legal_target_entity_ids.sort_unstable();
                Some(LegalActionView {
                    action_id: action.id.clone(),
                    name: action.name.clone(),
                    legal_target_entity_ids,
                })
            })
            .collect::<Vec<_>>();
        Ok(Some(PartyDecisionView {
            actor_entity_id: actor.entity_id,
            expected_revision: self.revision,
            legal_steps,
            can_turn: owns_movement,
            actions,
        }))
    }

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
                    step,
                });
            }
            PartyCommand::TurnLeft { .. } => {
                self.require_movement_action(&actor)?;
                self.world
                    .turn_left()
                    .map_err(|detail| error("session_party_turn_rejected", detail.to_string()))?;
                self.latest_receipts.push(TurnReceipt::PartyTurned {
                    actor_entity_id: actor.entity_id,
                    direction: PartyTurnDirection::Left,
                });
            }
            PartyCommand::TurnRight { .. } => {
                self.require_movement_action(&actor)?;
                self.world
                    .turn_right()
                    .map_err(|detail| error("session_party_turn_rejected", detail.to_string()))?;
                self.latest_receipts.push(TurnReceipt::PartyTurned {
                    actor_entity_id: actor.entity_id,
                    direction: PartyTurnDirection::Right,
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
        if action.target != ActionTargetCandidate::HostileCell {
            return Err(error(
                "session_party_target_mode_invalid",
                "party actions must target an enemy cell rather than the party square",
            ));
        }
        let ActionEffectDefinition::Attack { range, .. } = &action.effect else {
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
            .is_none_or(|distance| distance > u32::from(*range))
        {
            return Err(error(
                "session_target_out_of_range",
                "the target is outside the selected action's clear range",
            ));
        }

        let resolved = self.resolve_attack(actor, target, action_id, &action)?;
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

    fn resolve_attack(
        &mut self,
        actor: &ActorDefinition,
        target: EntityId,
        action_id: &RoguelikeId,
        action: &ActionDefinition,
    ) -> Result<ResolvedAttack, SessionError> {
        let ActionEffectDefinition::Attack {
            ability,
            defense,
            damage,
            ..
        } = &action.effect
        else {
            return Err(error(
                "session_action_not_attack",
                "attack resolution received a non-attack action",
            ));
        };
        let roll = self.roll.attack(damage.dice, damage.sides)?;
        let ability_score = actor
            .abilities
            .iter()
            .find(|score| &score.ability == ability)
            .ok_or_else(|| error("session_ability_missing", ability.to_string()))?
            .score;
        let ability_modifier = ability_modifier(ability_score);
        let attack_total = i16::from(roll.d20) + ability_modifier;
        let operation = OperationId::parse(format!(
            "turn.{}.{}.{}.{}",
            self.round,
            self.revision,
            actor.entity_id,
            target.raw()
        ))
        .map_err(|detail| error("session_operation_invalid", detail.to_string()))?;
        let catalog = self.world.rules().mechanics().clone();
        let defense_value = StatService::evaluate(
            self.world.entities(),
            &catalog,
            target,
            &defense_stat_id(defense),
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
        Ok(ResolvedAttack {
            d20: roll.d20,
            ability_modifier,
            attack_total,
            defense: defense_value,
            hit,
            damage_rolls: roll.damage,
            damage_bonus: damage.bonus,
            requested_damage: u16::try_from(requested_damage)
                .expect("compiled attack damage fits u16"),
            applied_damage: u16::try_from(applied_damage).expect("applied damage fits u16"),
        })
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
                    && distance.is_some_and(|distance| distance <= u32::from(range)))
                .then(|| (id.clone(), action.clone()))
            })
            .collect::<Vec<_>>();
        legal_attacks.sort_by(|left, right| left.0.cmp(&right.0));
        if let Some((action_id, action)) = legal_attacks.into_iter().next() {
            let (target, target_receipt) = self.select_party_member(entity)?;
            let resolved = self.resolve_attack(&actor, target, &action_id, &action)?;
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
