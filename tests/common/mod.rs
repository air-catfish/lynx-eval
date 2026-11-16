#![allow(dead_code)]

use lynx_eval::Card;

/// Разбирает строку вида `"As Kd 7c"`.
pub fn cards(text: &str) -> Vec<Card> {
    text.split_whitespace()
        .map(|token| token.parse().expect("карта"))
        .collect()
}

/// Разбирает ровно `N` карт.
pub fn hand<const N: usize>(text: &str) -> [Card; N] {
    cards(text)
        .try_into()
        .unwrap_or_else(|cards: Vec<Card>| panic!("ожидалось {N} карт, получено {}", cards.len()))
}
