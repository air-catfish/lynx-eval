//! Свойства: случайные раздачи без паник, совпадение с лучшей пятёркой,
//! повторяемость по seed.

use lynx_eval::{Card, best_five, describe, equity_vs_hand_mc, evaluate};

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn fresh_deck() -> [Card; 52] {
    let mut deck = [Card::from_index(0).expect("двойка треф"); 52];
    for index in 0..52u8 {
        deck[index as usize] = Card::from_index(index).expect("карта");
    }
    deck
}

fn shuffle(deck: &mut [Card; 52], rng: &mut Rng) {
    for index in (1..deck.len()).rev() {
        let other = (rng.next() % (index as u64 + 1)) as usize;
        deck.swap(index, other);
    }
}

#[test]
fn random_deals_do_not_panic_and_match_best_five() {
    let mut rng = Rng::new(2026);
    let mut deck = fresh_deck();

    for round in 0..100_000 {
        shuffle(&mut deck, &mut rng);
        let seven = &deck[..7];

        let value = evaluate(seven).expect("семь карт");
        let _ = describe(value);

        // Полный оракул — максимум по всем пятёркам — на каждой пятой раздаче.
        if round % 5 == 0 {
            let best = best_five(seven).expect("лучшая пятёрка");
            assert_eq!(
                value,
                evaluate(&best).expect("пятёрка"),
                "прямая оценка семи карт разошлась с лучшей пятёркой: {seven:?}"
            );
        }
    }
}

#[test]
fn same_seed_same_monte_carlo_result() {
    let hero = ["As".parse().unwrap(), "Ah".parse().unwrap()];
    let villain = ["Ks".parse().unwrap(), "Kd".parse().unwrap()];
    let board = [
        "2c".parse().unwrap(),
        "7d".parse().unwrap(),
        "9h".parse().unwrap(),
    ];

    let first = equity_vs_hand_mc(hero, villain, &board, &[], 5_000, 0).expect("seed 0");
    let second = equity_vs_hand_mc(hero, villain, &board, &[], 5_000, 0).expect("seed 0");
    assert_eq!(first.wins, second.wins);
    assert_eq!(first.losses, second.losses);
    assert_eq!(first.ties, second.ties);
    assert_eq!(first.samples, 5_000);
}
