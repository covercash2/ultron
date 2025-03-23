//! a module for parsing and evaluating dice rolls.
//! similar to other dice rolling libraries, this module
use std::str::FromStr;

use caith::{RollError, Roller};

pub const HELP_MESSAGE: &str = r#"
roll a d20: `d20`
roll _2_ d20s: `2d20`
roll a d6 and a d8: `d6 + d8`
roll with advantage: `2d20K1` (k for "keep")
roll with disadvantage: `2d20k1`
keep the highest 3 of 4d6: `4d6K3`

input is passed as is to the `caith` crate:
https://docs.rs/caith/4.2.4/caith/#syntax
"#;

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

type DiceRollResult<T> = Result<T, DiceRollError>;

#[derive(thiserror::Error, Debug)]
pub enum DiceRollError {
    #[error("failed to parse dice roll from input: {0}")]
    Parse(#[from] RollError),
}

impl FromStr for DiceRoll {
    type Err = DiceRollError;

    fn from_str(input: &str) -> DiceRollResult<Self> {
        if input == "help" {
            tracing::debug!("sending help message");
            return Ok(DiceRoll {
                result: HELP_MESSAGE.to_string(),
            });
        }

        let roll_result = Roller::new(input)?.roll()?;

        tracing::debug!("computed roll: {}", roll_result);
        Ok(DiceRoll {
            result: format!("{}", roll_result),
        })
    }
}
