#![no_std]

use gstd::{exec, msg};
use pebbles_game_io::*;

// Global variable to store game state
static mut PEBBLES_GAME: Option<GameState> = None;

#[no_mangle]
extern "C" fn init() {
    // Receive initialization message
    let init_message: PebblesInit = msg::load().expect("Can't load init");

    if init_message.pebbles_count == 0 || init_message.max_pebbles_per_turn == 0 {
        panic!("Invalid pebbles_count or max_pebbles_per_turn");
    }

    // Select the first player
    let first_player = get_first_player();

    // Get initial pebbles count
    let pebbles_remaining = get_init_pebbles_remain(
        init_message.pebbles_count,
        init_message.max_pebbles_per_turn,
        first_player.clone(),
        init_message.difficulty.clone(),
    );

    // Fill game state struct
    let initial_state = GameState {
        pebbles_count: init_message.pebbles_count,
        max_pebbles_per_turn: init_message.max_pebbles_per_turn,
        pebbles_remaining,
        difficulty: init_message.difficulty,
        first_player,
        winner: None,
    };

    // Set global variable
    unsafe {
        PEBBLES_GAME = Some(initial_state);
    }
}

#[no_mangle]
extern "C" fn handle() {
    // Receive player action message
    let action: PebblesAction = msg::load().expect("Can't get PebblesAction");
    // Get current game state
    let mut game_state = unsafe { PEBBLES_GAME.take().expect("Can't get GameState") };

    match action {
        PebblesAction::Turn(pebbles_taken) => {
            // Handle player's turn
            if pebbles_taken > game_state.max_pebbles_per_turn || pebbles_taken == 0 {
                panic!("Invalid pebbles taken");
            }

            if pebbles_taken > game_state.pebbles_remaining {
                panic!("Can't get enought pebbles remaining");
            }

            game_state.pebbles_remaining -= pebbles_taken;

            if game_state.pebbles_remaining == 0 {
                game_state.winner = Some(Player::User);
                msg::reply(PebblesEvent::Won(Player::User), 0)
                    .expect("Failed to reply with PebblesEvent - Won");
            } else {
                let counter_pebbles_taken = get_contract_pebbles_taken(
                    game_state.pebbles_remaining,
                    game_state.max_pebbles_per_turn,
                    game_state.difficulty.clone(),
                );

                game_state.pebbles_remaining -= counter_pebbles_taken;

                if game_state.pebbles_remaining == 0 {
                    game_state.winner = Some(Player::Program);
                    msg::reply(PebblesEvent::Won(Player::Program), 0)
                        .expect("Failed to reply with PebblesEvent - Won");
                } else {
                    msg::reply(PebblesEvent::CounterTurn(counter_pebbles_taken), 0)
                        .expect("Failed to reply with PebblesEvent - CounterTurn");
                }
            }
        }
        PebblesAction::GiveUp => {
            // Handle player giving up
            game_state.winner = Some(Player::Program);
            msg::reply(PebblesEvent::Won(Player::Program), 0)
                .expect("Failed to reply with PebblesEvent - Won");
        }
        PebblesAction::Restart {
            difficulty,
            pebbles_count,
            max_pebbles_per_turn,
        } => {
            // Handle game restart
            let first_player = get_first_player();
            let pebbles_remaining = get_init_pebbles_remain(
                pebbles_count,
                max_pebbles_per_turn,
                first_player.clone(),
                difficulty.clone(),
            );

            game_state = GameState {
                pebbles_count,
                max_pebbles_per_turn,
                pebbles_remaining,
                difficulty,
                first_player,
                winner: None,
            };
        }
    }

    // Update global variable
    unsafe {
        PEBBLES_GAME = Some(game_state);
    }
}

#[no_mangle]
extern "C" fn state() {
    // Return current game state
    let game_state = unsafe { PEBBLES_GAME.clone().expect("Can't get GameState") };
    msg::reply(game_state, 0).expect("Failed to reply");
}

// Helper function to get random number, not for testing
#[cfg(not(test))]
fn get_random_u32() -> u32 {
    let salt = msg::id();
    let (hash, _num) = exec::random(salt.into()).expect("get_random_u32(): random call failed");
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}

// For testing only
#[cfg(test)]
fn get_random_u32() -> u32 {
    2
}

// Calculate the number of pebbles the program should take based on game difficulty and remaining pebbles
fn get_contract_pebbles_taken(
    pebbles_remaining: u32,
    max_pebbles_per_turn: u32,
    difficulty: DifficultyLevel,
) -> u32 {
    match difficulty {
        DifficultyLevel::Easy => {
            let random_number = get_random_u32();
            (random_number % max_pebbles_per_turn + 1).min(pebbles_remaining)
        }
        DifficultyLevel::Hard => {
            let optimal_pebbles_taken = pebbles_remaining % (max_pebbles_per_turn + 1);
            if optimal_pebbles_taken == 0 {
                1
            } else {
                optimal_pebbles_taken
            }
        }
    }
}

// Randomly select the first player
fn get_first_player() -> Player {
    let random_number = get_random_u32();
    if random_number % 2 == 0 {
        Player::User
    } else {
        Player::Program
    }
}

// Calculate initial remaining pebbles
fn get_init_pebbles_remain(
    pebbles_count: u32,
    max_pebbles_per_turn: u32,
    first_player: Player,
    difficulty: DifficultyLevel,
) -> u32 {
    let mut pebbles_remaining = pebbles_count;

    if first_player == Player::Program {
        let counter_pebbles_taken =
            get_contract_pebbles_taken(pebbles_count, max_pebbles_per_turn, difficulty.clone());

        pebbles_remaining -= counter_pebbles_taken;
        msg::reply(PebblesEvent::CounterTurn(counter_pebbles_taken), 0)
            .expect("Failed to reply with PebblesEvent - CounterTurn");
    }

    pebbles_remaining
}
