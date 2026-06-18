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
    use kev::{CardInt, Rank, Suit};
    use rstest::rstest;

    pub(crate) fn card(suit: Suit, rank: Rank) -> CardInt {
        CardInt::from_u8((suit as u8) << 4 | rank as u8).expect("valid card")
    }

    pub(crate) fn hand(cards: &[CardInt]) -> BaccHand {
        let mut h = BaccHand::default();
        for c in cards {
            h.take(c);
        }
        h
    }

    #[rstest]
    #[case(Rank::Ace, 1)]
    #[case(Rank::Deuce, 2)]
    #[case(Rank::Trey, 3)]
    #[case(Rank::Four, 4)]
    #[case(Rank::Five, 5)]
    #[case(Rank::Six, 6)]
    #[case(Rank::Seven, 7)]
    #[case(Rank::Eight, 8)]
    #[case(Rank::Nine, 9)]
    #[case(Rank::Ten, 10)]
    #[case(Rank::Jack, 10)]
    #[case(Rank::Queen, 10)]
    #[case(Rank::King, 10)]
    fn pip_value_all_ranks(#[case] rank: Rank, #[case] expected: u8) {
        assert_eq!(pip_value(card(Suit::Club, rank)), expected);
    }
}
