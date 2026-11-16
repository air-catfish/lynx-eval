//! Разбор `HandValue` на категорию и значимые ранги — без строк.

use crate::cards::Rank;
use crate::eval::{HandCategory, HandValue};

/// Разбор силы руки.
///
/// Текста здесь нет намеренно: локализация — в приложении. Приложение берёт
/// [`HandDescription::category`] и [`HandDescription::ranks`] и собирает подпись
/// само, поэтому «A kicker» не появляется в открытом крейте.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandDescription {
    /// Категория комбинации.
    pub category: HandCategory,
    /// Значимые ранги по убыванию; неиспользованные — `None`.
    ///
    /// По категориям: старшая карта и флеш — 5 рангов; пара — ранг пары и три
    /// кикера; две пары — две пары и кикер; сет — сет и два кикера; стрит и
    /// стрит-флеш — старшая карта (колесо — пятёрка); фулл-хаус — сет и пара;
    /// каре — каре и кикер.
    pub ranks: [Option<Rank>; 5],
}

/// Разбирает силу руки на категорию и ранги.
#[must_use]
pub fn describe(value: HandValue) -> HandDescription {
    let category = value.category();
    let mut ranks = [None; 5];
    for (position, slot) in ranks
        .iter_mut()
        .enumerate()
        .take(significant_ranks(category))
    {
        *slot = Rank::from_index(value.rank_at(position));
    }
    HandDescription { category, ranks }
}

pub(crate) fn significant_ranks(category: HandCategory) -> usize {
    match category {
        HandCategory::HighCard | HandCategory::Flush => 5,
        HandCategory::Pair => 4,
        HandCategory::TwoPair | HandCategory::ThreeOfAKind => 3,
        HandCategory::Straight | HandCategory::StraightFlush => 1,
        HandCategory::FullHouse | HandCategory::FourOfAKind => 2,
    }
}
