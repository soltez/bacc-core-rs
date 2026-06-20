//! `bacc-core`: Primitive `no_std` types for the bacc ecosystem.
#![no_std]

pub mod round;
pub mod scoreboard;

pub use round::{BaccHand, BaccOutcome, BaccRound};
pub use scoreboard::BaccScoreboard;

use arrayvec::{ArrayString, ArrayVec};
use core::fmt::Write as _;
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

pub(crate) fn bytes_to_hex<const IN: usize, const OUT: usize>(
    bytes: &ArrayVec<u8, IN>,
) -> ArrayString<OUT> {
    let mut s = ArrayString::new();
    for &b in bytes.as_slice() {
        write!(s, "{b:02x}").expect("hex fits capacity");
    }
    s
}

pub(crate) fn hex_to_bytes<const N: usize>(hex: &str) -> ArrayVec<u8, N> {
    let mut v = ArrayVec::new();
    let nibs = hex.as_bytes();
    let offset = nibs.len() % 2;
    if offset == 1 {
        v.push(hex_to_nib(nibs[0]));
    }
    for chunk in nibs[offset..].chunks(2) {
        v.push((hex_to_nib(chunk[0]) << 4) | hex_to_nib(chunk[1]));
    }
    v
}

fn hex_to_nib(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{BaccHand, bytes_to_hex, hex_to_bytes, hex_to_nib, pip_value};
    use arrayvec::ArrayVec;
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

    #[test]
    fn bytes_to_hex_single_byte_with_leading_zero() {
        let mut v: ArrayVec<u8, 4> = ArrayVec::new();
        v.push(0x09);
        assert_eq!(bytes_to_hex::<4, 8>(&v).as_str(), "09");
    }

    #[test]
    fn bytes_to_hex_multiple_bytes() {
        let mut v: ArrayVec<u8, 4> = ArrayVec::new();
        v.push(0x09);
        v.push(0x03);
        v.push(0xab);
        assert_eq!(bytes_to_hex::<4, 8>(&v).as_str(), "0903ab");
    }

    #[test]
    fn hex_to_bytes_empty() {
        let v: ArrayVec<u8, 4> = hex_to_bytes("");
        assert_eq!(v.as_slice(), &[]);
    }

    #[test]
    fn hex_to_bytes_even_length() {
        let v: ArrayVec<u8, 4> = hex_to_bytes("0903ab");
        assert_eq!(v.as_slice(), &[0x09, 0x03, 0xab]);
    }

    #[test]
    fn hex_to_bytes_odd_length_leading_zero_implied() {
        let v: ArrayVec<u8, 4> = hex_to_bytes("abc");
        assert_eq!(v.as_slice(), &[0x0a, 0xbc]);
    }

    #[test]
    fn bytes_to_hex_and_hex_to_bytes_roundtrip() {
        let mut v: ArrayVec<u8, 4> = ArrayVec::new();
        v.push(0x09);
        v.push(0x03);
        v.push(0xab);
        let hex = bytes_to_hex::<4, 8>(&v);
        let decoded: ArrayVec<u8, 4> = hex_to_bytes(hex.as_str());
        assert_eq!(decoded.as_slice(), v.as_slice());
    }

    #[rstest]
    #[case(b'0', 0)]
    #[case(b'9', 9)]
    #[case(b'a', 10)]
    #[case(b'f', 15)]
    #[case(b'A', 10)]
    #[case(b'F', 15)]
    #[case(b'x', 0)]
    fn hex_to_nib_cases(#[case] input: u8, #[case] expected: u8) {
        assert_eq!(hex_to_nib(input), expected);
    }
}
