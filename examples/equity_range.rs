//! Пример: эквити Hero против диапазона и диапазон против диапазона.
//!
//! `cargo run -p lynx-eval --example equity_range`

use std::time::Instant;

use lynx_eval::{Card, Range, equity_range_vs_range, equity_range_vs_range_mc, equity_vs_range};

fn card(text: &str) -> Card {
    text.parse().expect("карта")
}

fn main() {
    let hero = [card("As"), card("Ah")];
    let board = [card("2c"), card("7d"), card("9h")];
    let range = Range::parse("KK").expect("диапазон");
    let equity = equity_vs_range(hero, &board, &range, &[]).expect("эквити");

    println!(
        "Hero AsAh vs KK на 2c7d9h: {:.4}% (wins {:.4}, ties {:.4}, losses {:.4}), \
         combos {}, samples {}, exact {}",
        equity.hero_equity() * 100.0,
        equity.wins * 100.0,
        equity.ties * 100.0,
        equity.losses * 100.0,
        equity.combos,
        equity.samples,
        equity.exact,
    );

    let dead = [card("4c"), card("5d")];
    let with_dead = equity_vs_range(hero, &board, &range, &dead).expect("эквити с dead");
    println!(
        "Hero AsAh vs KK на 2c7d9h, мёртвые 4c5d: {:.4}% (wins {:.4}, ties {:.4}, \
         losses {:.4}), combos {}, samples {}, exact {}",
        with_dead.hero_equity() * 100.0,
        with_dead.wins * 100.0,
        with_dead.ties * 100.0,
        with_dead.losses * 100.0,
        with_dead.combos,
        with_dead.samples,
        with_dead.exact,
    );

    let hero_range = Range::parse("QQ+, AKs").expect("hero range");
    let villain_range = Range::parse("JJ+, AQs+").expect("villain range");

    let started = Instant::now();
    match equity_range_vs_range(&hero_range, &villain_range, &board, &[]) {
        Ok(result) => println!(
            "Range vs range exact на 2c7d9h: Hero {:.4}%, Villain {:.4}% \
             (wins {:.4}, ties {:.4}), combos {} vs {}, samples {}, exact {}, {:?}",
            result.hero_equity * 100.0,
            result.villain_equity * 100.0,
            result.hero_wins * 100.0,
            result.ties * 100.0,
            result.hero_combos,
            result.villain_combos,
            result.samples,
            result.exact,
            started.elapsed(),
        ),
        Err(error) => println!("Range vs range exact недоступен: {error}"),
    }

    let started = Instant::now();
    let sampled = equity_range_vs_range_mc(&hero_range, &villain_range, &board, &[], 200_000, 42)
        .expect("MC");
    println!(
        "Range vs range MC 200k: Hero {:.4}%, Villain {:.4}% \
         (wins {:.4}, ties {:.4}), combos {} vs {}, samples {}, exact {}, {:?}",
        sampled.hero_equity * 100.0,
        sampled.villain_equity * 100.0,
        sampled.hero_wins * 100.0,
        sampled.ties * 100.0,
        sampled.hero_combos,
        sampled.villain_combos,
        sampled.samples,
        sampled.exact,
        started.elapsed(),
    );
}
