use svc_rng::{RngSeed, ScopedRng};

use crate::{RollPolicyCandidate, RollPolicyKindCandidate, StaticRollCandidate};

use super::SessionError;

#[derive(Debug, Clone)]
pub(super) enum RollSource {
    Seeded {
        seed: u64,
        next_roll: u64,
    },
    Static {
        rolls: Vec<StaticRollCandidate>,
        next_roll: u64,
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
            RollPolicyKindCandidate::Seeded => Ok(Self::Seeded {
                seed: policy.seed.ok_or_else(|| {
                    error("session_roll_policy_invalid", "seeded policy has no seed")
                })?,
                next_roll: 0,
            }),
            RollPolicyKindCandidate::Static => Ok(Self::Static {
                rolls: policy.rolls.clone(),
                next_roll: 0,
            }),
        }
    }

    pub(super) fn restore(
        policy: &RollPolicyCandidate,
        next_roll: u64,
    ) -> Result<Self, SessionError> {
        let mut source = Self::new(policy)?;
        match &mut source {
            Self::Seeded {
                next_roll: cursor, ..
            } => *cursor = next_roll,
            Self::Static {
                rolls,
                next_roll: cursor,
            } => {
                if usize::try_from(next_roll)
                    .ok()
                    .is_none_or(|value| value > rolls.len())
                {
                    return Err(error(
                        "session_static_roll_position_invalid",
                        "the saved static roll position exceeds the admitted tape",
                    ));
                }
                *cursor = next_roll;
            }
        }
        Ok(source)
    }

    pub(super) const fn next_roll(&self) -> u64 {
        match self {
            Self::Seeded { next_roll, .. } | Self::Static { next_roll, .. } => *next_roll,
        }
    }

    pub(super) fn attack(&mut self, dice: u8, sides: u16) -> Result<AttackRoll, SessionError> {
        match self {
            Self::Seeded { seed, next_roll } => {
                let mut rng = ScopedRng::new(
                    RngSeed::new(*seed),
                    &format!("rusty-roguelike/session-action/{next_roll}"),
                );
                let roll = AttackRoll {
                    d20: (rng.next_bounded_u32(20).expect("fixed positive bound") + 1) as u8,
                    damage: (0..dice)
                        .map(|_| {
                            (rng.next_bounded_u32(u32::from(sides))
                                .expect("admitted die")
                                + 1) as u16
                        })
                        .collect(),
                };
                *next_roll = next_roll.checked_add(1).ok_or_else(|| {
                    error("session_roll_position_overflow", "roll position overflowed")
                })?;
                Ok(roll)
            }
            Self::Static { rolls, next_roll } => {
                let index = usize::try_from(*next_roll).map_err(|_| {
                    error(
                        "session_static_rolls_exhausted",
                        "the static roll position does not fit memory",
                    )
                })?;
                let roll = rolls.get(index).ok_or_else(|| {
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
                *next_roll = next_roll.checked_add(1).ok_or_else(|| {
                    error("session_roll_position_overflow", "roll position overflowed")
                })?;
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
