use rand::Rng;

#[derive(Debug)]
pub struct Gamble {
    pub channel_id: u64,
    pub user_id: u64,
    pub amount: i64,
    pub game: Game,
}

#[derive(Debug)]
pub enum Game {
    DiceRoll(u32),
}

pub enum Status {
    Win(i64),
    Lose(i64),
    Draw,
}

impl Gamble {
    pub fn play(&self) -> Status {
	match self.game {
	    Game::DiceRoll(sides) => roll_dice(sides, self.amount),
	}
    }
}

fn roll_dice(sides: u32, amount: i64) -> Status {
    let mut rng = rand::thread_rng();

    let player_roll = rng.gen_range(0, sides);
    let bank_roll = rng.gen_range(0, sides);

    if player_roll > bank_roll {
	Status::Win(amount)
    } else if player_roll == bank_roll {
	Status::Draw
    } else {
	Status::Lose(amount)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_dice_roll() {
	
    }
}
