//! `bacc-core`: Primitive `no_std` types for the bacc ecosystem.
#![no_std]

pub mod round;
pub mod scoreboard;

pub use round::{BaccHand, BaccOutcome, BaccRound};
pub use scoreboard::{BaccBeadPlate, BaccBigRoad, BaccDerivedRoads, BaccScoreboard};

use kev::{CardInt, Rank};

/// Returns the pip value of a single [`CardInt`].
///
/// | Rank                   | Value |
/// |------------------------|-------|
/// | Ace                    | 1     |
/// | 2-9                    | pip   |
/// | Ten, Jack, Queen, King | 10    |
#[must_use]
pub fn pip_value(card: CardInt) -> u8 {
    match card.rank() {
        Rank::Ace => 1,
        Rank::King | Rank::Queen | Rank::Jack | Rank::Ten => 10,
        Rank::Nine => 9,
        Rank::Eight => 8,
        Rank::Seven => 7,
        Rank::Six => 6,
        Rank::Five => 5,
        Rank::Four => 4,
        Rank::Trey => 3,
        Rank::Deuce => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::{BaccHand, pip_value};
    use kev::CardInt;
    use rstest::rstest;

    pub(crate) fn hand(cards: &[CardInt]) -> BaccHand {
        let mut h = BaccHand::default();
        for c in cards {
            h.take(c);
        }
        h
    }

    #[rstest]
    #[case(CardInt::CardAc, 1)]
    #[case(CardInt::Card2c, 2)]
    #[case(CardInt::Card3c, 3)]
    #[case(CardInt::Card4c, 4)]
    #[case(CardInt::Card5c, 5)]
    #[case(CardInt::Card6c, 6)]
    #[case(CardInt::Card7c, 7)]
    #[case(CardInt::Card8c, 8)]
    #[case(CardInt::Card9c, 9)]
    #[case(CardInt::CardTc, 10)]
    #[case(CardInt::CardJc, 10)]
    #[case(CardInt::CardQc, 10)]
    #[case(CardInt::CardKc, 10)]
    fn pip_value_all_ranks(#[case] card: CardInt, #[case] expected: u8) {
        assert_eq!(pip_value(card), expected);
    }
}
