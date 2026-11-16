//! Сила руки: категория, упакованный `HandValue`, лучшая пятёрка.

use core::fmt;

use crate::cards::Card;

/// Категория покерной комбинации.
///
/// Порядок вариантов — старшинство: производный `Ord` сравнивает категории
/// так же, как правила покера (флеш бьёт стрит, фулл-хаус бьёт флеш).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum HandCategory {
    /// Старшая карта.
    HighCard = 0,
    /// Пара.
    Pair = 1,
    /// Две пары.
    TwoPair = 2,
    /// Сет.
    ThreeOfAKind = 3,
    /// Стрит.
    Straight = 4,
    /// Флеш.
    Flush = 5,
    /// Фулл-хаус.
    FullHouse = 6,
    /// Каре.
    FourOfAKind = 7,
    /// Стрит-флеш; роял-флеш — его старший случай.
    StraightFlush = 8,
}

impl HandCategory {
    pub(crate) const fn from_code(code: u8) -> Self {
        match code {
            0 => Self::HighCard,
            1 => Self::Pair,
            2 => Self::TwoPair,
            3 => Self::ThreeOfAKind,
            4 => Self::Straight,
            5 => Self::Flush,
            6 => Self::FullHouse,
            7 => Self::FourOfAKind,
            _ => Self::StraightFlush,
        }
    }
}

/// Сила руки: одна упакованная `u32`.
///
/// Старший байт — категория, дальше пять рангов по четыре бита. Производный
/// `Ord` сравнивает руки сначала по категории, затем по рангам лексикографически,
/// что и есть покерное сравнение.
///
/// Числовое представление непрозрачно и не является стабильным контрактом.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandValue(u32);

const CATEGORY_SHIFT: u32 = 20;

impl HandValue {
    pub(crate) const fn new(category: HandCategory, ranks: [u8; 5]) -> Self {
        HandValue(
            ((category as u32) << CATEGORY_SHIFT)
                | ((ranks[0] as u32) << 16)
                | ((ranks[1] as u32) << 12)
                | ((ranks[2] as u32) << 8)
                | ((ranks[3] as u32) << 4)
                | (ranks[4] as u32),
        )
    }

    /// Упакованное значение; для отладки и тестов, не для сериализации.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Категория руки.
    #[must_use]
    pub const fn category(self) -> HandCategory {
        HandCategory::from_code((self.0 >> CATEGORY_SHIFT) as u8)
    }

    /// Ранг на позиции `0..5` (0 — самый значимый).
    pub(crate) const fn rank_at(self, position: usize) -> u8 {
        ((self.0 >> (CATEGORY_SHIFT - 4 * (position as u32 + 1))) & 0xF) as u8
    }
}

/// Ошибка эвалюатора или расчёта эквити.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalError {
    /// Меньше пяти карт в [`evaluate`] или [`best_five`].
    NotEnoughCards,
    /// Больше семи карт в [`evaluate`] или [`best_five`].
    TooManyCards,
    /// Одна и та же карта встретилась дважды.
    DuplicateCard(Card),
    /// Борд должен быть пустым или содержать 3, 4 либо 5 карт.
    BadBoard {
        /// Фактическое число карт на борде.
        cards: usize,
    },
    /// В колоде не осталось карт, чтобы достроить борд.
    NotEnoughRemainingCards,
    /// Монте-Карло вызван с `samples == 0`.
    EmptySamples,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::NotEnoughCards => write!(f, "нужно от 5 до 7 карт"),
            EvalError::TooManyCards => write!(f, "допустимо не больше 7 карт"),
            EvalError::DuplicateCard(card) => write!(f, "карта {card} встретилась дважды"),
            EvalError::BadBoard { cards } => {
                write!(
                    f,
                    "борд должен быть пустым или из 3, 4, 5 карт, получено {cards}"
                )
            }
            EvalError::NotEnoughRemainingCards => write!(f, "в колоде не хватает карт на борд"),
            EvalError::EmptySamples => write!(f, "число сэмплов должно быть больше нуля"),
        }
    }
}

impl std::error::Error for EvalError {}

/// Оценивает руку из 5–7 карт.
///
/// # Ошибки
///
/// Меньше 5 карт, больше 7 или повтор карты — [`EvalError`].
pub fn evaluate(cards: &[Card]) -> Result<HandValue, EvalError> {
    match cards.len() {
        5..=7 => ensure_distinct(cards)?,
        len if len < 5 => return Err(EvalError::NotEnoughCards),
        _ => return Err(EvalError::TooManyCards),
    }
    Ok(eval_unchecked(cards))
}

/// Выбирает лучшую пятёрку из 5–7 карт.
///
/// Если несколько наборов дают одинаковую силу (на практике это редкость),
/// возвращается первый в порядке перебора — результат детерминирован.
///
/// # Ошибки
///
/// Меньше 5 карт, больше 7 или повтор карты — [`EvalError`].
pub fn best_five(cards: &[Card]) -> Result<[Card; 5], EvalError> {
    let len = cards.len();
    match len {
        5..=7 => ensure_distinct(cards)?,
        len if len < 5 => return Err(EvalError::NotEnoughCards),
        _ => return Err(EvalError::TooManyCards),
    }
    if len == 5 {
        let mut hand = [cards[0]; 5];
        hand.copy_from_slice(cards);
        return Ok(hand);
    }

    let mut best = [cards[0]; 5];
    let mut best_value = HandValue(0);
    for first in 0..len - 4 {
        for second in first + 1..len - 3 {
            for third in second + 1..len - 2 {
                for fourth in third + 1..len - 1 {
                    for fifth in fourth + 1..len {
                        let hand = [
                            cards[first],
                            cards[second],
                            cards[third],
                            cards[fourth],
                            cards[fifth],
                        ];
                        let value = eval_unchecked(&hand);
                        if value > best_value {
                            best_value = value;
                            best = hand;
                        }
                    }
                }
            }
        }
    }
    Ok(best)
}

pub(crate) fn ensure_distinct(cards: &[Card]) -> Result<(), EvalError> {
    let mut seen = 0u64;
    for &card in cards {
        let bit = 1u64 << card.index();
        if seen & bit != 0 {
            return Err(EvalError::DuplicateCard(card));
        }
        seen |= bit;
    }
    Ok(())
}

/// Быстрая оценка 5–7 **различных** карт без проверок.
///
/// Каждая карта учитывается один раз; вызывающий обязан это гарантировать.
pub(crate) fn eval_unchecked(cards: &[Card]) -> HandValue {
    debug_assert!((5..=7).contains(&cards.len()));

    let mut rank_mask = 0u16;
    let mut suit_mask = [0u16; 4];
    let mut counts = [0u8; 13];
    for &card in cards {
        let index = card.index();
        let rank = (index >> 2) as usize;
        let suit = (index & 3) as usize;
        counts[rank] += 1;
        rank_mask |= 1u16 << rank;
        suit_mask[suit] |= 1u16 << rank;
    }

    // Флеш и стрит-флеш: карт одной масти не меньше пяти, масть максимум одна.
    for mask in suit_mask {
        if mask.count_ones() >= 5 {
            if let Some(high) = straight_high(mask) {
                return HandValue::new(HandCategory::StraightFlush, [high, 0, 0, 0, 0]);
            }
            return HandValue::new(HandCategory::Flush, top_five(mask));
        }
    }

    // Стрит (после флеша: флеш старше; каре и фулл-хаус при флеше невозможны).
    if let Some(high) = straight_high(rank_mask) {
        return HandValue::new(HandCategory::Straight, [high, 0, 0, 0, 0]);
    }

    let mut quad = None;
    let mut trips = [0u8; 2];
    let mut trips_len = 0;
    let mut pairs = [0u8; 3];
    let mut pairs_len = 0;
    for rank in (0..13).rev() {
        match counts[rank] {
            4 => quad = Some(rank as u8),
            3 => {
                if trips_len < trips.len() {
                    trips[trips_len] = rank as u8;
                }
                trips_len += 1;
            }
            2 => {
                if pairs_len < pairs.len() {
                    pairs[pairs_len] = rank as u8;
                }
                pairs_len += 1;
            }
            _ => {}
        }
    }

    // Каре.
    if let Some(quad) = quad {
        let mut kicker = [0u8; 1];
        highest_ranks(&counts, &[quad], &mut kicker);
        return HandValue::new(HandCategory::FourOfAKind, [quad, kicker[0], 0, 0, 0]);
    }

    // Фулл-хаус: тройка + пара (вторая тройка считается парой).
    if trips_len > 0 {
        let trip = trips[0];
        let boat_pair = if trips_len >= 2 {
            Some(trips[1])
        } else if pairs_len >= 1 {
            Some(pairs[0])
        } else {
            None
        };
        if let Some(pair) = boat_pair {
            return HandValue::new(HandCategory::FullHouse, [trip, pair, 0, 0, 0]);
        }
        let mut kickers = [0u8; 2];
        highest_ranks(&counts, &[trip], &mut kickers);
        return HandValue::new(
            HandCategory::ThreeOfAKind,
            [trip, kickers[0], kickers[1], 0, 0],
        );
    }

    // Две пары.
    if pairs_len >= 2 {
        let first = pairs[0];
        let second = pairs[1];
        let mut kicker = [0u8; 1];
        highest_ranks(&counts, &[first, second], &mut kicker);
        return HandValue::new(HandCategory::TwoPair, [first, second, kicker[0], 0, 0]);
    }

    // Пара.
    if pairs_len == 1 {
        let pair = pairs[0];
        let mut kickers = [0u8; 3];
        highest_ranks(&counts, &[pair], &mut kickers);
        return HandValue::new(
            HandCategory::Pair,
            [pair, kickers[0], kickers[1], kickers[2], 0],
        );
    }

    // Старшая карта.
    HandValue::new(HandCategory::HighCard, top_five(rank_mask))
}

/// Пять старших рангов из маски.
fn top_five(mask: u16) -> [u8; 5] {
    let mut ranks = [0u8; 5];
    let mut filled = 0;
    for rank in (0..13).rev() {
        if mask & (1u16 << rank) != 0 {
            ranks[filled] = rank as u8;
            filled += 1;
            if filled == 5 {
                break;
            }
        }
    }
    ranks
}

/// Старшие ранги, кроме перечисленных, по убыванию.
fn highest_ranks(counts: &[u8; 13], excluded: &[u8], out: &mut [u8]) {
    let mut filled = 0;
    for rank in (0..13).rev() {
        if counts[rank] == 0 {
            continue;
        }
        let rank = rank as u8;
        if excluded.contains(&rank) {
            continue;
        }
        out[filled] = rank;
        filled += 1;
        if filled == out.len() {
            break;
        }
    }
}

/// Старшая карта стрита из маски рангов; колесо `A-2-3-4-5` — пятёрка.
fn straight_high(mask: u16) -> Option<u8> {
    for high in (4..=12).rev() {
        let window: u16 = 0b1_1111 << (high - 4);
        if mask & window == window {
            return Some(high as u8);
        }
    }
    // Колесо: A, 2, 3, 4, 5 — старшая карта пятёрка (индекс 3).
    let wheel: u16 = (1u16 << 12) | 0b1111;
    if mask & wheel == wheel {
        return Some(3);
    }
    None
}
