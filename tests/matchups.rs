//! Канонические мэтчапы. Значения — точный перебор всех бордов (не Монте-Карло),
//! сверены с независимыми публичными переборами; см. `docs/19-lynx-eval-report.md`.

mod common;

use common::hand;
use lynx_eval::equity_vs_hand;

fn preflop_hero_equity(hero: &str, villain: &str) -> f64 {
    equity_vs_hand(hand::<2>(hero), hand::<2>(villain), &[], &[])
        .expect("префлоп")
        .hero_equity()
}

#[test]
fn aces_vs_kings_one_shared_suit() {
    // A♠A♥ vs K♠K♦: у королей один общий флеш-масть; публичный ориентир 81.95%.
    let equity = preflop_hero_equity("As Ah", "Ks Kd");
    assert!((equity - 0.819_461).abs() < 0.000_5, "получено {equity}");
}

#[test]
fn aces_vs_kings_both_suits_shared() {
    // A♠A♥ vs K♠K♥: оба флеша королей бьются тузом; публичный ориентир 82.64%.
    let equity = preflop_hero_equity("As Ah", "Ks Kh");
    assert!((equity - 0.826_366).abs() < 0.000_5, "получено {equity}");
}

#[test]
fn aces_vs_kings_no_shared_suits() {
    // A♠A♥ vs K♣K♦: у королей две живые флеш-масти; публичный ориентир ~81.3%.
    let equity = preflop_hero_equity("As Ah", "Kc Kd");
    assert!((equity - 0.812_555).abs() < 0.000_5, "получено {equity}");
}

#[test]
fn suited_big_ace_vs_small_pair_is_a_coinflip() {
    // A♣K♣ vs 2♦2♥: публичный ориентир PokerStove — 50.084%.
    let equity = preflop_hero_equity("Ac Kc", "2d 2h");
    assert!((equity - 0.500_842).abs() < 0.000_5, "получено {equity}");
}

#[test]
fn dominated_ace_suited_keeps_three_suits_worth_of_outs() {
    // A♠5♠ vs A♥K♦: доминирован, но флеш и стрит дают ~32%.
    let equity = preflop_hero_equity("As 5s", "Ah Kd");
    assert!((equity - 0.319_475).abs() < 0.000_5, "получено {equity}");
}
