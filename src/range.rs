//! Диапазоны рук с весами и эквити против диапазона.
//!
//! Диапазон — 169 классов (пары, сьюты, офсьюты) с весами `0..=1000`.
//! Для расчёта он разворачивается в комбо, блокированные карты выкидываются,
//! а эквити считается точным перебором (флоп/тёрн/ривер) или Монте-Карло
//! (префлоп).

use core::fmt;

use crate::cards::{Card, Rank, Suit};
use crate::equity::Rng;
use crate::eval::{EvalError, eval_unchecked};

const CLASSES: usize = 169;
const WEIGHT_SCALE: u16 = 1000;
const ZERO_CARD: Card = Card::new(Rank::Two, Suit::Clubs);

/// Ошибка разбора диапазона.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeError {
    /// Неизвестный класс или некорректная запись (`AK`, `AXs`, `1h`).
    BadClass(String),
    /// Вес вне `0..=1` / `0..=100%`.
    BadWeight(String),
    /// Пустая строка диапазона.
    Empty,
}

impl fmt::Display for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangeError::BadClass(class) => write!(f, "неизвестный класс диапазона: {class:?}"),
            RangeError::BadWeight(weight) => write!(f, "некорректный вес диапазона: {weight:?}"),
            RangeError::Empty => write!(f, "пустой диапазон"),
        }
    }
}

impl std::error::Error for RangeError {}

/// Диапазон рук: 169 классов с весами.
///
/// Вес класса — `0..=1000` внутри крейта; в нотации — `0..=1` или `0..=100%`.
/// Комбо класса наследуют вес класса.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    weights: [u16; CLASSES],
}

impl Default for Range {
    fn default() -> Self {
        Self::new()
    }
}

impl Range {
    /// Пустой диапазон.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            weights: [0; CLASSES],
        }
    }

    /// Диапазон из 169 весов классов.
    #[must_use]
    pub const fn from_matrix(weights: [u16; CLASSES]) -> Self {
        Self { weights }
    }

    /// 169 весов классов.
    #[must_use]
    pub const fn matrix(&self) -> [u16; CLASSES] {
        self.weights
    }

    /// Разобрать нотацию: `22+`, `A2s+`, `KQo:0.5`, `AA:80%`.
    ///
    /// Токены разделяются запятыми или пробелами. Плюс означает диапазон:
    /// `22+` — все пары от 22 до AA, `A2s+` — A2s..AKs.
    pub fn parse(text: &str) -> Result<Self, RangeError> {
        let mut range = Self::new();
        let mut any = false;
        for token in text.split(|c: char| c == ',' || c.is_whitespace()) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            any = true;
            let (class_text, weight) = match token.split_once(':') {
                Some((class, weight)) => (class, parse_weight(weight)?),
                None => (token, WEIGHT_SCALE),
            };
            for (high, low, suited) in parse_class(class_text)? {
                range.weights[class_index(high, low, suited)] = weight;
            }
        }
        if !any {
            return Err(RangeError::Empty);
        }
        Ok(range)
    }

    /// Установить вес класса (например, `"AKs"`), `0..=1`.
    pub fn set_class_weight(&mut self, class: &str, weight: f64) -> Result<(), RangeError> {
        if !(0.0..=1.0).contains(&weight) {
            return Err(RangeError::BadWeight(format!("{weight}")));
        }
        let value = (weight * f64::from(WEIGHT_SCALE)).round() as u16;
        for (high, low, suited) in parse_class(class)? {
            self.weights[class_index(high, low, suited)] = value;
        }
        Ok(())
    }

    /// Суммарный вес всех классов.
    #[must_use]
    pub fn total_weight(&self) -> u32 {
        self.weights.iter().map(|&w| u32::from(w)).sum()
    }

    /// Есть ли хоть один класс с ненулевым весом.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_weight() == 0
    }

    /// Сколько комбо диапазона не заблокировано картами `dead`.
    #[must_use]
    pub fn combo_count(&self, dead: &[Card]) -> u32 {
        let mut mask = 0u64;
        for &card in dead {
            mask |= 1u64 << card.index();
        }
        eligible_combos(self, mask).len() as u32
    }
}

/// Эквити Hero против диапазона.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeEquity {
    /// Доля побед Hero.
    pub wins: f64,
    /// Доля побед диапазона.
    pub losses: f64,
    /// Доля сплитов.
    pub ties: f64,
    /// Число точных ранаутов или сэмплов MC.
    pub samples: u64,
    /// `true` — полный перебор; `false` — Монте-Карло.
    pub exact: bool,
    /// Сколько комбо диапазона участвовало.
    pub combos: u32,
}

impl RangeEquity {
    /// Эквити Hero: `wins + ties / 2`.
    #[must_use]
    pub fn hero_equity(&self) -> f64 {
        self.wins + self.ties / 2.0
    }

    /// Эквити диапазона: `losses + ties / 2`.
    #[must_use]
    pub fn villain_equity(&self) -> f64 {
        self.losses + self.ties / 2.0
    }
}

/// Точное эквити против диапазона: полный перебор бордов.
///
/// `board` — 3, 4 или 5 карт; префлоп точным перебором не считается
/// ([`EvalError::ExactTooLarge`]), для него — [`equity_vs_range_mc`].
///
/// # Ошибки
///
/// Неверный борд, дубль карты, пустой диапазон.
pub fn equity_vs_range(
    hero: [Card; 2],
    board: &[Card],
    range: &Range,
    dead: &[Card],
) -> Result<RangeEquity, EvalError> {
    validate_board(board)?;
    if board.is_empty() {
        return Err(EvalError::ExactTooLarge);
    }
    let blocked = blocked_mask(hero, board, dead)?;
    let combos = eligible_combos(range, blocked);
    if combos.is_empty() {
        return Err(EvalError::EmptyRange);
    }

    let deck = deck_excluding(blocked);
    let mut board_full = [ZERO_CARD; 5];
    board_full[..board.len()].copy_from_slice(board);
    let mut run = ExactRun {
        hero,
        board: board_full,
        combos: &combos,
        deck: &deck,
        tally: Tally::default(),
    };
    run.walk(5 - board.len(), 0, board.len(), 0);
    run.tally.into_equity(true, combos.len() as u32)
}

/// Монте-Карло эквити против диапазона (в том числе префлоп).
///
/// Комбо сэмплируется по весам, затем случайный ранаут. `seed == 0` допустим.
///
/// # Ошибки
///
/// Неверный борд, дубль карты, пустой диапазон, `samples == 0`.
pub fn equity_vs_range_mc(
    hero: [Card; 2],
    board: &[Card],
    range: &Range,
    dead: &[Card],
    samples: u32,
    seed: u64,
) -> Result<RangeEquity, EvalError> {
    validate_board(board)?;
    if samples == 0 {
        return Err(EvalError::EmptySamples);
    }
    let blocked = blocked_mask(hero, board, dead)?;
    let combos = eligible_combos(range, blocked);
    if combos.is_empty() {
        return Err(EvalError::EmptyRange);
    }

    let base = deck_excluding(blocked);
    let need = 5 - board.len();
    if base.len() < need + 2 {
        return Err(EvalError::NotEnoughRemainingCards);
    }

    let mut cdf = Vec::with_capacity(combos.len());
    let mut total = 0u64;
    for combo in &combos {
        total += u64::from(combo.weight);
        cdf.push(total);
    }

    let mut rng = Rng::new(seed);
    let mut board_full = [ZERO_CARD; 5];
    board_full[..board.len()].copy_from_slice(board);
    let mut deck = [ZERO_CARD; 52];
    let mut runs = 0u64;
    let mut tally = Tally::default();

    for _ in 0..samples {
        let pick = rng.next_u64() % total;
        let index = cdf.partition_point(|&weight| weight <= pick);
        let combo = &combos[index.min(combos.len() - 1)];

        let mut len = 0;
        for &card in &base {
            if card == combo.first || card == combo.second {
                continue;
            }
            deck[len] = card;
            len += 1;
        }
        for slot in 0..need {
            let pick = slot + (rng.next_u64() % (len - slot) as u64) as usize;
            deck.swap(slot, pick);
            board_full[board.len() + slot] = deck[slot];
        }
        tally.record(hero, combo, &board_full, 1);
        runs += 1;
    }

    let mut equity = tally.into_equity(false, combos.len() as u32)?;
    equity.samples = runs;
    Ok(equity)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ComboWeight {
    pub(crate) first: Card,
    pub(crate) second: Card,
    pub(crate) weight: u32,
    pub(crate) mask: u64,
}

#[derive(Default)]
struct Tally {
    wins: u64,
    losses: u64,
    ties: u64,
    weight_sum: u64,
    boards: u64,
}

impl Tally {
    fn record(&mut self, hero: [Card; 2], combo: &ComboWeight, board: &[Card; 5], weight: u64) {
        let hero_hand = [
            hero[0], hero[1], board[0], board[1], board[2], board[3], board[4],
        ];
        let villain_hand = [
            combo.first,
            combo.second,
            board[0],
            board[1],
            board[2],
            board[3],
            board[4],
        ];
        self.weight_sum += weight;
        match eval_unchecked(&hero_hand).cmp(&eval_unchecked(&villain_hand)) {
            core::cmp::Ordering::Greater => self.wins += weight,
            core::cmp::Ordering::Less => self.losses += weight,
            core::cmp::Ordering::Equal => self.ties += weight,
        }
    }
}

struct ExactRun<'a> {
    hero: [Card; 2],
    board: [Card; 5],
    combos: &'a [ComboWeight],
    deck: &'a [Card],
    tally: Tally,
}

impl ExactRun<'_> {
    fn walk(&mut self, need: usize, start: usize, filled: usize, runout_mask: u64) {
        if need == 0 {
            self.record(runout_mask);
            return;
        }
        let last = self.deck.len() - need;
        for index in start..=last {
            let card = self.deck[index];
            self.board[filled] = card;
            self.walk(
                need - 1,
                index + 1,
                filled + 1,
                runout_mask | (1u64 << card.index()),
            );
        }
    }

    fn record(&mut self, runout_mask: u64) {
        self.tally.boards += 1;
        for combo in self.combos {
            if combo.mask & runout_mask != 0 {
                continue;
            }
            let hero_hand = [
                self.hero[0],
                self.hero[1],
                self.board[0],
                self.board[1],
                self.board[2],
                self.board[3],
                self.board[4],
            ];
            let villain_hand = [
                combo.first,
                combo.second,
                self.board[0],
                self.board[1],
                self.board[2],
                self.board[3],
                self.board[4],
            ];
            self.tally.weight_sum += u64::from(combo.weight);
            match eval_unchecked(&hero_hand).cmp(&eval_unchecked(&villain_hand)) {
                core::cmp::Ordering::Greater => self.tally.wins += u64::from(combo.weight),
                core::cmp::Ordering::Less => self.tally.losses += u64::from(combo.weight),
                core::cmp::Ordering::Equal => self.tally.ties += u64::from(combo.weight),
            }
        }
    }
}

impl Tally {
    fn into_equity(self, exact: bool, combos: u32) -> Result<RangeEquity, EvalError> {
        if self.weight_sum == 0 {
            return Err(EvalError::EmptyRange);
        }
        let divisor = self.weight_sum as f64;
        let samples = if exact { self.boards } else { self.weight_sum };
        Ok(RangeEquity {
            wins: self.wins as f64 / divisor,
            losses: self.losses as f64 / divisor,
            ties: self.ties as f64 / divisor,
            samples,
            exact,
            combos,
        })
    }
}

pub(crate) fn validate_board(board: &[Card]) -> Result<(), EvalError> {
    if !matches!(board.len(), 0 | 3 | 4 | 5) {
        return Err(EvalError::BadBoard { cards: board.len() });
    }
    Ok(())
}

fn blocked_mask(hero: [Card; 2], board: &[Card], dead: &[Card]) -> Result<u64, EvalError> {
    let mut mask = 0u64;
    for &card in hero.iter().chain(board.iter()).chain(dead.iter()) {
        let bit = 1u64 << card.index();
        if mask & bit != 0 {
            return Err(EvalError::DuplicateCard(card));
        }
        mask |= bit;
    }
    Ok(mask)
}

fn deck_excluding(mask: u64) -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    for index in 0..52u8 {
        if mask & (1u64 << index) != 0 {
            continue;
        }
        if let Some(card) = Card::from_index(index) {
            deck.push(card);
        }
    }
    deck
}

pub(crate) fn eligible_combos(range: &Range, blocked: u64) -> Vec<ComboWeight> {
    let mut combos = Vec::new();
    for first in 0..52u8 {
        let Some(a) = Card::from_index(first) else {
            continue;
        };
        let a_bit = 1u64 << first;
        if a_bit & blocked != 0 {
            continue;
        }
        for second in first + 1..52u8 {
            let Some(b) = Card::from_index(second) else {
                continue;
            };
            let b_bit = 1u64 << second;
            if b_bit & blocked != 0 {
                continue;
            }
            let index = combo_class(a, b);
            let weight = u32::from(range.weights[index]);
            if weight == 0 {
                continue;
            }
            combos.push(ComboWeight {
                first: a,
                second: b,
                weight,
                mask: a_bit | b_bit,
            });
        }
    }
    combos
}

fn combo_class(first: Card, second: Card) -> usize {
    let a = first.rank().index();
    let b = second.rank().index();
    let high = a.max(b);
    let low = a.min(b);
    if high == low {
        class_index(high, low, false)
    } else {
        class_index(high, low, first.suit() == second.suit())
    }
}

fn class_index(high: u8, low: u8, suited: bool) -> usize {
    if high == low {
        usize::from(high) * 13 + usize::from(low)
    } else if suited {
        // Матрица 13×13: пары на диагонали, сьюты выше неё, офсьюты ниже.
        usize::from(high) * 13 + usize::from(low)
    } else {
        usize::from(low) * 13 + usize::from(high)
    }
}

fn parse_weight(text: &str) -> Result<u16, RangeError> {
    let text = text.trim();
    let (number, percent) = match text.strip_suffix('%') {
        Some(number) => (number, true),
        None => (text, false),
    };
    let value: f64 = number
        .trim()
        .parse()
        .map_err(|_| RangeError::BadWeight(text.to_string()))?;
    if !value.is_finite() || value < 0.0 {
        return Err(RangeError::BadWeight(text.to_string()));
    }
    let normalized = if percent || value > 1.0 {
        value / 100.0
    } else {
        value
    };
    if normalized > 1.0 {
        return Err(RangeError::BadWeight(text.to_string()));
    }
    Ok((normalized * f64::from(WEIGHT_SCALE)).round() as u16)
}

fn parse_class(text: &str) -> Result<Vec<(u8, u8, bool)>, RangeError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(RangeError::BadClass(text.to_string()));
    }
    let (body, plus) = match text.strip_suffix('+') {
        Some(body) => (body, true),
        None => (text, false),
    };
    let chars: Vec<char> = body.chars().collect();
    if chars.len() < 2 || chars.len() > 3 {
        return Err(RangeError::BadClass(text.to_string()));
    }
    let first = rank_index(chars[0]).ok_or_else(|| RangeError::BadClass(text.to_string()))?;
    let second = rank_index(chars[1]).ok_or_else(|| RangeError::BadClass(text.to_string()))?;
    let high = first.max(second);
    let low = first.min(second);

    if high == low {
        if chars.len() == 3 {
            return Err(RangeError::BadClass(text.to_string()));
        }
        if plus {
            return Ok((low..=12).map(|rank| (rank, rank, false)).collect());
        }
        return Ok(vec![(high, low, false)]);
    }

    if chars.len() != 3 {
        return Err(RangeError::BadClass(text.to_string()));
    }
    let suited = match chars[2] {
        's' | 'S' => true,
        'o' | 'O' => false,
        _ => return Err(RangeError::BadClass(text.to_string())),
    };
    if plus {
        return Ok((low..high).map(|l| (high, l, suited)).collect());
    }
    Ok(vec![(high, low, suited)])
}

fn rank_index(c: char) -> Option<u8> {
    Some(match c.to_ascii_uppercase() {
        '2' => 0,
        '3' => 1,
        '4' => 2,
        '5' => 3,
        '6' => 4,
        '7' => 5,
        '8' => 6,
        '9' => 7,
        'T' => 8,
        'J' => 9,
        'Q' => 10,
        'K' => 11,
        'A' => 12,
        _ => return None,
    })
}
