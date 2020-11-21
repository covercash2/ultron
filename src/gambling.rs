use rand::Rng;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidState(State),
}

#[derive(Debug)]
pub struct Gamble {
    pub channel_id: u64,
    pub player_id: u64,
    pub amount: i64,
    pub game: Game,
    state: State,
}

#[derive(Debug)]
pub enum Game {
    DiceRoll(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Waiting,
    Win(i64),
    Lose(i64),
    Draw,
}

#[derive(Debug)]
pub enum GambleOutput {
    DiceRoll {
        player_id: u64,
        amount: i64,
        house_roll: u32,
        player_roll: u32,
        state: State,
    },
}

impl Gamble {
    pub fn new(channel_id: u64, player_id: u64, amount: i64, game: Game) -> Self {
        let state = State::Waiting;

        Gamble {
            channel_id,
            player_id,
            amount,
            game,
            state,
        }
    }

    pub fn play(&mut self) -> Result<GambleOutput> {
        match self.game {
            Game::DiceRoll(sides) => match self.state {
                State::Waiting => {
                    let output = play_dice(self.player_id, self.amount, sides);
                    Ok(output)
                }
                _ => Err(Error::InvalidState(self.state.clone())),
            },
        }
    }
}

fn play_dice(player_id: u64, amount: i64, sides: u32) -> GambleOutput {
    let mut rng = rand::thread_rng();

    let player_roll = rng.gen_range(0, sides);
    let house_roll = rng.gen_range(0, sides);

    let state = if player_roll > house_roll {
        State::Win(amount)
    } else if player_roll == house_roll {
        State::Draw
    } else {
        State::Lose(amount)
    };

    GambleOutput::DiceRoll {
        player_id,
        amount,
        house_roll,
        player_roll,
        state,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_dice_roll() {}
}
