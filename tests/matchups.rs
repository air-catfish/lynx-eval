//! Канонические мэтчапы. Эталонные значения — из независимых публичных
//! переборов (проверено вручную, см. `docs/19-lynx-eval-report.md`).

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
    // A♠A♥ vs K♠K♦: у королей один общий флеш-масть; ориентир 81.95%.
    let equity = preflop_hero_equity("As Ah", "Ks Kd");
    println!("AA vs KK (одна общая масть): {equity:.6}");
    assert!((equity - 0.8195).abs() < 0.001, "получено {equity}");
}

#[test]
fn aces_vs_kings_both_suits_shared() {
    // A♠A♥ vs K♠K♥: оба флеша королей бьются тузом; ориентир 82.64%.
    let equity = preflop_hero_equity("As Ah", "Ks Kh");
    println!("AA vs KK (обе масти общие): {equity:.6}");
    assert!((equity - 0.8264).abs() < 0.001, "получено {equity}");
}

#[test]
fn aces_vs_kings_no_shared_suits() {
    // A♠A♥ vs K♣K♦: у королей две живые флеш-масти; ориентир ~81.3%.
    let equity = preflop_hero_equity("As Ah", "Kc Kd");
    println!("AA vs KK (без общих мастей): {equity:.6}");
    assert!((equity - 0.8133).abs() < 0.005, "получено {equity}");
}

#[test]
fn suited_big_ace_vs_small_pair_is_a_coinflip() {
    // A♣K♣ vs 2♦2♥: классический «бросок монеты».
    let equity = preflop_hero_equity("Ac Kc", "2d 2h");
    println!("AKs vs 22: {equity:.6}");
    assert!((equity - 0.50).abs() < 0.01, "получено {equity}");
}

#[test]
fn dominated_ace_suited_keeps_three_suits_worth_of_outs() {
    // A♠5♠ vs A♥K♦: A5s доминирован, но флеш и стрит дают ~31%.
    let equity = preflop_hero_equity("As 5s", "Ah Kd");
    println!("A5s vs AKo: {equity:.6}");
    assert!((equity - 0.31).abs() < 0.02, "получено {equity}");
}
