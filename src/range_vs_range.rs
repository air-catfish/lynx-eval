//! Эквити «диапазон против диапазона»: точный перебор и Монте-Карло.
//!
//! В отличие от [`equity_vs_range`](crate::equity_vs_range), здесь обе стороны —
//! диапазоны. Считаются только **совместимые** пары комбо (без общих карт);
//! карты борда и мёртвые карты выкидываются с обеих сторон. Вес пары —
//! `weight_hero * weight_villain` (вес класса из [`Range`]).
//!
//! Точный перебор разрешён только при небольшом объёме работы: число
//! совместимых пар, умноженное на число ранаутов, не должно превышать
//! [`EXACT_COMPARISON_BUDGET`]. Иначе — [`EvalError::ExactTooLarge`], и считать
//! нужно Монте-Карло: тихого переключения режима нет.

use core::cmp::Ordering;

use crate::cards::{Card, DECK_SIZE, Rank, Suit};
use crate::equity::Rng;
use crate::eval::{EvalError, eval_unchecked};
use crate::range::{ComboWeight, Range, eligible_combos, validate_board};

/// Предел работы точного перебора: число сравнений «пара комбо × ранаут».
///
/// Пять миллионов сравнений — это около 10 млн оценок 7-карточных рук, разумная
/// цена для нажатия кнопки. Широкий флоп и префлоп в бюджет не помещаются — для
/// них честно возвращается [`EvalError::ExactTooLarge`] и предлагается
/// Монте-Карло.
pub const EXACT_COMPARISON_BUDGET: u64 = 5_000_000;

/// Сколько раз Монте-Карло может пересэмплировать пару комбо, прежде чем
/// сдаться. Диапазоны почти полностью совпадают — редкость, но зависать нельзя.
const BASE_REJECTIONS: u64 = 1_000_000;
const REJECTIONS_PER_SAMPLE: u64 = 20;

const ZERO_CARD: Card = Card::new(Rank::Two, Suit::Clubs);

/// Результат эквити «диапазон против диапазона».
///
/// Три исхода нормированы: `hero_wins + villain_wins + ties == 1` — с точностью
/// до плавающей точки. Сплит делится пополам: `hero_equity + villain_equity == 1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeVsRangeEquity {
    /// Доля побед Hero.
    pub hero_wins: f64,
    /// Доля побед Villain.
    pub villain_wins: f64,
    /// Доля сплитов.
    pub ties: f64,
    /// Эквити Hero: `hero_wins + ties / 2`.
    pub hero_equity: f64,
    /// Эквити Villain: `villain_wins + ties / 2`.
    pub villain_equity: f64,
    /// Комбо Hero после блокеров борда и мёртвых карт.
    pub hero_combos: u32,
    /// Комбо Villain после блокеров борда и мёртвых карт.
    pub villain_combos: u32,
    /// Точные сравнения «пара × ранаут» или число сэмплов Монте-Карло.
    pub samples: u64,
    /// `true` — полный перебор; `false` — Монте-Карло.
    pub exact: bool,
}

/// Точное эквити «диапазон против диапазона».
///
/// Перебираются все совместимые пары комбо и все ранауты. Бюджет —
/// [`EXACT_COMPARISON_BUDGET`]: при превышении возвращается
/// [`EvalError::ExactTooLarge`], а не частичный или «досчитанный» результат.
///
/// # Ошибки
///
/// [`EvalError::BadBoard`], [`EvalError::DuplicateCard`],
/// [`EvalError::NotEnoughRemainingCards`], [`EvalError::EmptyRange`] (нет комбо
/// или нет ни одной совместимой пары), [`EvalError::ExactTooLarge`].
pub fn equity_range_vs_range(
    hero: &Range,
    villain: &Range,
    board: &[Card],
    dead: &[Card],
) -> Result<RangeVsRangeEquity, EvalError> {
    validate_board(board)?;
    let (base_mask, base_count) = base_blockers(board, dead)?;

    let hero_combos = eligible_combos(hero, base_mask);
    let villain_combos = eligible_combos(villain, base_mask);
    if hero_combos.is_empty() || villain_combos.is_empty() {
        return Err(EvalError::EmptyRange);
    }

    let need = 5 - board.len();
    let remaining = DECK_SIZE as u64 - u64::from(base_count) - 4;
    if remaining < need as u64 {
        return Err(EvalError::NotEnoughRemainingCards);
    }
    let runouts = choose(remaining, need as u64);

    // Точный объём работы известен заранее: пары × ранауты. Перебор
    // прекращается, как только бюджет превышен, — не пересчитывая весь список.
    let mut work = 0u64;
    for first in &hero_combos {
        for second in &villain_combos {
            if first.mask & second.mask != 0 {
                continue;
            }
            work = work.saturating_add(runouts);
            if work > EXACT_COMPARISON_BUDGET {
                return Err(EvalError::ExactTooLarge);
            }
        }
    }
    if work == 0 {
        return Err(EvalError::EmptyRange);
    }

    let mut board_full = [ZERO_CARD; 5];
    board_full[..board.len()].copy_from_slice(board);
    let mut deck = [ZERO_CARD; DECK_SIZE];
    let mut tally = Tally::default();

    for first in &hero_combos {
        for second in &villain_combos {
            if first.mask & second.mask != 0 {
                continue;
            }
            let len = fill_deck(&mut deck, base_mask | first.mask | second.mask);
            let weight = u64::from(first.weight) * u64::from(second.weight);
            let mut run = ExactRun {
                board: &mut board_full,
                board_len: board.len(),
                hero: first,
                villain: second,
                weight,
                tally: &mut tally,
            };
            run.walk(&deck[..len], need, 0, 0);
        }
    }

    tally.into_equity(hero_combos.len() as u32, villain_combos.len() as u32, true)
}

/// Монте-Карло эквити «диапазон против диапазона».
///
/// Пара комбо сэмплируется по весам с учётом несовместимости (вес пары
/// пропорционален `weight_hero * weight_villain`), затем сэмплируется ранаут.
/// `seed == 0` допустим; тот же `seed` и те же диапазоны дают тот же результат.
///
/// # Ошибки
///
/// Те же, что у [`equity_range_vs_range`], плюс [`EvalError::EmptySamples`] при
/// `samples == 0` и [`EvalError::SamplingFailed`], если совместимую пару не
/// удалось выбрать за отведённое число попыток.
pub fn equity_range_vs_range_mc(
    hero: &Range,
    villain: &Range,
    board: &[Card],
    dead: &[Card],
    samples: u32,
    seed: u64,
) -> Result<RangeVsRangeEquity, EvalError> {
    validate_board(board)?;
    if samples == 0 {
        return Err(EvalError::EmptySamples);
    }
    let (base_mask, base_count) = base_blockers(board, dead)?;

    let hero_combos = eligible_combos(hero, base_mask);
    let villain_combos = eligible_combos(villain, base_mask);
    if hero_combos.is_empty() || villain_combos.is_empty() {
        return Err(EvalError::EmptyRange);
    }

    let need = 5 - board.len();
    let remaining = DECK_SIZE as u64 - u64::from(base_count) - 4;
    if remaining < need as u64 {
        return Err(EvalError::NotEnoughRemainingCards);
    }
    if !has_compatible_pair(&hero_combos, &villain_combos) {
        return Err(EvalError::EmptyRange);
    }

    let (hero_cdf, hero_total) = cumulative(&hero_combos);
    let (villain_cdf, villain_total) = cumulative(&villain_combos);

    let mut base_deck = [ZERO_CARD; DECK_SIZE];
    let mut base_len = 0;
    for index in 0..DECK_SIZE as u8 {
        if base_mask & (1u64 << index) != 0 {
            continue;
        }
        if let Some(card) = Card::from_index(index) {
            base_deck[base_len] = card;
            base_len += 1;
        }
    }

    let mut rng = Rng::new(seed);
    let mut board_full = [ZERO_CARD; 5];
    board_full[..board.len()].copy_from_slice(board);
    let mut deck = [ZERO_CARD; DECK_SIZE];
    let mut tally = Tally::default();
    let mut rejections = 0u64;
    // Вероятность совместимой пары при классовых весах не меньше 1/6, но
    // предел всё равно масштабируется от `samples`: зависание хуже ошибки.
    let max_rejections = u64::from(samples)
        .saturating_mul(REJECTIONS_PER_SAMPLE)
        .saturating_add(BASE_REJECTIONS);

    for _ in 0..samples {
        let (first, second) = loop {
            let first = pick(&hero_combos, &hero_cdf, hero_total, &mut rng);
            let second = pick(&villain_combos, &villain_cdf, villain_total, &mut rng);
            if first.mask & second.mask == 0 {
                break (first, second);
            }
            rejections += 1;
            if rejections > max_rejections {
                return Err(EvalError::SamplingFailed);
            }
        };

        let pair_mask = first.mask | second.mask;
        let mut len = 0;
        for &card in base_deck.iter().take(base_len) {
            if pair_mask & (1u64 << card.index()) != 0 {
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
        tally.record(first, second, &board_full, 1);
    }

    tally.into_equity(hero_combos.len() as u32, villain_combos.len() as u32, false)
}

/// Маска карт борда и мёртвых карт; дубль — ошибка.
fn base_blockers(board: &[Card], dead: &[Card]) -> Result<(u64, u32), EvalError> {
    let mut mask = 0u64;
    let mut count = 0u32;
    for &card in board.iter().chain(dead.iter()) {
        let bit = 1u64 << card.index();
        if mask & bit != 0 {
            return Err(EvalError::DuplicateCard(card));
        }
        mask |= bit;
        count += 1;
    }
    Ok((mask, count))
}

/// Записать в `deck` все карты колоды, кроме исключённых; вернуть их число.
fn fill_deck(deck: &mut [Card; DECK_SIZE], excluded: u64) -> usize {
    let mut len = 0;
    for index in 0..DECK_SIZE as u8 {
        if excluded & (1u64 << index) != 0 {
            continue;
        }
        if let Some(card) = Card::from_index(index) {
            deck[len] = card;
            len += 1;
        }
    }
    len
}

/// `C(n, k)` без переполнения: при выходе за `u64` — `u64::MAX`.
fn choose(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result: u128 = 1;
    for step in 0..k as u128 {
        result = result * (u128::from(n) - step) / (step + 1);
    }
    u64::try_from(result).unwrap_or(u64::MAX)
}

fn has_compatible_pair(hero: &[ComboWeight], villain: &[ComboWeight]) -> bool {
    hero.iter()
        .any(|first| villain.iter().any(|second| first.mask & second.mask == 0))
}

fn cumulative(combos: &[ComboWeight]) -> (Vec<u64>, u64) {
    let mut cdf = Vec::with_capacity(combos.len());
    let mut total = 0u64;
    for combo in combos {
        total += u64::from(combo.weight);
        cdf.push(total);
    }
    (cdf, total)
}

fn pick<'a>(combos: &'a [ComboWeight], cdf: &[u64], total: u64, rng: &mut Rng) -> &'a ComboWeight {
    let value = rng.next_u64() % total;
    let index = cdf.partition_point(|&weight| weight <= value);
    &combos[index.min(combos.len() - 1)]
}

#[derive(Default)]
struct Tally {
    hero_wins: u64,
    villain_wins: u64,
    ties: u64,
    weight_sum: u64,
    comparisons: u64,
}

impl Tally {
    fn record(
        &mut self,
        first: &ComboWeight,
        second: &ComboWeight,
        board: &[Card; 5],
        weight: u64,
    ) {
        let hero_hand = [
            first.first,
            first.second,
            board[0],
            board[1],
            board[2],
            board[3],
            board[4],
        ];
        let villain_hand = [
            second.first,
            second.second,
            board[0],
            board[1],
            board[2],
            board[3],
            board[4],
        ];
        self.comparisons += 1;
        self.weight_sum += weight;
        match eval_unchecked(&hero_hand).cmp(&eval_unchecked(&villain_hand)) {
            Ordering::Greater => self.hero_wins += weight,
            Ordering::Less => self.villain_wins += weight,
            Ordering::Equal => self.ties += weight,
        }
    }

    fn into_equity(
        self,
        hero_combos: u32,
        villain_combos: u32,
        exact: bool,
    ) -> Result<RangeVsRangeEquity, EvalError> {
        if self.weight_sum == 0 {
            return Err(EvalError::EmptyRange);
        }
        let divisor = self.weight_sum as f64;
        let hero_wins = self.hero_wins as f64 / divisor;
        let villain_wins = self.villain_wins as f64 / divisor;
        let ties = self.ties as f64 / divisor;
        Ok(RangeVsRangeEquity {
            hero_wins,
            villain_wins,
            ties,
            hero_equity: hero_wins + ties / 2.0,
            villain_equity: villain_wins + ties / 2.0,
            hero_combos,
            villain_combos,
            samples: if exact {
                self.comparisons
            } else {
                self.weight_sum
            },
            exact,
        })
    }
}

struct ExactRun<'a> {
    board: &'a mut [Card; 5],
    board_len: usize,
    hero: &'a ComboWeight,
    villain: &'a ComboWeight,
    weight: u64,
    tally: &'a mut Tally,
}

impl ExactRun<'_> {
    fn walk(&mut self, deck: &[Card], need: usize, start: usize, filled: usize) {
        if need == 0 {
            self.tally
                .record(self.hero, self.villain, self.board, self.weight);
            return;
        }
        let last = deck.len() - need;
        for index in start..=last {
            self.board[self.board_len + filled] = deck[index];
            self.walk(deck, need - 1, index + 1, filled + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::choose;

    #[test]
    fn binomial_is_exact_and_saturates() {
        assert_eq!(choose(48, 5), 1_712_304);
        assert_eq!(choose(45, 2), 990);
        assert_eq!(choose(44, 1), 44);
        assert_eq!(choose(5, 0), 1);
        assert_eq!(choose(3, 4), 0);
        assert_eq!(
            choose(u64::MAX, 2),
            u64::MAX,
            "переполнение не оборачивается"
        );
    }
}
