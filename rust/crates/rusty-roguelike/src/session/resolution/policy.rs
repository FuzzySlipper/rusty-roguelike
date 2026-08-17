use std::collections::BTreeSet;

use rusty_engine::core_ids::EntityId;
use rusty_engine::gameplay_mechanics::{
    EquipmentComponent, ItemComponent, StatService, TracksComponent,
};
use rusty_engine::gameplay_resolution::{
    PolicyFailure, PolicyProgram, PolicyResult, Program, ResolutionPlan, ResolutionPolicy,
    ResolutionTraceSink,
};

use crate::{
    defense_stat_id, item_definition_id, vitality_track_id, ActionEffectDefinition,
    ActionTargetCandidate, ActorDefinition, ActorSideCandidate, RoguelikeId, WorldState,
};

use super::{
    ability_modifier, evidence_d20_id, evidence_damage_id, RoguelikeAdmittedIntent,
    RoguelikeAttackResolved, RoguelikeEffect, RoguelikeEvent, RoguelikeEvidence, RoguelikeFacts,
    RoguelikeFault, RoguelikeIntent, RoguelikeIntentOrigin, RoguelikeInterceptor,
    RoguelikeOperation, RoguelikePredicate, RoguelikeRejection, RoguelikeSuspension,
    RoguelikeTargetFacts, RoguelikeTraceDetail,
};

/// The downstream attack policy. It reads an immutable snapshot of the world
/// (never the live authoritative state) and plans typed effects/events.
pub(super) struct RoguelikeResolutionPolicy {
    snapshot: WorldState,
    operation: rusty_engine::gameplay_mechanics::OperationId,
}

impl RoguelikeResolutionPolicy {
    pub(super) fn new(
        snapshot: WorldState,
        operation: rusty_engine::gameplay_mechanics::OperationId,
    ) -> Self {
        Self {
            snapshot,
            operation,
        }
    }

    fn actor(&self, entity_id: u64) -> Option<&ActorDefinition> {
        self.snapshot
            .rules()
            .actors()
            .values()
            .find(|actor| actor.entity_id == entity_id)
    }

    fn is_alive(&self, entity_id: u64) -> Result<bool, RoguelikeFault> {
        let entity = EntityId::new(entity_id);
        let tracks = self
            .snapshot
            .entities()
            .component::<TracksComponent>(entity)
            .map_err(|detail| fault("session_tracks_read", detail.to_string()))?
            .ok_or_else(|| fault("session_tracks_missing", format!("entity {entity}")))?;
        Ok(tracks
            .current(&vitality_track_id())
            .is_some_and(|value| value.get() > 0))
    }

    /// Replicates the session's equipment/class availability rule against the
    /// snapshot; party actors only (opposition attacks never check
    /// availability, matching the previous manual flow).
    fn action_available(
        &self,
        actor: &ActorDefinition,
        action_id: &RoguelikeId,
    ) -> Result<bool, RoguelikeFault> {
        let class = &self.snapshot.rules().classes()[&actor.class];
        if class
            .levels
            .iter()
            .take(usize::from(actor.class_level))
            .any(|level| level.actions.contains(action_id))
        {
            return Ok(true);
        }
        let required = self
            .snapshot
            .rules()
            .items()
            .values()
            .filter(|item| item.grants_action.as_ref() == Some(action_id))
            .map(|item| item_definition_id(&item.id))
            .collect::<BTreeSet<_>>();
        if required.is_empty() {
            return Ok(false);
        }
        let entity = EntityId::new(actor.entity_id);
        let equipment = self
            .snapshot
            .entities()
            .component::<EquipmentComponent>(entity)
            .map_err(|detail| fault("session_equipment_read", detail.to_string()))?
            .ok_or_else(|| fault("session_equipment_missing", format!("entity {entity}")))?;
        for assignment in equipment.assignments() {
            let component = self
                .snapshot
                .entities()
                .component::<ItemComponent>(assignment.item)
                .map_err(|detail| fault("session_item_read", detail.to_string()))?
                .ok_or_else(|| {
                    fault(
                        "session_item_missing",
                        format!("entity {}", assignment.item),
                    )
                })?;
            if required.contains(component.definition()) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn fault(code: &'static str, detail: impl Into<String>) -> RoguelikeFault {
    RoguelikeFault::Session {
        code,
        detail: detail.into(),
    }
}

fn evidence_value(
    evidence: &[RoguelikeEvidence],
    id: &str,
    minimum: i64,
    maximum: i64,
) -> PolicyResult<i64, RoguelikeRejection, RoguelikeFault, RoguelikeSuspension> {
    let value = evidence
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.value)
        .ok_or_else(|| {
            PolicyFailure::Rejected(RoguelikeRejection::MissingEvidence { id: id.to_string() })
        })?;
    if !(minimum..=maximum).contains(&value) {
        return Err(PolicyFailure::Rejected(
            RoguelikeRejection::RollOutOfBounds {
                id: id.to_string(),
                value,
                minimum,
                maximum,
            },
        ));
    }
    Ok(value)
}

impl ResolutionPolicy for RoguelikeResolutionPolicy {
    type RawIntent = RoguelikeIntent;
    type Intent = RoguelikeAdmittedIntent;
    type Facts = RoguelikeFacts;
    type Predicate = RoguelikePredicate;
    type Operation = RoguelikeOperation;
    type Effect = RoguelikeEffect;
    type Event = RoguelikeEvent;
    type Evidence = RoguelikeEvidence;
    type Interceptor = RoguelikeInterceptor;
    type TraceDetail = RoguelikeTraceDetail;
    type Rejection = RoguelikeRejection;
    type Fault = RoguelikeFault;
    type Suspension = RoguelikeSuspension;

    fn admit(
        &mut self,
        intent: &RoguelikeIntent,
        _evidence: &[RoguelikeEvidence],
        trace: &mut dyn ResolutionTraceSink<RoguelikeTraceDetail>,
    ) -> PolicyResult<
        RoguelikeAdmittedIntent,
        RoguelikeRejection,
        RoguelikeFault,
        RoguelikeSuspension,
    > {
        let actor = self
            .actor(intent.actor_entity_id)
            .cloned()
            .ok_or(PolicyFailure::Rejected(RoguelikeRejection::UnknownActor {
                entity_id: intent.actor_entity_id,
            }))?;
        if !actor.actions.contains(&intent.action_id) {
            return Err(PolicyFailure::Rejected(RoguelikeRejection::NotOwned {
                action_id: intent.action_id.to_string(),
            }));
        }
        if intent.origin == RoguelikeIntentOrigin::Party
            && !self
                .action_available(&actor, &intent.action_id)
                .map_err(PolicyFailure::Fault)?
        {
            return Err(PolicyFailure::Rejected(
                RoguelikeRejection::EquipmentRequired {
                    action_id: intent.action_id.to_string(),
                },
            ));
        }
        let action = self
            .snapshot
            .rules()
            .actions()
            .get(&intent.action_id)
            .cloned()
            .ok_or_else(|| {
                PolicyFailure::Rejected(RoguelikeRejection::UnknownAction {
                    action_id: intent.action_id.to_string(),
                })
            })?;
        if action.activation_cost != 1 {
            return Err(PolicyFailure::Rejected(
                RoguelikeRejection::ActivationCostInvalid {
                    action_id: intent.action_id.to_string(),
                },
            ));
        }
        match intent.origin {
            RoguelikeIntentOrigin::Party => {
                if action.target != ActionTargetCandidate::HostileCell {
                    return Err(PolicyFailure::Rejected(
                        RoguelikeRejection::PartyTargetModeInvalid {
                            action_id: intent.action_id.to_string(),
                        },
                    ));
                }
            }
            RoguelikeIntentOrigin::Opposition => {
                if action.target != ActionTargetCandidate::HostilePartySquare {
                    return Err(PolicyFailure::Rejected(
                        RoguelikeRejection::OppositionTargetModeInvalid {
                            action_id: intent.action_id.to_string(),
                        },
                    ));
                }
            }
        }
        if !matches!(action.effect, ActionEffectDefinition::Attack { .. }) {
            return Err(PolicyFailure::Rejected(RoguelikeRejection::NotAnAttack {
                action_id: intent.action_id.to_string(),
            }));
        }
        if self.actor(intent.target_entity_id).is_none() {
            return Err(PolicyFailure::Rejected(RoguelikeRejection::UnknownTarget {
                entity_id: intent.target_entity_id,
            }));
        }
        trace.record(RoguelikeTraceDetail::Decision {
            reason: format!("admitted action {} for actor {}", action.id, actor.id),
        });
        Ok(RoguelikeAdmittedIntent {
            actor,
            action,
            target_entity_id: intent.target_entity_id,
            origin: intent.origin,
        })
    }

    fn gather(
        &mut self,
        intent: &RoguelikeAdmittedIntent,
        _evidence: &[RoguelikeEvidence],
        trace: &mut dyn ResolutionTraceSink<RoguelikeTraceDetail>,
    ) -> PolicyResult<RoguelikeFacts, RoguelikeRejection, RoguelikeFault, RoguelikeSuspension> {
        let ActionEffectDefinition::Attack {
            ability, defense, ..
        } = &intent.action.effect
        else {
            return Err(PolicyFailure::Fault(fault(
                "session_action_requires_step",
                "non-attack action reached attack gathering",
            )));
        };
        let ability_score = intent
            .actor
            .abilities
            .iter()
            .find(|score| &score.ability == ability)
            .map(|score| score.score)
            .ok_or_else(|| {
                PolicyFailure::Fault(fault("session_ability_missing", ability.to_string()))
            })?;
        let catalog = self.snapshot.rules().mechanics().clone();
        let defense_evaluation = StatService::evaluate(
            self.snapshot.entities(),
            &catalog,
            EntityId::new(intent.target_entity_id),
            &defense_stat_id(defense),
            &self.operation,
            &[],
        )
        .map_err(|detail| {
            PolicyFailure::Fault(fault(
                "session_defense_evaluation_failed",
                detail.to_string(),
            ))
        })?;
        let defense_value = i16::try_from(defense_evaluation.value.get()).map_err(|_| {
            PolicyFailure::Fault(fault(
                "session_defense_out_of_range",
                "defense does not fit i16",
            ))
        })?;
        let target =
            self.actor(intent.target_entity_id)
                .cloned()
                .ok_or(PolicyFailure::Rejected(RoguelikeRejection::UnknownTarget {
                    entity_id: intent.target_entity_id,
                }))?;
        let alive = self
            .is_alive(intent.target_entity_id)
            .map_err(PolicyFailure::Fault)?;
        let (visible, participating, distance) = match intent.origin {
            RoguelikeIntentOrigin::Party => {
                let view = self.snapshot.view().map_err(|detail| {
                    PolicyFailure::Fault(fault("session_world_view", detail.to_string()))
                })?;
                let visible = view
                    .visible_actors
                    .iter()
                    .any(|entry| entry.entity_id == intent.target_entity_id);
                let participating = self
                    .snapshot
                    .participating_enemies()
                    .map_err(|detail| {
                        PolicyFailure::Fault(fault("session_world_read", detail.to_string()))
                    })?
                    .contains(&EntityId::new(intent.target_entity_id));
                let party_position = self.snapshot.party_position().map_err(|detail| {
                    PolicyFailure::Fault(fault("session_world_read", detail.to_string()))
                })?;
                // A non-enemy target has no enemy placement; treat that as an
                // unknown distance so the legality checks reject it with the
                // classified target error rather than a world-read fault.
                let distance = match self
                    .snapshot
                    .enemy_position(EntityId::new(intent.target_entity_id))
                {
                    Ok(target_position) => self
                        .snapshot
                        .clear_distance(party_position, target_position),
                    Err(_) => None,
                };
                (visible, participating, distance)
            }
            RoguelikeIntentOrigin::Opposition => {
                let party_position = self.snapshot.party_position().map_err(|detail| {
                    PolicyFailure::Fault(fault("session_world_read", detail.to_string()))
                })?;
                let origin_position = self
                    .snapshot
                    .enemy_position(EntityId::new(intent.actor.entity_id))
                    .map_err(|detail| {
                        PolicyFailure::Fault(fault("session_world_read", detail.to_string()))
                    })?;
                let distance = self
                    .snapshot
                    .clear_distance(origin_position, party_position);
                (false, false, distance)
            }
        };
        trace.record(RoguelikeTraceDetail::Decision {
            reason: format!(
                "gathered ability {} defense {} for target {}",
                ability_score, defense_value, target.id
            ),
        });
        Ok(RoguelikeFacts {
            ability_score,
            defense_value,
            target: RoguelikeTargetFacts {
                side: target.side,
                alive,
                visible,
                participating,
                distance,
            },
        })
    }

    fn check(
        &mut self,
        intent: &RoguelikeAdmittedIntent,
        facts: &RoguelikeFacts,
        _evidence: &[RoguelikeEvidence],
        trace: &mut dyn ResolutionTraceSink<RoguelikeTraceDetail>,
    ) -> PolicyResult<(), RoguelikeRejection, RoguelikeFault, RoguelikeSuspension> {
        match intent.origin {
            RoguelikeIntentOrigin::Party => {
                if facts.target.side != ActorSideCandidate::Opposition {
                    return Err(PolicyFailure::Rejected(
                        RoguelikeRejection::TargetNotOpposition {
                            entity_id: intent.target_entity_id,
                        },
                    ));
                }
                if !facts.target.participating {
                    return Err(PolicyFailure::Rejected(
                        RoguelikeRejection::TargetNotParticipating {
                            entity_id: intent.target_entity_id,
                        },
                    ));
                }
                if !facts.target.alive {
                    return Err(PolicyFailure::Rejected(RoguelikeRejection::TargetDead {
                        entity_id: intent.target_entity_id,
                    }));
                }
                if !facts.target.visible {
                    return Err(PolicyFailure::Rejected(
                        RoguelikeRejection::TargetNotVisible {
                            entity_id: intent.target_entity_id,
                        },
                    ));
                }
                let ActionEffectDefinition::Attack { range, .. } = &intent.action.effect else {
                    return Err(PolicyFailure::Fault(fault(
                        "session_action_requires_step",
                        "non-attack action reached attack checking",
                    )));
                };
                if facts
                    .target
                    .distance
                    .is_none_or(|distance| distance > u32::from(*range))
                {
                    return Err(PolicyFailure::Rejected(
                        RoguelikeRejection::TargetOutOfRange {
                            entity_id: intent.target_entity_id,
                        },
                    ));
                }
            }
            RoguelikeIntentOrigin::Opposition => {
                if facts.target.side != ActorSideCandidate::Party {
                    return Err(PolicyFailure::Rejected(
                        RoguelikeRejection::TargetNotOpposition {
                            entity_id: intent.target_entity_id,
                        },
                    ));
                }
                if !facts.target.alive {
                    return Err(PolicyFailure::Rejected(RoguelikeRejection::TargetDead {
                        entity_id: intent.target_entity_id,
                    }));
                }
            }
        }
        trace.record(RoguelikeTraceDetail::Decision {
            reason: "attack legality checks passed".to_string(),
        });
        Ok(())
    }

    fn plan(
        &mut self,
        intent: &RoguelikeAdmittedIntent,
        _facts: &RoguelikeFacts,
        _evidence: &[RoguelikeEvidence],
        _trace: &mut dyn ResolutionTraceSink<RoguelikeTraceDetail>,
    ) -> PolicyResult<PolicyProgram<Self>, RoguelikeRejection, RoguelikeFault, RoguelikeSuspension>
    {
        let ActionEffectDefinition::Attack {
            ability,
            defense,
            damage,
            range,
        } = &intent.action.effect
        else {
            return Err(PolicyFailure::Fault(fault(
                "session_action_requires_step",
                "non-attack action reached attack planning",
            )));
        };
        Ok(Program::Operation(RoguelikeOperation::Attack {
            ability: ability.clone(),
            defense: defense.clone(),
            damage: damage.clone(),
            range: *range,
        }))
    }

    fn evaluate_predicate(
        &mut self,
        predicate: &RoguelikePredicate,
        _intent: &RoguelikeAdmittedIntent,
        _facts: &RoguelikeFacts,
        _evidence: &[RoguelikeEvidence],
        _trace: &mut dyn ResolutionTraceSink<RoguelikeTraceDetail>,
    ) -> PolicyResult<bool, RoguelikeRejection, RoguelikeFault, RoguelikeSuspension> {
        match *predicate {}
    }

    fn plan_operation(
        &mut self,
        operation: &RoguelikeOperation,
        intent: &RoguelikeAdmittedIntent,
        facts: &RoguelikeFacts,
        evidence: &[RoguelikeEvidence],
        trace: &mut dyn ResolutionTraceSink<RoguelikeTraceDetail>,
    ) -> PolicyResult<
        ResolutionPlan<RoguelikeEffect, RoguelikeEvent, RoguelikeIntent, RoguelikeEvidence>,
        RoguelikeRejection,
        RoguelikeFault,
        RoguelikeSuspension,
    > {
        let mut plan = ResolutionPlan::new();
        let RoguelikeOperation::Attack { damage, .. } = operation;
        let d20 = evidence_value(evidence, &evidence_d20_id(&intent.action.id), 1, 20)?;
        let mut damage_rolls = Vec::with_capacity(usize::from(damage.dice));
        for index in 0..damage.dice {
            let value = evidence_value(
                evidence,
                &evidence_damage_id(&intent.action.id, index),
                1,
                i64::from(damage.sides),
            )?;
            damage_rolls.push(u16::try_from(value).expect("bounded evidence fits u16"));
        }
        let ability_modifier = ability_modifier(facts.ability_score);
        let attack_total = i16::try_from(d20).expect("bounded d20 fits i16") + ability_modifier;
        let hit = attack_total >= facts.defense_value;
        let rolled_damage = damage_rolls
            .iter()
            .map(|value| i64::from(*value))
            .sum::<i64>()
            .saturating_add(i64::from(damage.bonus))
            .max(0);
        let requested_damage = if hit {
            u16::try_from(rolled_damage).expect("compiled attack damage fits u16")
        } else {
            0
        };
        plan.push_effect(RoguelikeEffect::Damage {
            target_entity_id: intent.target_entity_id,
            amount: requested_damage,
        });
        plan.push_event(RoguelikeEvent {
            attack_resolved: RoguelikeAttackResolved {
                d20: u8::try_from(d20).expect("bounded d20 fits u8"),
                ability_modifier,
                attack_total,
                defense: facts.defense_value,
                hit,
                damage_rolls,
                damage_bonus: damage.bonus,
                requested_damage,
            },
        });
        trace.record(RoguelikeTraceDetail::Decision {
            reason: format!(
                "d20 {d20} + {ability_modifier} vs {} => hit={hit}, requested={requested_damage}",
                facts.defense_value
            ),
        });
        Ok(plan)
    }
}
