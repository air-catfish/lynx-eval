//! Диапазон против диапазона: совместимые пары, веса, блокеры, exact/MC.

mod common;

use common::{cards, hand};
use lynx_eval::{
    EXACT_COMPARISON_BUDGET, EvalError, Range, equity_range_vs_range, equity_range_vs_range_mc,
    equity_vs_hand, equity_vs_range,
};

fn combo(text: &str) -> [lynx_eval::Card; 2] {
    hand::<2>(text)
}

#[test]
fn single_combo_pair_matches_hand_equity() {
    // Девять мёртвых карт оставляют ровно одно комбо с каждой стороны:
    // Hero — AcKc, Villain — QcJc. Тогда range-vs-range обязан совпасть с
    // эквити «рука против руки».
    let dead = cards("As Ad Ah Qs Qd Qh Js Jd Jh");
    let board = cards("2h 7d 9s");
    let hero_range = Range::parse("AKs").expect("AKs");
    let villain_range = Range::parse("QJs").expect("QJs");

    let exact =
        equity_range_vs_range(&hero_range, &villain_range, &board, &dead).expect("range-vs-range");
    let direct = equity_vs_hand(combo("Ac Kc"), combo("Qc Jc"), &board, &dead).expect("рука");

    assert!(exact.exact);
    assert_eq!(exact.hero_combos, 1);
    assert_eq!(exact.villain_combos, 1);
    // 52 − 3 борда − 9 мёртвых − 4 карты пары = 36 карт, C(36, 2) = 630.
    assert_eq!(exact.samples, 630, "один ранаут-набор на одно комбо");
    assert!((exact.hero_wins - direct.wins).abs() < 1e-12, "{exact:?}");
    assert!(
        (exact.villain_wins - direct.losses).abs() < 1e-12,
        "{exact:?}"
    );
    assert!((exact.ties - direct.ties).abs() < 1e-12, "{exact:?}");
    assert!((exact.hero_equity - direct.hero_equity()).abs() < 1e-12);
}

#[test]
fn same_range_is_symmetric() {
    let hero = Range::parse("QQ").expect("QQ");
    let board = cards("2c 7d 9h");
    let exact = equity_range_vs_range(&hero, &hero, &board, &[]).expect("exact");

    assert_eq!(exact.hero_wins, exact.villain_wins, "пары (A,B) и (B,A)");
    assert!((exact.hero_equity - 0.5).abs() < 1e-12);
    assert!((exact.villain_equity - 0.5).abs() < 1e-12);
    assert!((exact.hero_wins + exact.villain_wins + exact.ties - 1.0).abs() < 1e-12);
}

#[test]
fn exact_flop_matches_mc_on_river_and_flop() {
    let hero = Range::parse("QQ").expect("QQ");
    let villain = Range::parse("JJ").expect("JJ");

    let flop = cards("2c 7d 9h");
    let exact_flop = equity_range_vs_range(&hero, &villain, &flop, &[]).expect("флоп exact");
    let mc_flop =
        equity_range_vs_range_mc(&hero, &villain, &flop, &[], 200_000, 42).expect("флоп MC");
    assert!(exact_flop.exact);
    assert!(!mc_flop.exact);
    assert_eq!(mc_flop.samples, 200_000);
    assert!(
        (exact_flop.hero_equity - mc_flop.hero_equity).abs() < 0.005,
        "{exact_flop:?} vs {mc_flop:?}"
    );

    let river = cards("2c 7d 9h Ks 3d");
    let exact_river = equity_range_vs_range(&hero, &villain, &river, &[]).expect("ривер exact");
    assert_eq!(exact_river.samples, 36, "6 комбо × 6 комбо, один ранаут");
    let mc_river =
        equity_range_vs_range_mc(&hero, &villain, &river, &[], 200_000, 42).expect("ривер MC");
    assert!((exact_river.hero_equity - mc_river.hero_equity).abs() < 0.005);
}

#[test]
fn weighted_average_of_pairs_is_used() {
    let board = cards("2c 7d 9h");
    let villain = Range::parse("KK").expect("KK");

    let pure = equity_range_vs_range(&Range::parse("AA").expect("AA"), &villain, &board, &[])
        .expect("AA vs KK");
    let queeny = equity_range_vs_range(&Range::parse("QQ").expect("QQ"), &villain, &board, &[])
        .expect("QQ vs KK");
    let mixed = equity_range_vs_range(
        &Range::parse("AA:0.5, QQ:1.0").expect("mixed"),
        &villain,
        &board,
        &[],
    )
    .expect("mixed");

    let expected = (pure.hero_equity * 0.5 + queeny.hero_equity) / 1.5;
    assert!((mixed.hero_equity - expected).abs() < 1e-9, "{mixed:?}");
}

#[test]
fn dead_cards_shrink_combos_and_pairs() {
    let hero = Range::parse("AA").expect("AA");
    let villain = Range::parse("KK").expect("KK");
    let board = cards("2c 7d 9h");
    let dead = cards("As Ah");

    let blocked = equity_range_vs_range(&hero, &villain, &board, &dead).expect("exact");
    assert_eq!(blocked.hero_combos, 1, "AdAc");
    assert_eq!(blocked.villain_combos, 6);
    // 52 − 3 борда − 2 мёртвых − 4 карты пары = 43, C(43, 2) = 903.
    assert_eq!(blocked.samples, 6 * 903, "1 × 6 пар × 903 ранаута");

    let free = equity_range_vs_range(&hero, &villain, &board, &[]).expect("exact");
    assert_eq!(free.hero_combos, 6);
    assert_eq!(free.samples, 6 * 6 * 990);
}

#[test]
fn outcomes_and_equities_sum_to_one() {
    let hero = Range::parse("AA, KK:0.5").expect("hero");
    let villain = Range::parse("QQ, AKs").expect("villain");
    let board = cards("2c 7d 9h");

    let exact = equity_range_vs_range(&hero, &villain, &board, &[]).expect("exact");
    assert!(
        (exact.hero_wins + exact.villain_wins + exact.ties - 1.0).abs() < 1e-12,
        "{exact:?}"
    );
    assert!((exact.hero_equity + exact.villain_equity - 1.0).abs() < 1e-12);
    assert!((exact.hero_equity - (exact.hero_wins + exact.ties / 2.0)).abs() < 1e-12);

    let sampled = equity_range_vs_range_mc(&hero, &villain, &board, &[], 50_000, 9).expect("MC");
    assert!((sampled.hero_wins + sampled.villain_wins + sampled.ties - 1.0).abs() < 1e-12);
    assert!((sampled.hero_equity + sampled.villain_equity - 1.0).abs() < 1e-12);
}

#[test]
fn monte_carlo_is_deterministic_and_works_preflop() {
    let hero = Range::parse("AA, KK").expect("hero");
    let villain = Range::parse("QQ, AKs").expect("villain");
    let first = equity_range_vs_range_mc(&hero, &villain, &[], &[], 5_000, 7).expect("MC");
    let second = equity_range_vs_range_mc(&hero, &villain, &[], &[], 5_000, 7).expect("MC");
    assert_eq!(first.hero_wins, second.hero_wins);
    assert_eq!(first.villain_wins, second.villain_wins);
    assert_eq!(first.ties, second.ties);
    assert!(!first.exact);
    assert!((first.hero_wins + first.villain_wins + first.ties - 1.0).abs() < 1e-12);
}

#[test]
fn exact_is_rejected_when_work_exceeds_budget() {
    let full = Range::from_matrix([1000; 169]);
    assert_eq!(
        equity_range_vs_range(&full, &full, &cards("2c 7d 9h"), &[]),
        Err(EvalError::ExactTooLarge),
        "полный диапазон на флопе заведомо больше бюджета"
    );
    assert_eq!(
        equity_range_vs_range(
            &Range::parse("AA").expect("AA"),
            &Range::parse("KK").expect("KK"),
            &[],
            &[]
        ),
        Err(EvalError::ExactTooLarge),
        "префлоп точным перебором не считается"
    );
    // Узкий флоп обязан помещаться в бюджет и перебираться честно.
    let narrow = equity_range_vs_range(
        &Range::parse("AA").expect("AA"),
        &Range::parse("KK").expect("KK"),
        &cards("2c 7d 9h"),
        &[],
    )
    .expect("узкий флоп");
    assert!(narrow.exact);
    assert!(narrow.samples <= EXACT_COMPARISON_BUDGET);
}

#[test]
fn empty_and_incompatible_ranges_are_errors() {
    let board = cards("2c 7d 9h");
    let range = Range::parse("KK").expect("KK");
    assert_eq!(
        equity_range_vs_range(&Range::new(), &range, &board, &[]),
        Err(EvalError::EmptyRange)
    );
    assert_eq!(
        equity_range_vs_range(&range, &Range::new(), &board, &[]),
        Err(EvalError::EmptyRange)
    );

    // Мёртвые карты As, Ad, Ah оставляют обоим диапазонам одно и то же комбо
    // AcKc: пересечение по картам, совместимых пар нет.
    let same = Range::parse("AKs").expect("AKs");
    assert_eq!(
        equity_range_vs_range(&same, &same, &board, &cards("As Ad Ah")),
        Err(EvalError::EmptyRange)
    );
    assert_eq!(
        equity_range_vs_range_mc(&same, &same, &board, &cards("As Ad Ah"), 1_000, 1),
        Err(EvalError::EmptyRange)
    );
}

#[test]
fn invalid_input_is_an_error() {
    let hero = Range::parse("AA").expect("AA");
    let villain = Range::parse("KK").expect("KK");

    assert_eq!(
        equity_range_vs_range(&hero, &villain, &cards("2c"), &[]),
        Err(EvalError::BadBoard { cards: 1 })
    );
    assert_eq!(
        equity_range_vs_range(&hero, &villain, &cards("2c 7d 9h"), &cards("2c")),
        Err(EvalError::DuplicateCard(cards("2c")[0]))
    );
    assert_eq!(
        equity_range_vs_range_mc(&hero, &villain, &cards("2c 7d 9h"), &[], 0, 1),
        Err(EvalError::EmptySamples)
    );
}

#[test]
fn exact_dead_cards_do_not_change_river_without_overlap() {
    // На ривере ранаутов нет: мёртвые карты вне Hero, борда и диапазона не
    // меняют ни одного числа. Их влияние на флопе/тёрне — честное: карт в
    // колоде меньше, и ранауты меняются.
    let hero = combo("As Ah");
    let board = cards("2c 7d 9h Ks 3d");
    let range = Range::parse("QQ").expect("QQ");

    let without = equity_vs_range(hero, &board, &range, &[]).expect("без dead");
    let with = equity_vs_range(hero, &board, &range, &cards("4c 5d")).expect("с dead");
    assert_eq!(without.combos, with.combos);
    assert!((without.hero_equity() - with.hero_equity()).abs() < 1e-12);
}

#[test]
fn dead_cards_reduce_hand_vs_range_combos() {
    let hero = combo("Ks Kh");
    let board = cards("2c 7d 9h");
    let range = Range::parse("AA").expect("AA");

    let free = equity_vs_range(hero, &board, &range, &[]).expect("без dead");
    let blocked = equity_vs_range(hero, &board, &range, &cards("As Ah")).expect("с dead");
    assert_eq!(free.combos, 6);
    assert_eq!(blocked.combos, 1, "остаётся AdAc");
    assert!(
        (free.hero_equity() - blocked.hero_equity()).abs() > 1e-6,
        "блокеры меняют соперника, а не только число комбо"
    );
}

#[test]
fn dead_intersecting_hand_is_an_error() {
    let hero = combo("As Ah");
    assert_eq!(
        equity_vs_range(
            hero,
            &cards("2c 7d 9h"),
            &Range::parse("KK").unwrap(),
            &cards("As")
        ),
        Err(EvalError::DuplicateCard(cards("As")[0]))
    );
}
