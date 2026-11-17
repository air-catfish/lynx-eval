//! Fast, dependency-free 5–7 card poker hand evaluator and exact hand-vs-hand equity.
//!
//! The crate is deliberately small and honest: it knows cards, hand strength,
//! equity, ranges with weights and range-vs-range equity, and nothing about
//! solvers or strategy. Every number it returns is either an exact count over all
//! remaining boards or a Monte-Carlo estimate that says so via `exact`.
//!
//! # Example
//!
//! ```
//! use lynx_eval::{Card, HandCategory, describe, evaluate};
//!
//! let hand: Vec<Card> = ["As", "Ks", "Qs", "Js", "Ts"]
//!     .iter()
//!     .map(|text| text.parse().unwrap())
//!     .collect();
//! let value = evaluate(&hand).unwrap();
//!
//! assert_eq!(value.category(), HandCategory::StraightFlush);
//! assert_eq!(describe(value).ranks[0].unwrap().index(), 12); // туз
//! ```
//!
//! # Границы
//!
//! - Ноль внешних зависимостей и ноль `unsafe`.
//! - Невалидный вход (мало/много карт, дубль, борд из 1–2 карт) возвращает
//!   [`EvalError`]; крейт не паникует и не «угадывает».
//! - `HandValue` — упакованный `u32`; его числовое представление не является
//!   стабильным контрактом.
//!
//! # Лицензия
//!
//! Apache-2.0, см. `LICENSE`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod cards;
mod describe;
mod equity;
mod eval;
mod range;
mod range_vs_range;

pub use cards::{Card, CardError, DECK_SIZE, Rank, Suit};
pub use describe::{HandDescription, describe};
pub use equity::{Equity, equity_vs_hand, equity_vs_hand_mc};
pub use eval::{EvalError, HandCategory, HandValue, best_five, evaluate};
pub use range::{Range, RangeEquity, RangeError, equity_vs_range, equity_vs_range_mc};
pub use range_vs_range::{
    EXACT_COMPARISON_BUDGET, RangeVsRangeEquity, equity_range_vs_range, equity_range_vs_range_mc,
};
