//! Диапазоны: нотация, блокаторы, веса, exact/MC.

mod common;

use common::{cards, hand};
use lynx_eval::{
    EvalError, Range, RangeError, equity_vs_hand, equity_vs_range, equity_vs_range_mc,
};

fn combo(text: &str) -> [lynx_eval::Card; 2] {
    hand::<2>(text)
}

#[test]
fn parses_class_counts() {
    assert_eq!(Range::parse("AA").unwrap().combo_count(&[]), 6);
    assert_eq!(Range::parse("AKs").unwrap().combo_count(&[]), 4);
    assert_eq!(Range::parse("AKo").unwrap().combo_count(&[]), 12);
    assert_eq!(Range::parse("22+").unwrap().combo_count(&[]), 13 * 6);
    assert_eq!(Range::parse("A2s+").unwrap().combo_count(&[]), 12 * 4);
    assert_eq!(Range::parse("K9o+").unwrap().combo_count(&[]), 4 * 12);
}

#[test]
fn matrix_is_ui_order() {
    let aa = Range::parse("AA").expect("AA").matrix();
    assert_eq!(aa[12 * 13 + 12], 1000);
    let aks = Range::parse("AKs").expect("AKs").matrix();
    assert_eq!(aks[12 * 13 + 11], 1000, "AKs выше диагонали");
    let ako = Range::parse("AKo").expect("AKo").matrix();
    assert_eq!(ako[11 * 13 + 12], 1000, "AKo ниже диагонали");
    let twos = Range::parse("22").expect("22").matrix();
    assert_eq!(twos[0], 1000);
}

#[test]
fn parses_weights_and_empty() {
    let range = Range::parse("AA:0.5, KK:100%").expect("диапазон");
    assert_eq!(range.combo_count(&[]), 12);
    assert!(!range.is_empty());
    let zero = Range::parse("AA:0").expect("нулевой вес");
    assert!(zero.is_empty());
}

#[test]
fn rejects_bad_notation() {
    assert!(matches!(Range::parse("AK"), Err(RangeError::BadClass(_))));
    assert!(matches!(Range::parse("AXs"), Err(RangeError::BadClass(_))));
    assert!(matches!(
        Range::parse("AA:150%"),
        Err(RangeError::BadWeight(_))
    ));
    assert!(matches!(
        Range::parse("AA:abc"),
        Err(RangeError::BadWeight(_))
    ));
    assert_eq!(Range::parse("   "), Err(RangeError::Empty));
    assert!(matches!(Range::parse("AAx"), Err(RangeError::BadClass(_))));
}

#[test]
fn blockers_remove_combos() {
    let range = Range::parse("AA").expect("AA");
    let dead = cards("As Ah");
    assert_eq!(range.combo_count(&dead), 1, "остаётся AdAc");
}

#[test]
fn one_remaining_combo_matches_hand_equity() {
    let hero = combo("As Ah");
    let board = cards("2c 7d 9h");
    let range = Range::parse("AA").expect("AA");
    let exact = equity_vs_range(hero, &board, &range, &[]).expect("exact");
    let direct = equity_vs_hand(hero, combo("Ad Ac"), &board, &[]).expect("рука");

    assert!(exact.exact);
    assert_eq!(exact.combos, 1);
    assert!((exact.wins - direct.wins).abs() < 1e-12, "{exact:?}");
    assert!((exact.losses - direct.losses).abs() < 1e-12, "{exact:?}");
    assert!((exact.ties - direct.ties).abs() < 1e-12, "{exact:?}");
}

#[test]
fn range_equity_is_weighted_average_of_hands() {
    let hero = combo("As Ah");
    let board = cards("2c 7d 9h");
    let range = Range::parse("KK").expect("KK");
    let exact = equity_vs_range(hero, &board, &range, &[]).expect("exact");

    let combos = ["Kc Kd", "Kc Kh", "Kc Ks", "Kd Kh", "Kd Ks", "Kh Ks"];
    let mut wins = 0.0;
    let mut losses = 0.0;
    let mut ties = 0.0;
    for text in combos {
        let equity = equity_vs_hand(hero, combo(text), &board, &[]).expect("рука");
        wins += equity.wins;
        losses += equity.losses;
        ties += equity.ties;
    }
    let count = combos.len() as f64;
    assert!((exact.wins - wins / count).abs() < 1e-12);
    assert!((exact.losses - losses / count).abs() < 1e-12);
    assert!((exact.ties - ties / count).abs() < 1e-12);
}

#[test]
fn weights_shift_the_result() {
    let hero = combo("Qs Qh");
    let board = cards("2c 7d 9h");
    let aa = equity_vs_range(hero, &board, &Range::parse("AA").expect("AA"), &[]).expect("AA");
    let kk = equity_vs_range(hero, &board, &Range::parse("KK").expect("KK"), &[]).expect("KK");
    let mixed = equity_vs_range(
        hero,
        &board,
        &Range::parse("AA:0.5, KK:1.0").expect("mixed"),
        &[],
    )
    .expect("mixed");
    let expected = (aa.hero_equity() * 0.5 + kk.hero_equity()) / 1.5;
    assert!((mixed.hero_equity() - expected).abs() < 1e-12, "{mixed:?}");
}

#[test]
fn exact_preflop_is_rejected() {
    let hero = combo("As Ah");
    let range = Range::parse("KK").expect("KK");
    assert_eq!(
        equity_vs_range(hero, &[], &range, &[]),
        Err(EvalError::ExactTooLarge)
    );
}

#[test]
fn monte_carlo_single_combo_matches_exact() {
    let hero = combo("As Ah");
    let board = cards("2c 7d 9h");
    let range = Range::parse("AA").expect("AA");
    let exact = equity_vs_range(hero, &board, &range, &[]).expect("exact");
    let sampled = equity_vs_range_mc(hero, &board, &range, &[], 200_000, 42).expect("MC");
    assert!((exact.hero_equity() - sampled.hero_equity()).abs() < 0.005);
}

#[test]
fn full_range_exact_on_flop_is_fast_enough() {
    let hero = combo("As Ah");
    let board = cards("2c 7d 9h");
    let range = Range::from_matrix([1000; 169]);
    let exact = equity_vs_range(hero, &board, &range, &[]).expect("exact");
    assert!(exact.exact);
    assert!(exact.combos > 1000, "{}", exact.combos);
    assert!((exact.wins + exact.losses + exact.ties - 1.0).abs() < 1e-12);
}

#[test]
fn monte_carlo_matches_exact_on_flop() {
    let hero = combo("As Ah");
    let board = cards("2c 7d 9h");
    let range = Range::parse("QQ").expect("QQ");
    let exact = equity_vs_range(hero, &board, &range, &[]).expect("exact");
    let sampled = equity_vs_range_mc(hero, &board, &range, &[], 200_000, 42).expect("MC");

    assert!(exact.exact);
    assert!(!sampled.exact);
    assert_eq!(sampled.samples, 200_000);
    assert!((exact.hero_equity() - sampled.hero_equity()).abs() < 0.005);
}

#[test]
fn monte_carlo_is_deterministic_and_works_preflop() {
    let hero = combo("As Ah");
    let range = Range::parse("KK").expect("KK");
    let first = equity_vs_range_mc(hero, &[], &range, &[], 10_000, 7).expect("MC");
    let second = equity_vs_range_mc(hero, &[], &range, &[], 10_000, 7).expect("MC");
    assert_eq!(first.wins, second.wins);
    assert_eq!(first.losses, second.losses);
    assert_eq!(first.ties, second.ties);
    assert!((first.wins + first.losses + first.ties - 1.0).abs() < 1e-12);
}

#[test]
fn empty_and_invalid_requests_are_errors() {
    let hero = combo("As Ah");
    let board = cards("2c 7d 9h");
    assert_eq!(
        equity_vs_range(hero, &board, &Range::new(), &[]),
        Err(EvalError::EmptyRange)
    );
    assert_eq!(
        equity_vs_range_mc(hero, &board, &Range::new(), &[], 100, 1),
        Err(EvalError::EmptyRange)
    );
    assert_eq!(
        equity_vs_range(hero, &cards("2c"), &Range::parse("KK").unwrap(), &[]),
        Err(EvalError::BadBoard { cards: 1 })
    );
    assert!(matches!(
        equity_vs_range(hero, &cards("As 7d 9h"), &Range::parse("KK").unwrap(), &[]),
        Err(EvalError::DuplicateCard(_))
    ));
    assert_eq!(
        equity_vs_range_mc(hero, &board, &Range::parse("KK").unwrap(), &[], 0, 1),
        Err(EvalError::EmptySamples)
    );
}

#[test]
fn combo_count_is_zero_when_range_is_fully_blocked() {
    let range = Range::parse("AA").expect("AA");
    let dead = cards("As Ah Ad Ac");
    assert_eq!(range.combo_count(&dead), 0);
}
