//! Инварианты эквити: нормировка, симметрия, exact/MC, сплиты.

mod common;

use common::{cards, hand};
use lynx_eval::{equity_vs_hand, equity_vs_hand_mc};

fn close(left: f64, right: f64, epsilon: f64) {
    assert!(
        (left - right).abs() <= epsilon,
        "{left} не равно {right} с точностью {epsilon}"
    );
}

#[test]
fn exact_preflop_is_normalized_and_full() {
    let equity = equity_vs_hand(hand::<2>("As Ah"), hand::<2>("Ks Kd"), &[], &[]).expect("префлоп");

    assert!(equity.exact);
    assert_eq!(equity.samples, 1_712_304, "C(48,5)");
    close(equity.wins + equity.losses + equity.ties, 1.0, 1e-12);
    close(equity.hero_equity() + equity.villain_equity(), 1.0, 1e-12);
    assert!(equity.hero_equity() > equity.villain_equity());
}

#[test]
fn swapping_players_swaps_outcomes() {
    let hero = hand::<2>("As Ah");
    let villain = hand::<2>("Ks Kd");
    let dead = cards("2c 3d");

    let direct = equity_vs_hand(hero, villain, &[], &dead).expect("прямой расчёт");
    let swapped = equity_vs_hand(villain, hero, &[], &dead).expect("обратный расчёт");

    assert_eq!(direct.wins, swapped.losses);
    assert_eq!(direct.losses, swapped.wins);
    assert_eq!(direct.ties, swapped.ties);
    assert_eq!(direct.samples, swapped.samples);
}

#[test]
fn dead_cards_shrink_the_board_space() {
    let equity = equity_vs_hand(hand::<2>("As Ah"), hand::<2>("Ks Kd"), &[], &cards("2c 3d"))
        .expect("префлоп");
    assert_eq!(equity.samples, 1_370_754, "C(46,5)");
}

#[test]
fn river_is_a_single_sample() {
    let board = cards("As Ks Qd 7c 2h");
    let equity =
        equity_vs_hand(hand::<2>("Ah Ad"), hand::<2>("Kc Kd"), &board, &[]).expect("ривер");

    assert_eq!(equity.samples, 1);
    assert_eq!(equity.wins, 1.0);
    assert_eq!(equity.losses, 0.0);
    assert_eq!(equity.ties, 0.0);
    assert_eq!(equity.hero_equity(), 1.0);
}

#[test]
fn board_play_is_a_split() {
    let board = cards("As Ks Qs Js Ts");
    let equity = equity_vs_hand(hand::<2>("2c 3d"), hand::<2>("4c 5d"), &board, &[]).expect("борд");

    assert_eq!(equity.wins, 0.0);
    assert_eq!(equity.losses, 0.0);
    assert_eq!(equity.ties, 1.0);
    assert_eq!(equity.hero_equity(), 0.5);
    assert_eq!(equity.villain_equity(), 0.5);
}

#[test]
fn monte_carlo_matches_exact_on_flop() {
    let board = cards("2c 7d 9h");
    let hero = hand::<2>("As Ah");
    let villain = hand::<2>("Ks Kd");

    let exact = equity_vs_hand(hero, villain, &board, &[]).expect("точный флоп");
    let sampled = equity_vs_hand_mc(hero, villain, &board, &[], 200_000, 42).expect("Монте-Карло");

    assert!(exact.exact);
    assert!(!sampled.exact);
    assert_eq!(sampled.samples, 200_000);
    close(exact.hero_equity(), sampled.hero_equity(), 0.005);
}

#[test]
fn monte_carlo_is_deterministic_by_seed() {
    let board = cards("2c 7d 9h");
    let hero = hand::<2>("As Ah");
    let villain = hand::<2>("Ks Kd");

    let first = equity_vs_hand_mc(hero, villain, &board, &[], 10_000, 7).expect("первый");
    let second = equity_vs_hand_mc(hero, villain, &board, &[], 10_000, 7).expect("второй");
    assert_eq!(first.wins, second.wins);
    assert_eq!(first.losses, second.losses);
    assert_eq!(first.ties, second.ties);
}
