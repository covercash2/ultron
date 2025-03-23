//! a module for parsing and evaluating dice rolls.
//! similar to other dice rolling libraries, this module
use std::str::FromStr;

use caith::{RollError, Roller};

/// A dice roll that represents a collection of dice
/// that can be rolled together.
/// Example: 2d6 + 3d8
/// This would be a DiceRoll with 2 Die with 6 sides
/// and 3 Die with 8 sides.
/// Advantage and disadvantage can be represented by
/// rolling 2 dice and taking the highest or lowest
/// respectively.
/// Example: 2d20h1 or 2d20l1
/// This would be a DiceRoll with 2 Die with 20 sides
/// and taking the highest or lowest respectively.
#[derive(Debug, Clone, PartialEq, derive_more::Display)]
pub struct DiceRoll {
    result: String,
}

#[derive(Debug, Clone, PartialEq)]
struct RollReport {
    total: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DieRoll {
    sides: u64,
    value: u64,
}

type DiceRollResult<T> = Result<T, DiceRollError>;

#[derive(thiserror::Error, Debug)]
pub enum DiceRollError {
    #[error("failed to parse dice roll from input: {0}")]
    Parse(#[from] RollError),
}

impl FromStr for DiceRoll {
    type Err = DiceRollError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let roll_result = Roller::new(input)?.roll()?;

        Ok(DiceRoll {
            result: format!("{}", roll_result),
        })
    }
}
