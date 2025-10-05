//! a module for parsing and evaluating dice rolls.
//! similar to other dice rolling libraries, this module
use std::{fmt::Display, str::FromStr};

use rmcp::schemars;
use serde::{Deserialize, Serialize};
use tyche::{
    Expr,
    dice::{self, Roller},
    expr::{Describe, Evaled},
};

use crate::{
    command::Command,
    event_processor::{BotMessage, Event, EventConsumer, EventResult},
};

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

pub trait RollerImpl: tyche::dice::Roller + std::fmt::Debug + Clone + Send + Sync {}

impl RollerImpl for dice::roller::FastRand {}
impl RollerImpl for dice::roller::Max {}

/// a cloneable dice roller that controls the RNG
#[derive(Debug, Clone)]
pub struct DiceRoller<TInner> {
    inner: TInner,
}

impl Default for DiceRoller<dice::roller::FastRand> {
    fn default() -> Self {
        Self::with_default_rng()
    }
}

impl<TInner> DiceRoller<TInner>
where
    TInner: RollerImpl,
{
    pub fn roll_expr(mut self, expr: Expr) -> Result<Evaled<'static>, DiceRollError> {
        Ok(expr.eval(&mut self.inner)?.into_owned())
    }

    pub fn roll_string(self, input: &str) -> Result<Evaled<'static>, DiceRollError> {
        let expr = Expr::from_str(input)?;
        self.roll_expr(expr)
    }
}

impl<TRoller> From<TRoller> for DiceRoller<TRoller>
where
    TRoller: RollerImpl,
{
    fn from(inner: TRoller) -> Self {
        Self { inner }
    }
}

impl DiceRoller<dice::roller::FastRand> {
    pub fn with_rng(seed: u64) -> Self {
        Self::from(dice::roller::FastRand::with_seed(seed))
    }

    pub fn with_default_rng() -> Self {
        Self::from(dice::roller::FastRand::default())
    }
}

impl DiceRoller<dice::roller::Max> {
    pub fn max() -> Self {
        Self::from(dice::roller::Max::default())
    }
}

pub fn roller<T>() -> T
where
    T: Roller + Default,
{
    Default::default()
}

#[cfg(test)]
pub fn test_roller() -> dice::roller::Max {
    roller()
}

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
///
/// [`Evaled`] shows the individual rolls
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DiceRoll {
    #[schemars(description = "the evaluated expression of the dice roll")]
    evaluated_expression: String,
    #[schemars(description = "the total of the dice roll")]
    total: i32,
}

impl TryFrom<Evaled<'_>> for DiceRoll {
    type Error = DiceRollError;

    fn try_from(evaled: Evaled<'_>) -> Result<Self, Self::Error> {
        let total = evaled.calc()?;
        let dice_limit = None;
        let evaluated_expression = evaled.describe(dice_limit);
        Ok(Self {
            evaluated_expression,
            total,
        })
    }
}

impl Display for DiceRoll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_{}_ = **{}**", self.evaluated_expression, self.total)
    }
}

#[derive(Debug, Clone)]
pub enum DiceRollResult {
    Roll(DiceRoll),
    Help(&'static str),
}

impl DiceRollResult {
    pub fn from_str<TRoller>(
        input: &str,
        roller: DiceRoller<TRoller>,
    ) -> Result<Self, DiceRollError>
    where
        TRoller: RollerImpl,
    {
        if input == "help" {
            tracing::debug!("sending help message");
            return Ok(DiceRollResult::Help(HELP_MESSAGE));
        }

        let expr = Expr::from_str(input)?;
        let result = roller.roll_expr(expr)?;

        let dice_roll: DiceRoll = result.try_into()?;

        tracing::debug!("computed roll: {dice_roll}");
        Ok(DiceRollResult::Roll(dice_roll))
    }
}

impl Display for DiceRollResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiceRollResult::Roll(roll) => write!(f, "{}", roll),
            DiceRollResult::Help(message) => write!(f, "{}", message),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DiceRollError {
    #[error("failed to parse dice roll from input: {0}")]
    Parse(#[from] tyche::parse::Error),

    #[error("failed to evaluate dice roll: {0}")]
    Eval(#[from] tyche::expr::EvalError),

    #[error("failed to calculate dice roll: {0}")]
    Calc(#[from] tyche::expr::CalcError),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// known good rolls
    const GOOD_ROLLS: &[&str] = &[
        // Copilot made all these lol
        "d20",
        "2d20",
        "d6 + d8",
        "2d20K1",
        "2d20k1",
        "4d6K3",
        "4d6k3",
        "1d4 + 1d6 + 1d8 + 1d10 + 1d12 + 1d20",
        "3d6 + 2d8 - 1d4",
        "5d10 * 2",
        "(2d6 + 3) / 2",
        "2d20kh1 + 1d4", // keep highest
        "2d20kl1 + 1d4", // keep lowest
        // from the tycho docs
        "4d6rr<3 + 2d8 - 4",
        "4d6 + 2d8 - 2",
    ];

    #[test]
    fn simple_roll() {
        let roller = DiceRoller::max();
        for &roll in GOOD_ROLLS {
            let result =
                DiceRollResult::from_str(roll, roller.clone()).expect("failed to parse roll");
            if let DiceRollResult::Roll(dice_roll) = result {
                tracing::info!("roll: {roll} => {dice_roll}");
            } else {
                panic!("expected roll, got help");
            }
        }
    }
}
