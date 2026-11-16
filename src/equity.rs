//! Эквити «рука против руки»: точный перебор всех бордов и Монте-Карло.

use core::cmp::Ordering;

use crate::cards::{Card, DECK_SIZE, Rank, Suit};
use crate::eval::{EvalError, eval_unchecked};

const ZERO_CARD: Card = Card::new(Rank::Two, Suit::Clubs);

/// Исход «рука против руки».
///
/// Три исхода нормированы: `wins + losses + ties == 1`. Эквити Hero
/// ([`Equity::hero_equity`]) — это `wins + ties / 2`; сплит делится пополам.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Equity {
    /// Доля бордов, где Hero выигрывает в одиночку.
    pub wins: f64,
    /// Доля бордов, где выигрывает Villain.
    pub losses: f64,
    /// Доля бордов со сплитом.
    pub ties: f64,
    /// Число разобранных бордов (точных или сэмплированных).
    pub samples: u64,
    /// `true` — перебраны все борды; `false` — Монте-Карло.
    pub exact: bool,
}

impl Equity {
    /// Эквити Hero: `wins + ties / 2`.
    #[must_use]
    pub fn hero_equity(&self) -> f64 {
        self.wins + self.ties / 2.0
    }

    /// Эквити Villain: `losses + ties / 2`.
    #[must_use]
    pub fn villain_equity(&self) -> f64 {
        self.losses + self.ties / 2.0
    }
}

/// Точное эквити: перебираются все борды до пяти карт.
///
/// `board` — пустой префлоп или 3, 4, 5 карт. `dead` — известные мёртвые карты
/// (например, показанные вскрытые карты, которые не участвуют в раздаче);
/// пересечение с руками и бордом — ошибка.
///
/// # Стоимость
///
/// Префлоп — `C(48, 5) = 1 712 304` борда и две оценки на борд. На флопе, терне
/// и ривере — 990, 44 и 1 борд соответственно.
///
/// # Ошибки
///
/// [`EvalError::BadBoard`], [`EvalError::DuplicateCard`],
/// [`EvalError::NotEnoughRemainingCards`].
pub fn equity_vs_hand(
    hero: [Card; 2],
    villain: [Card; 2],
    board: &[Card],
    dead: &[Card],
) -> Result<Equity, EvalError> {
    let state = Prepared::new(hero, villain, board, dead)?;
    let mut board_full = state.board;
    let need = 5 - state.board_len;
    let mut tally = Tally::default();
    enumerate(
        &state.deck[..state.len],
        need,
        0,
        &mut board_full,
        state.board_len,
        &state,
        &mut tally,
    );
    Ok(tally.into_equity(true))
}

/// Монте-Карло эквити: `samples` случайных бордов, воспроизводимо по `seed`.
///
/// `seed == 0` допустим — генератор подставляет ненулевое стартовое состояние.
/// Тот же `seed` и те же карты дают тот же результат.
///
/// # Ошибки
///
/// Те же, что у [`equity_vs_hand`], плюс [`EvalError::EmptySamples`] при
/// `samples == 0`.
pub fn equity_vs_hand_mc(
    hero: [Card; 2],
    villain: [Card; 2],
    board: &[Card],
    dead: &[Card],
    samples: u32,
    seed: u64,
) -> Result<Equity, EvalError> {
    if samples == 0 {
        return Err(EvalError::EmptySamples);
    }
    let state = Prepared::new(hero, villain, board, dead)?;
    let need = 5 - state.board_len;
    let mut deck = state.deck;
    let mut board_full = state.board;
    let mut rng = Rng::new(seed);
    let mut tally = Tally::default();
    for _ in 0..samples {
        for slot in 0..need {
            let remaining = (state.len - slot) as u64;
            let pick = slot + (rng.next_u64() % remaining) as usize;
            deck.swap(slot, pick);
            board_full[state.board_len + slot] = deck[slot];
        }
        tally.record(state.hero, state.villain, board_full);
    }
    Ok(tally.into_equity(false))
}

struct Prepared {
    deck: [Card; DECK_SIZE],
    len: usize,
    hero: [Card; 2],
    villain: [Card; 2],
    board: [Card; 5],
    board_len: usize,
}

impl Prepared {
    fn new(
        hero: [Card; 2],
        villain: [Card; 2],
        board: &[Card],
        dead: &[Card],
    ) -> Result<Self, EvalError> {
        if !matches!(board.len(), 0 | 3 | 4 | 5) {
            return Err(EvalError::BadBoard { cards: board.len() });
        }

        let mut seen = 0u64;
        for &card in hero
            .iter()
            .chain(villain.iter())
            .chain(board.iter())
            .chain(dead.iter())
        {
            let bit = 1u64 << card.index();
            if seen & bit != 0 {
                return Err(EvalError::DuplicateCard(card));
            }
            seen |= bit;
        }

        let mut deck = [ZERO_CARD; DECK_SIZE];
        let mut len = 0;
        for index in 0..DECK_SIZE as u8 {
            if seen & (1u64 << index) == 0
                && let Some(card) = Card::from_index(index)
            {
                deck[len] = card;
                len += 1;
            }
        }
        let need = 5 - board.len();
        if len < need {
            return Err(EvalError::NotEnoughRemainingCards);
        }

        let mut board_full = [ZERO_CARD; 5];
        board_full[..board.len()].copy_from_slice(board);
        Ok(Prepared {
            deck,
            len,
            hero,
            villain,
            board: board_full,
            board_len: board.len(),
        })
    }
}

#[derive(Default)]
struct Tally {
    wins: u64,
    losses: u64,
    ties: u64,
}

impl Tally {
    fn record(&mut self, hero: [Card; 2], villain: [Card; 2], board: [Card; 5]) {
        let hero_hand = [
            hero[0], hero[1], board[0], board[1], board[2], board[3], board[4],
        ];
        let villain_hand = [
            villain[0], villain[1], board[0], board[1], board[2], board[3], board[4],
        ];
        match eval_unchecked(&hero_hand).cmp(&eval_unchecked(&villain_hand)) {
            Ordering::Greater => self.wins += 1,
            Ordering::Less => self.losses += 1,
            Ordering::Equal => self.ties += 1,
        }
    }

    fn into_equity(self, exact: bool) -> Equity {
        let samples = self.wins + self.losses + self.ties;
        let divisor = samples as f64;
        Equity {
            wins: self.wins as f64 / divisor,
            losses: self.losses as f64 / divisor,
            ties: self.ties as f64 / divisor,
            samples,
            exact,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn enumerate(
    deck: &[Card],
    need: usize,
    start: usize,
    board: &mut [Card; 5],
    filled: usize,
    state: &Prepared,
    tally: &mut Tally,
) {
    if need == 0 {
        tally.record(state.hero, state.villain, *board);
        return;
    }
    let last = deck.len() - need;
    for index in start..=last {
        board[filled] = deck[index];
        enumerate(deck, need - 1, index + 1, board, filled + 1, state, tally);
    }
}

/// xorshift64*: крошечный детерминированный генератор без зависимостей.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}
