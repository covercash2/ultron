use rand::Rng;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidState(State),
}

/// Parameters for a betting game.
#[derive(Debug)]
pub struct Gamble {
    pub channel_id: u64,
    pub player_id: u64,
    pub amount: i64,
    pub game: Game,
    state: State,
}

/// Different games that are supported
#[derive(Debug)]
pub enum Game {
    /// Rolls two dice with a specified number of sides
    DiceRoll(u32),
}

/// The state of the game
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Waiting,
    Win,
    Lose,
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
    /// Create a new Gamble object.
    /// The initial state is set to `State::Waiting`
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

    /// Play the game a return the results
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
        State::Win
    } else if player_roll == house_roll {
        State::Draw
    } else {
        State::Lose
    };

    GambleOutput::DiceRoll {
        player_id,
        amount,
        house_roll,
        player_roll,
        state,
    }
}
