use log::*;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Ledger {
    map: HashMap<u64, i64>,
}

impl Ledger {
    /// Increment the user account by `amount`.
    /// This function can be used to decrement the account by passing a negative number.
    pub fn increment_balance(&mut self, user: &u64, amount: i64) {
        match self.map.get_mut(user) {
            // TODO maybe check for overflows here
            Some(balance) => *balance += amount,
            None => {
                if let Some(old_amount) = self.map.insert(*user, amount) {
                    error!("value overwritten for user: {} -- {}", user, old_amount);
                }
            }
        }
    }

    pub fn transfer(&mut self, from_user: &u64, to_user: &u64, amount: i64) {
        self.increment_balance(&from_user, -amount);
        self.increment_balance(&to_user, amount);
    }

    /// Get the balance of the user account or create it and initialize it with 0
    pub fn get_balance(&mut self, user: &u64) -> i64 {
        match self.map.get(user) {
            Some(&amount) => amount,
            None => {
                self.map.insert(*user, 0);
                0
            }
        }
    }

    pub fn get_balances(&mut self, users: Vec<u64>) -> Vec<(u64, i64)> {
        users
            .iter()
            .map(|user| (*user, self.get_balance(user)))
            .collect()
    }

    pub fn get_all_balances(&self) -> Vec<(u64, i64)> {
        self.map
            .iter()
            .map(|(uid, amount)| (*uid, *amount))
            .collect()
    }
}

impl Default for Ledger {
    fn default() -> Self {
	return Ledger {
	    map: HashMap::new(),
	}
    }
}
