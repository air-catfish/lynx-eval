//! Невалидный вход: ошибки вместо паник и вместо «тихо неверного числа».

mod common;

use common::{cards, hand};
use lynx_eval::{CardError, EvalError, equity_vs_hand, equity_vs_hand_mc, evaluate};

#[test]
fn card_parse_rejects_bad_text() {
    assert_eq!("A".parse::<lynx_eval::Card>(), Err(CardError::BadLength));
    assert_eq!("".parse::<lynx_eval::Card>(), Err(CardError::BadLength));
    assert_eq!("As ".parse::<lynx_eval::Card>(), Err(CardError::BadLength));
    assert_eq!(
        "1s".parse::<lynx_eval::Card>(),
        Err(CardError::UnknownRank('1'))
    );
    assert_eq!(
        "Ax".parse::<lynx_eval::Card>(),
        Err(CardError::UnknownSuit('x'))
    );
}

#[test]
fn card_parse_is_case_insensitive() {
    assert_eq!(
        "aS".parse::<lynx_eval::Card>(),
        "As".parse::<lynx_eval::Card>()
    );
    assert_eq!(
        "td".parse::<lynx_eval::Card>(),
        "Td".parse::<lynx_eval::Card>()
    );
}

#[test]
fn evaluate_checks_card_count() {
    let four = cards("As Kd Qc Jh");
    let eight = cards("As Kd Qc Jh 9s 8d 7c 6h");
    assert_eq!(evaluate(&four), Err(EvalError::NotEnoughCards));
    assert_eq!(evaluate(&eight), Err(EvalError::TooManyCards));
}

#[test]
fn evaluate_rejects_duplicates() {
    let duplicate = cards("As As Kd Qc Jh");
    assert_eq!(
        evaluate(&duplicate),
        Err(EvalError::DuplicateCard("As".parse().unwrap()))
    );
}

#[test]
fn best_five_rejects_duplicates() {
    let duplicate = cards("As As Kd Qc Jh 9s 8d");
    assert!(matches!(
        lynx_eval::best_five(&duplicate),
        Err(EvalError::DuplicateCard(_))
    ));
}

#[test]
fn equity_rejects_two_card_board() {
    let result = equity_vs_hand(hand::<2>("As Ah"), hand::<2>("Ks Kd"), &cards("2c 7d"), &[]);
    assert_eq!(result, Err(EvalError::BadBoard { cards: 2 }));
}

#[test]
fn equity_rejects_dead_card_collision() {
    let result = equity_vs_hand(hand::<2>("As Ah"), hand::<2>("Ks Kd"), &[], &cards("As"));
    assert_eq!(result, Err(EvalError::DuplicateCard("As".parse().unwrap())));
}

#[test]
fn equity_rejects_hand_collision() {
    let result = equity_vs_hand(hand::<2>("As Ah"), hand::<2>("As Kd"), &[], &[]);
    assert_eq!(result, Err(EvalError::DuplicateCard("As".parse().unwrap())));
}

#[test]
fn monte_carlo_rejects_zero_samples() {
    let result = equity_vs_hand_mc(hand::<2>("As Ah"), hand::<2>("Ks Kd"), &[], &[], 0, 1);
    assert_eq!(result, Err(EvalError::EmptySamples));
}
