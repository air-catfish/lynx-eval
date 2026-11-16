//! Замеры. Только release, чтобы числа были осмысленными:
//!
//! ```text
//! cargo test -p lynx-eval --release --test perf -- --ignored --nocapture
//! ```

use std::hint::black_box;
use std::time::Instant;

use lynx_eval::{Card, equity_vs_hand, equity_vs_hand_mc, evaluate};

fn card(text: &str) -> Card {
    text.parse().expect("карта")
}

#[test]
#[ignore = "замеры; запускать в release с --nocapture"]
fn measure_evaluate_and_equity() {
    let hand = [
        card("As"),
        card("Kd"),
        card("7c"),
        card("2h"),
        card("9s"),
        card("Qd"),
        card("Jc"),
    ];

    for _ in 0..10_000 {
        black_box(evaluate(black_box(&hand)).expect("рука"));
    }

    let iterations = 2_000_000u32;
    let started = Instant::now();
    for _ in 0..iterations {
        black_box(evaluate(black_box(&hand)).expect("рука"));
    }
    let elapsed = started.elapsed();
    let nanos = elapsed.as_nanos() as f64 / f64::from(iterations);
    println!("evaluate(7): {nanos:.1} нс/рука ({iterations} итераций за {elapsed:?})");

    let hero = [card("As"), card("Ah")];
    let villain = [card("Ks"), card("Kd")];

    let started = Instant::now();
    let exact = equity_vs_hand(hero, villain, &[], &[]).expect("префлоп exact");
    println!(
        "префлоп exact: {:?} (сэмплов {}, hero {:.4}%)",
        started.elapsed(),
        exact.samples,
        exact.hero_equity() * 100.0
    );

    let started = Instant::now();
    let sampled = equity_vs_hand_mc(hero, villain, &[], &[], 200_000, 42).expect("префлоп MC");
    println!(
        "префлоп MC 200k: {:?} (hero {:.4}%)",
        started.elapsed(),
        sampled.hero_equity() * 100.0
    );

    let board = [card("2c"), card("7d"), card("9h")];
    let started = Instant::now();
    let flop = equity_vs_hand(hero, villain, &board, &[]).expect("флоп exact");
    println!(
        "флоп exact: {:?} (сэмплов {}, hero {:.4}%)",
        started.elapsed(),
        flop.samples,
        flop.hero_equity() * 100.0
    );
}
