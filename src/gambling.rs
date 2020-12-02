use rand::Rng;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidState(State),
}

/// Parameters for a betting game.
#[derive(Debug)]
pub struct Gamble {
    pub player_id: u64,
    pub prize: Prize,
    pub game: Game,
    state: State,
}

#[derive(Debug, Clone)]
pub enum Prize {
    Coins(i64),
    AllCoins,
}

impl From<i64> for Prize {
    fn from(n: i64) -> Self {
	Prize::Coins(n)
    }
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
        prize: Prize,
        house_roll: u32,
        player_roll: u32,
        state: State,
    },
}

impl Gamble {
    /// Create a new Gamble object.
    /// The initial state is set to `State::Waiting`
    pub fn new(player_id: u64, prize: Prize, game: Game) -> Self {
        let state = State::Waiting;

        Gamble {
            player_id,
	    prize,
            game,
            state,
        }
    }

    /// Play the game a return the results
    pub fn play(self) -> Result<GambleOutput> {
        match self.game {
            Game::DiceRoll(sides) => match self.state {
                State::Waiting => {
                    let output = play_dice(self.player_id, self.prize, sides);
                    Ok(output)
                }
                _ => Err(Error::InvalidState(self.state.clone())),
            },
        }
    }
}

fn play_dice(player_id: u64, prize: Prize, sides: u32) -> GambleOutput {
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
        prize,
        house_roll,
        player_roll,
        state,
    }
}
