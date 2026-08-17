use rusty_engine::core_ids::EntityId;
use rusty_engine::gameplay_mechanics::{
    DamageKindId, DamagePart, DamageRequest, DamageService, MechanicsCatalog, MechanicsScalar,
    OperationId, SourceInstanceId, SourceInstanceIdentity,
};
use rusty_engine::gameplay_resolution::ResolutionTransaction;

use crate::{vitality_track_id, RoguelikeId, WorldState};

use super::{RoguelikeEffect, RoguelikeTransactionError};

/// The fail-atomic attack transaction. `stage` only collects typed effects;
/// `commit` applies staged damage through DamageService to the staged world
/// and records the actually-applied total. The session command already
/// operates on a fork, so any commit error discards the fork and leaves the
/// authoritative session untouched.
pub(super) struct RoguelikeTransaction<'a> {
    world: &'a mut WorldState,
    mechanics: MechanicsCatalog,
    operation: OperationId,
    actor_entity_id: u64,
    action_id: RoguelikeId,
    damage_kind: RoguelikeId,
    staged: Vec<RoguelikeEffect>,
    applied_damage: u16,
}

impl<'a> RoguelikeTransaction<'a> {
    pub(super) fn new(
        world: &'a mut WorldState,
        mechanics: MechanicsCatalog,
        operation: OperationId,
        actor_entity_id: u64,
        action_id: RoguelikeId,
        damage_kind: RoguelikeId,
    ) -> Self {
        Self {
            world,
            mechanics,
            operation,
            actor_entity_id,
            action_id,
            damage_kind,
            staged: Vec::new(),
            applied_damage: 0,
        }
    }

    pub(super) fn applied_damage(&self) -> u16 {
        self.applied_damage
    }
}

impl ResolutionTransaction for RoguelikeTransaction<'_> {
    type Effect = RoguelikeEffect;
    type Error = RoguelikeTransactionError;

    fn stage(&mut self, effect: &RoguelikeEffect) -> Result<(), RoguelikeTransactionError> {
        self.staged.push(effect.clone());
        Ok(())
    }

    fn commit(&mut self) -> Result<(), RoguelikeTransactionError> {
        let mut applied_total: u64 = 0;
        for effect in &self.staged {
            let (target_entity_id, amount) = match effect {
                RoguelikeEffect::Damage {
                    target_entity_id,
                    amount,
                } => (*target_entity_id, *amount),
            };
            if amount == 0 {
                continue;
            }
            let operation = self.operation.clone();
            let instance = SourceInstanceId::parse(format!("action.{}", self.action_id)).map_err(
                |detail| RoguelikeTransactionError::SourceInvalid {
                    detail: detail.to_string(),
                },
            )?;
            let source = SourceInstanceIdentity::Request {
                operation: operation.clone(),
                instance,
            };
            let receipt = DamageService::apply(
                self.world.entities_mut(),
                &self.mechanics,
                DamageRequest {
                    operation,
                    source,
                    actor: Some(EntityId::new(self.actor_entity_id)),
                    target: EntityId::new(target_entity_id),
                    target_track: vitality_track_id(),
                    parts: vec![DamagePart {
                        amount: MechanicsScalar::new(i64::from(amount)).map_err(|detail| {
                            RoguelikeTransactionError::DamageInvalid {
                                detail: detail.to_string(),
                            }
                        })?,
                        kind: DamageKindId::parse(format!("damage.{}", self.damage_kind)).map_err(
                            |detail| RoguelikeTransactionError::DamageKindInvalid {
                                detail: detail.to_string(),
                            },
                        )?,
                    }],
                    request_sources: vec![],
                    expected_tracks_revision: None,
                },
            )
            .map_err(|detail| RoguelikeTransactionError::DamageFailed {
                detail: detail.to_string(),
            })?;
            applied_total += receipt
                .parts
                .iter()
                .map(|part| part.applied.get())
                .sum::<i64>() as u64;
        }
        self.applied_damage = u16::try_from(applied_total).expect("applied damage fits u16");
        self.staged.clear();
        Ok(())
    }

    fn abort(&mut self) {
        self.staged.clear();
    }
}
