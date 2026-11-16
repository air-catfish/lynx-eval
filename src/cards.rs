//! Карты: масти, достоинства и разбор строк вида `"As"`.

use core::fmt;
use core::str::FromStr;

/// Число карт в стандартной колоде.
pub const DECK_SIZE: usize = 52;

/// Достоинство карты.
///
/// Порядок вариантов — старшинство: [`Rank::Two`] младше [`Rank::Ace`],
/// поэтому производный `Ord` годится для сравнения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Rank {
    /// Двойка.
    Two = 0,
    /// Тройка.
    Three = 1,
    /// Четвёрка.
    Four = 2,
    /// Пятёрка.
    Five = 3,
    /// Шестёрка.
    Six = 4,
    /// Семёрка.
    Seven = 5,
    /// Восьмёрка.
    Eight = 6,
    /// Девятка.
    Nine = 7,
    /// Десятка.
    Ten = 8,
    /// Валет.
    Jack = 9,
    /// Дама.
    Queen = 10,
    /// Король.
    King = 11,
    /// Туз.
    Ace = 12,
}

impl Rank {
    /// Все достоинства в порядке возрастания.
    pub const ALL: [Rank; 13] = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    /// Достоинство по индексу `0..13` (`0` — двойка, `12` — туз).
    #[must_use]
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }

    /// Индекс `0..13`.
    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    fn from_char(c: char) -> Option<Self> {
        Some(match c.to_ascii_uppercase() {
            '2' => Rank::Two,
            '3' => Rank::Three,
            '4' => Rank::Four,
            '5' => Rank::Five,
            '6' => Rank::Six,
            '7' => Rank::Seven,
            '8' => Rank::Eight,
            '9' => Rank::Nine,
            'T' => Rank::Ten,
            'J' => Rank::Jack,
            'Q' => Rank::Queen,
            'K' => Rank::King,
            'A' => Rank::Ace,
            _ => return None,
        })
    }

    fn to_char(self) -> char {
        match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

/// Масть.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Suit {
    /// Трефы.
    Clubs = 0,
    /// Бубны.
    Diamonds = 1,
    /// Червы.
    Hearts = 2,
    /// Пики.
    Spades = 3,
}

impl Suit {
    /// Все масти.
    pub const ALL: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

    /// Масть по индексу `0..4`.
    #[must_use]
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }

    /// Индекс `0..4`.
    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    fn from_char(c: char) -> Option<Self> {
        Some(match c.to_ascii_lowercase() {
            'c' => Suit::Clubs,
            'd' => Suit::Diamonds,
            'h' => Suit::Hearts,
            's' => Suit::Spades,
            _ => return None,
        })
    }

    fn to_char(self) -> char {
        match self {
            Suit::Clubs => 'c',
            Suit::Diamonds => 'd',
            Suit::Hearts => 'h',
            Suit::Spades => 's',
        }
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

/// Игральная карта.
///
/// Индекс карты — `rank * 4 + suit` в диапазоне `0..52`; это удобно для
/// битовых масок. Порядок `Ord` — по достоинству, затем по масти; правилам
/// покера он не противоречит, но и не нужен для сравнения рук.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Card {
    rank: Rank,
    suit: Suit,
}

impl Card {
    /// Карта из достоинства и масти.
    #[must_use]
    pub const fn new(rank: Rank, suit: Suit) -> Self {
        Card { rank, suit }
    }

    /// Карта по индексу `0..52` (`rank * 4 + suit`).
    #[must_use]
    pub fn from_index(index: u8) -> Option<Self> {
        if index >= DECK_SIZE as u8 {
            return None;
        }
        let rank = Rank::from_index(index >> 2)?;
        let suit = Suit::from_index(index & 3)?;
        Some(Card { rank, suit })
    }

    /// Разбор строки из двух символов: `"As"`, `"Td"`, `"7c"`.
    ///
    /// Регистр не важен (как в парсере GG-логов): `"aS"` и `"As"` — одна карта.
    pub fn parse(text: &str) -> Result<Self, CardError> {
        let mut chars = text.chars();
        let Some(rank_char) = chars.next() else {
            return Err(CardError::BadLength);
        };
        let Some(suit_char) = chars.next() else {
            return Err(CardError::BadLength);
        };
        if chars.next().is_some() {
            return Err(CardError::BadLength);
        }
        let rank = Rank::from_char(rank_char).ok_or(CardError::UnknownRank(rank_char))?;
        let suit = Suit::from_char(suit_char).ok_or(CardError::UnknownSuit(suit_char))?;
        Ok(Card { rank, suit })
    }

    /// Индекс карты `0..52`.
    #[must_use]
    pub const fn index(self) -> u8 {
        self.rank.index() * 4 + self.suit.index()
    }

    /// Достоинство.
    #[must_use]
    pub const fn rank(self) -> Rank {
        self.rank
    }

    /// Масть.
    #[must_use]
    pub const fn suit(self) -> Suit {
        self.suit
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

impl FromStr for Card {
    type Err = CardError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// Ошибка разбора карты.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardError {
    /// Не два символа.
    BadLength,
    /// Неизвестное достоинство.
    UnknownRank(char),
    /// Неизвестная масть.
    UnknownSuit(char),
}

impl fmt::Display for CardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CardError::BadLength => write!(f, "карта должна состоять из двух символов"),
            CardError::UnknownRank(c) => write!(f, "неизвестное достоинство: {c:?}"),
            CardError::UnknownSuit(c) => write!(f, "неизвестная масть: {c:?}"),
        }
    }
}

impl std::error::Error for CardError {}
