use svc_rng::{RngSeed, ScopedRng};

use crate::{RollPolicyCandidate, RollPolicyKindCandidate, StaticRollCandidate};

use super::SessionError;

#[derive(Debug, Clone)]
pub(super) enum RollSource {
    Seeded(ScopedRng),
    Static {
        rolls: Vec<StaticRollCandidate>,
        cursor: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AttackRoll {
    pub d20: u8,
    pub damage: Vec<u16>,
}

impl RollSource {
    pub(super) fn new(policy: &RollPolicyCandidate) -> Result<Self, SessionError> {
        match policy.kind {
            RollPolicyKindCandidate::Seeded => Ok(Self::Seeded(ScopedRng::new(
                RngSeed::new(policy.seed.ok_or_else(|| {
                    error("session_roll_policy_invalid", "seeded policy has no seed")
                })?),
                "rusty-roguelike/session-actions",
            ))),
            RollPolicyKindCandidate::Static => Ok(Self::Static {
                rolls: policy.rolls.clone(),
                cursor: 0,
            }),
        }
    }

    pub(super) fn attack(&mut self, dice: u8, sides: u16) -> Result<AttackRoll, SessionError> {
        match self {
            Self::Seeded(rng) => Ok(AttackRoll {
                d20: (rng.next_bounded_u32(20).expect("fixed positive bound") + 1) as u8,
                damage: (0..dice)
                    .map(|_| {
                        (rng.next_bounded_u32(u32::from(sides))
                            .expect("admitted die")
                            + 1) as u16
                    })
                    .collect(),
            }),
            Self::Static { rolls, cursor } => {
                let roll = rolls.get(*cursor).ok_or_else(|| {
                    error(
                        "session_static_rolls_exhausted",
                        "the admitted static roll sequence has no next result",
                    )
                })?;
                if roll.damage.len() != usize::from(dice)
                    || roll.damage.iter().any(|value| *value > sides)
                {
                    return Err(error(
                        "session_static_roll_incompatible",
                        "the next static result does not match the selected action dice",
                    ));
                }
                *cursor += 1;
                Ok(AttackRoll {
                    d20: roll.d20,
                    damage: roll.damage.clone(),
                })
            }
        }
    }
}

fn error(code: &'static str, detail: impl Into<String>) -> SessionError {
    SessionError::new(code, detail)
}
