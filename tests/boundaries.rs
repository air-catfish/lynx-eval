//! Границы: колесо, флеш против стрита, кикеры, лучшая пятёрка из семи.

mod common;

use common::cards;
use lynx_eval::{HandCategory, Rank, best_five, describe, evaluate};

#[test]
fn wheel_is_five_high_straight_not_ace_high() {
    let value = evaluate(&cards("As 2d 3c 4h 5s")).expect("колесо");
    assert_eq!(value.category(), HandCategory::Straight);
    let text = describe(value);
    assert_eq!(text.ranks[0], Some(Rank::Five));
    assert_eq!(text.ranks[1], None);
}

#[test]
fn royal_flush_is_ace_high_straight_flush() {
    let value = evaluate(&cards("As Ks Qs Js Ts")).expect("роял");
    assert_eq!(value.category(), HandCategory::StraightFlush);
    assert_eq!(describe(value).ranks[0], Some(Rank::Ace));
}

#[test]
fn flush_beats_straight() {
    let flush = evaluate(&cards("As Ks Qs Js 9s")).expect("флеш");
    let straight = evaluate(&cards("As Kd Qc Jh Ts")).expect("стрит");
    assert_eq!(flush.category(), HandCategory::Flush);
    assert_eq!(straight.category(), HandCategory::Straight);
    assert!(flush > straight);
}

#[test]
fn four_of_a_kind_keeps_kicker() {
    let value = evaluate(&cards("As Ah Ad Ac Kd")).expect("каре");
    assert_eq!(value.category(), HandCategory::FourOfAKind);
    let text = describe(value);
    assert_eq!(text.ranks[0], Some(Rank::Ace));
    assert_eq!(text.ranks[1], Some(Rank::King));
}

#[test]
fn full_house_is_trip_over_pair() {
    let value = evaluate(&cards("As Ah Ad Kc Kd")).expect("фулл-хаус");
    assert_eq!(value.category(), HandCategory::FullHouse);
    let text = describe(value);
    assert_eq!(text.ranks[0], Some(Rank::Ace));
    assert_eq!(text.ranks[1], Some(Rank::King));
}

#[test]
fn two_trips_make_full_house_not_two_trips() {
    let value = evaluate(&cards("As Ah Ad Ks Kh Kd 2c")).expect("две тройки");
    assert_eq!(value.category(), HandCategory::FullHouse);
    let text = describe(value);
    assert_eq!(text.ranks[0], Some(Rank::Ace));
    assert_eq!(text.ranks[1], Some(Rank::King));
}

#[test]
fn best_five_uses_sixth_and_seventh_cards() {
    // 2-3-4-5-9 плюс 6 и 7: лучшая рука — стрит 3-4-5-6-7, собранный
    // шестой и седьмой картами.
    let all = cards("2c 3d 4h 5s 9c 6d 7s");
    let best = best_five(&all).expect("лучшая пятёрка");
    assert!(best.contains(&cards("6d")[0]), "в лучшей пятёрке нет 6d");
    assert!(best.contains(&cards("7s")[0]), "в лучшей пятёрке нет 7s");

    let value = evaluate(&all).expect("семь карт");
    assert_eq!(value.category(), HandCategory::Straight);
    assert_eq!(value, evaluate(&best).expect("пятёрка"));
}

#[test]
fn best_five_prefers_flush_over_straight() {
    let all = cards("As Ks Qs Js 9s Th 2d");
    let value = evaluate(&all).expect("семь карт");
    assert_eq!(value.category(), HandCategory::Flush);
    let best = best_five(&all).expect("лучшая пятёрка");
    assert_eq!(value, evaluate(&best).expect("пятёрка"));
}
