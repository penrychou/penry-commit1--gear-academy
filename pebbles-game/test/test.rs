use gstd::Encode;
use gtest::{Program, System};
use pebbles_game::PebbleGame;
use pebbles_game_io::*;

const USERS: &[u64] = &[3, 4, 5];

#[cfg(test)]
pub fn get_random_u32() -> u32 {
    use getrandom::getrandom;
    let mut buffer = [0u8; 4];
    getrandom(&mut buffer).expect("Failed to generate random number");
    u32::from_ne_bytes(buffer)
}

fn init_game(sys: &System, total: u32, turn_max: u32) {
    sys.init_logger();

    let game = Program::current_opt(sys);
    let res = game.send(
        USERS[0],
        PebblesInit {
            pebbles_count: total,
            max_pebbles_per_turn: turn_max,
            difficulty: DifficultyLevel::Easy,
        },
    );

    assert!(!res.main_failed());

    let gm: PebbleGame = game.read_state(0).expect("Invalid state.");
    assert_eq!(gm.pebbles_count, total);
    assert_eq!(gm.max_pebbles_per_turn, turn_max);
    match gm.first_player {
        Player::User => assert_eq!(gm.pebbles_count, gm.pebbles_remaining),
        Player::Program => assert_eq!(gm.pebbles_count, gm.pebbles_remaining + gm.program_lastmove),
    }
}

#[test]
fn init_successed() {
    let sys = System::new();
    sys.init_logger();

    let game = Program::current_opt(&sys);
    let res = game.send(
        USERS[0],
        PebblesInit {
            pebbles_count: 10,
            max_pebbles_per_turn: 9,
            difficulty: DifficultyLevel::Easy,
        },
    );
    assert!(!res.main_failed());
}

#[test]
fn init_failed() {
    let sys = System::new();
    sys.init_logger();

    let game = Program::current_opt(&sys);
    let res = game.send(
        USERS[0],
        PebblesInit {
            pebbles_count: 10,
            max_pebbles_per_turn: 11,
            difficulty: DifficultyLevel::Easy,
        },
    );
    assert!(res.main_failed());
}

#[test]
fn restart() {
    let sys = System::new();
    init_game(&sys, 3, 1);
    let game = sys.get_program(1).unwrap();
    let res = game.send(
        USERS[0],
        PebblesAction::Restart {
            difficulty: DifficultyLevel::Easy,
            pebbles_count: 50,
            max_pebbles_per_turn: 3,
        },
    );
    assert!(!res.main_failed());
    let gmstate: PebbleGame = game.read_state(0).expect("Invalid state.");
    assert_eq!(gmstate.pebbles_count, 50);
    assert_eq!(gmstate.max_pebbles_per_turn, 3);
}

#[test]
fn user_move() {
    let sys = System::new();
    init_game(&sys, 101, 3);
    let game = sys.get_program(1).unwrap();

    for _ in 0..100 {
        game.send(
            USERS[0],
            PebblesAction::Restart {
                difficulty: DifficultyLevel::Easy,
                pebbles_count: 101,
                max_pebbles_per_turn: 10,
            },
        );
        let mut count = get_random_u32() % 10;
        count += 1;
        let gmstate1: PebbleGame = game.read_state(0).expect("Invalid state.");
        let remaining = gmstate1.pebbles_remaining;
        let res = game.send(USERS[0], PebblesAction::Turn(count));
        let gmstate2: PebbleGame = game.read_state(0).expect("Invalid state.");
        assert!(res.contains(&(
            USERS[0],
            PebblesEvent::CounterTurn(gmstate2.program_lastmove).encode()
        )));
        assert_eq!(
            gmstate2.pebbles_remaining,
            remaining - count - gmstate2.program_lastmove
        );
    }
}

#[test]
fn user_move_failed() {
    let sys = System::new();
    init_game(&sys, 5, 2);
    let game = sys.get_program(1).unwrap();

    let res = game.send(USERS[0], PebblesAction::Turn(0));
    assert!(res.main_failed());
    let res = game.send(USERS[0], PebblesAction::Turn(3));
    assert!(res.main_failed());
}
#[test]
fn user_move_failed2() {
    let sys2 = System::new();
    init_game(&sys2, 3, 2);

    let game = sys2.get_program(1).unwrap();
    // restart 找到第一个是程序移动两个的情况, 到用户选择时只有1个可选
    loop {
        let gmstate: PebbleGame = game.read_state(0).expect("Invalid state.");
        if gmstate.program_lastmove == 2 {
            break;
        }
        game.send(
            USERS[0],
            PebblesAction::Restart {
                difficulty: DifficultyLevel::Easy,
                pebbles_count: 3,
                max_pebbles_per_turn: 2,
            },
        );
    }
    let res = game.send(USERS[0], PebblesAction::Turn(2));
    assert!(res.main_failed());
}

#[test]
fn program_move() {
    let sys = System::new();
    init_game(&sys, 99, 3);
    let game = sys.get_program(1).unwrap();

    for _ in 0..100 {
        game.send(
            USERS[0],
            PebblesAction::Restart {
                difficulty: DifficultyLevel::Easy,
                pebbles_count: 100,
                max_pebbles_per_turn: 5,
            },
        );

        let gmstate: PebbleGame = game.read_state(0).expect("Invalid state.");
        let remaing = gmstate.pebbles_remaining;
        let res = game.send(USERS[0], PebblesAction::GiveUp);
        let gmstate2: PebbleGame = game.read_state(0).expect("Invalid state.");
        assert!(res.contains(&(
            USERS[0],
            PebblesEvent::CounterTurn(gmstate2.program_lastmove).encode()
        )));
        assert_eq!(
            gmstate2.pebbles_remaining,
            remaing - gmstate2.program_lastmove
        );
    }
}

#[test]
fn winner() {
    let sys = System::new();
    init_game(&sys, 3, 1);
    let game = sys.get_program(1).unwrap();

    for _ in 0..100 {
        game.send(
            USERS[0],
            PebblesAction::Restart {
                difficulty: DifficultyLevel::Easy,
                pebbles_count: 3,
                max_pebbles_per_turn: 1,
            },
        );
        let gmstate: PebbleGame = game.read_state(0).expect("Invalid state.");
        let remaing = gmstate.pebbles_remaining;
        if remaing < 3 {
            let res = game.send(USERS[0], PebblesAction::Turn(1));
            assert!(res.contains(&(USERS[0], PebblesEvent::Won(Player::Program).encode())));
        } else {
            let res = game.send(USERS[0], PebblesAction::Turn(1));
            assert!(res.contains(&(USERS[0], PebblesEvent::CounterTurn(1).encode())));
            let res = game.send(USERS[0], PebblesAction::Turn(1));
            assert!(res.contains(&(USERS[0], PebblesEvent::Won(Player::User).encode())));
        }
    }
}
