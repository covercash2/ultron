use rand::Rng;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidState(State),
}

#[derive(Debug)]
pub struct Gamble {
    pub channel_id: u64,
    pub user_id: u64,
    pub amount: i64,
    pub game: Game,
    state: State
}

#[derive(Debug)]
pub enum Game {
    DiceRoll(u32),
}

#[derive(Debug, Clone)]
pub enum State {
    Waiting,
    Win(i64),
    Lose(i64),
    Draw,
}

impl Gamble {
    pub fn new(channel_id: u64, user_id: u64, amount: i64, game: Game) -> Self {
	let state = State::Waiting;

	Gamble {
	    channel_id,
	    user_id,
	    amount,
	    game,
	    state
	}
    }

    pub fn play(&mut self) -> Result<&State> {
	match self.game {
	    Game::DiceRoll(sides) => {
		match self.state {
		    State::Waiting => {
			self.state = roll_dice(sides, self.amount);
			Ok(&self.state)
		    }
		    _ => {
			Err(Error::InvalidState(self.state.clone()))
		    }
		}
	    },
	}
    }
}

fn roll_dice(sides: u32, amount: i64) -> State {
    let mut rng = rand::thread_rng();

    let player_roll = rng.gen_range(0, sides);
    let bank_roll = rng.gen_range(0, sides);

    if player_roll > bank_roll {
	State::Win(amount)
    } else if player_roll == bank_roll {
	State::Draw
    } else {
	State::Lose(amount)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_dice_roll() {
	
    }
}
