//! Полный перебор всех пятёрок: ровно 7462 класса рук.
//!
//! Тест тяжёлый, поэтому помечен `#[ignore]`; гоняется в release:
//!
//! ```text
//! cargo test -p lynx-eval --release --test classes -- --ignored
//! ```

mod common;

use common::cards;
use lynx_eval::{Card, evaluate};

#[test]
#[ignore = "2 598 960 пятёрок; запускать в release"]
fn five_card_hands_have_7462_classes() {
    // Сырое значение HandValue: категория (3 бита) + пять рангов по 4 бита,
    // максимум 0x8F_FFFF, поэтому хватает 2^24 бит.
    let words = (1usize << 24) / 64;
    let mut seen = vec![0u64; words];
    let mut classes = 0usize;
    let mut hands = 0u64;

    for first in 0..48u8 {
        let a = Card::from_index(first).expect("карта");
        for second in first + 1..49u8 {
            let b = Card::from_index(second).expect("карта");
            for third in second + 1..50u8 {
                let c = Card::from_index(third).expect("карта");
                for fourth in third + 1..51u8 {
                    let d = Card::from_index(fourth).expect("карта");
                    for fifth in fourth + 1..52u8 {
                        let e = Card::from_index(fifth).expect("карта");
                        let value = evaluate(&[a, b, c, d, e]).expect("пятёрка");
                        let raw = value.raw() as usize;
                        assert!(raw < (1 << 24), "сырое значение вне диапазона: {raw:#x}");

                        let word = raw / 64;
                        let bit = 1u64 << (raw % 64);
                        if seen[word] & bit == 0 {
                            seen[word] |= bit;
                            classes += 1;
                        }
                        hands += 1;
                    }
                }
            }
        }
    }

    assert_eq!(hands, 2_598_960, "число пятёрок C(52,5)");
    assert_eq!(classes, 7462, "классическое число классов 5-карточных рук");
}

#[test]
fn evaluate_rejects_a_duplicate_in_five_cards() {
    let hand = cards("As As Kd Qc Jh");
    assert!(evaluate(&hand).is_err());
}
