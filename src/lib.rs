//! `bacc-core`: Primitive `no_std` types for the bacc ecosystem.
#![no_std]

use arrayvec::ArrayVec;
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

/// A baccarat hand holding the cards dealt to one side (player or banker).
#[derive(Default)]
pub struct BaccHand {
    cards: ArrayVec<CardInt, 3>,
}

impl BaccHand {
    /// Adds `card` to this hand.
    pub fn take(&mut self, card: &CardInt) {
        self.cards.push(*card);
    }

    /// Returns the baccarat point value of the hand (0-9).
    ///
    /// Sums the pip value for each card in the hand and reduces the total
    /// modulo 10 - matching standard baccarat scoring rules.
    #[must_use]
    pub fn value(&self) -> u8 {
        let total: u8 = self.cards.iter().map(|&x| pip_value(x)).sum();
        total % 10
    }

    /// Returns `true` if the first two cards share the same rank.
    ///
    /// # Panics
    ///
    /// Panics if the hand contains fewer than two cards.
    #[must_use]
    pub fn is_pair(&self) -> bool {
        self.cards[0].rank() == self.cards[1].rank()
    }

    /// Returns `true` if the hand contains exactly three cards.
    #[must_use]
    pub fn has_third(&self) -> bool {
        self.cards.len() == 3
    }

    /// Returns a slice of the cards held in this hand.
    #[must_use]
    pub fn cards(&self) -> &[CardInt] {
        &self.cards
    }
}

/// The derived outcome of a single baccarat round.
///
/// Holds the winner, pair flags, third-card flags, and final hand values for
/// both sides. Constructed via [`BaccRound::outcome`] and serialisable
/// to/from a compact `u32` via [`encode`] and [`decode`].
///
/// [`encode`]: BaccOutcome::encode
/// [`decode`]: BaccOutcome::decode
pub struct BaccOutcome {
    /// Winner: `0x1` = player, `0x2` = banker, `0x3` = tie.
    marker: u8,
    /// Pair flags: `0x1` = player pair, `0x2` = banker pair, `0x3` = both.
    pairs: u8,
    /// Third-card flags: `0x1` = player drew, `0x2` = banker drew, `0x3` = both.
    thirds: u8,
    player_value: u8,
    banker_value: u8,
}

impl BaccOutcome {
    /// Encodes this outcome into a `u32`.
    ///
    /// ## Bit layout
    ///
    /// | Bits  | Field             | Values                                               |
    /// |-------|-------------------|------------------------------------------------------|
    /// | 0-1   | Marker            | `1` = player, `2` = banker, `3` = tie               |
    /// | 2     | Player pair       | `1` if player's first two cards share a rank         |
    /// | 3     | Banker pair       | `1` if banker's first two cards share a rank         |
    /// | 4     | Player third card | `1` if player drew a third card                      |
    /// | 5     | Banker third card | `1` if banker drew a third card                      |
    /// | 8-11  | Player hand value | Player's hand value 0-9                              |
    /// | 12-15 | Banker hand value | Banker's hand value 0-9                              |
    #[must_use]
    pub fn encode(&self) -> u32 {
        u32::from(self.marker)
            | u32::from(self.pairs) << 2
            | u32::from(self.thirds) << 4
            | u32::from(self.player_value) << 8
            | u32::from(self.banker_value) << 12
    }

    /// Decodes a [`BaccOutcome`] from the `u32` produced by [`BaccOutcome::encode`].
    #[must_use]
    pub fn decode(encoded: u32) -> Self {
        Self {
            banker_value: ((encoded >> 12) & 0xF) as u8,
            player_value: ((encoded >> 8) & 0xF) as u8,
            thirds: ((encoded >> 4) & 0x3) as u8,
            pairs: ((encoded >> 2) & 0x3) as u8,
            marker: (encoded & 0x3) as u8,
        }
    }

    /// Returns the winner of the round.
    ///
    /// - `0x1` = player wins
    /// - `0x2` = banker wins
    /// - `0x3` = tie
    #[must_use]
    pub fn marker(&self) -> u8 {
        self.marker
    }

    /// Returns the player's final hand value (0-9).
    #[must_use]
    pub fn player_value(&self) -> u8 {
        self.player_value
    }

    /// Returns the banker's final hand value (0-9).
    #[must_use]
    pub fn banker_value(&self) -> u8 {
        self.banker_value
    }

    /// Returns the pair flags for this round.
    ///
    /// - `0x0` = no pair
    /// - `0x1` = player pair only
    /// - `0x2` = banker pair only
    /// - `0x3` = both pairs
    #[must_use]
    pub fn pairs(&self) -> u8 {
        self.pairs
    }

    /// Returns the third-card flags for this round.
    ///
    /// - `0x0` = neither side drew a third card
    /// - `0x1` = player drew a third card only
    /// - `0x2` = banker drew a third card only
    /// - `0x3` = both sides drew a third card
    #[must_use]
    pub fn thirds(&self) -> u8 {
        self.thirds
    }
}

/// A single resolved baccarat round, holding the final hands for both sides.
pub struct BaccRound {
    player: BaccHand,
    banker: BaccHand,
    banker_forced_third: bool,
    cut_card_index: Option<u8>,
}

impl BaccRound {
    /// Creates a new [`BaccRound`].
    #[must_use]
    pub fn new(
        player: BaccHand,
        banker: BaccHand,
        banker_forced_third: bool,
        cut_card_index: Option<u8>,
    ) -> Self {
        Self {
            player,
            banker,
            banker_forced_third,
            cut_card_index,
        }
    }

    /// Encodes the full card sequence and metadata of this round into a `u64`.
    ///
    /// ## Bit layout
    ///
    /// | Bits  | Field          | Notes                                                        |
    /// |-------|----------------|--------------------------------------------------------------|
    /// | 55-52 | Reserved       | 0                                                            |
    /// | 51    | Forced third   | `1` if banker score was 0-2 when player drew a third card    |
    /// | 50-48 | Cut card index | 1-indexed: `0` = none, `1` = `Some(0)`, ..., `6` = `Some(5)`|
    /// | 47-40 | Banker card 3  | `cdhsrrrr`, or `0` if not drawn                             |
    /// | 39-32 | Player card 3  | `cdhsrrrr`, or `0` if not drawn                             |
    /// | 31-24 | Banker card 2  | `cdhsrrrr`                                                   |
    /// | 23-16 | Player card 2  | `cdhsrrrr`                                                   |
    /// | 15-8  | Banker card 1  | `cdhsrrrr`                                                   |
    /// | 7-0   | Player card 1  | `cdhsrrrr`                                                   |
    ///
    /// Each `cdhsrrrr` byte packs the Cactus Kev suit nibble (one-hot, bits 7-4:
    /// clubs=8, diamonds=4, hearts=2, spades=1) and rank index (bits 3-0:
    /// deuce=0, trey=1, ..., ace=12). A card slot that was not dealt is `0`.
    ///
    /// # Panics
    ///
    /// Panics if either hand contains fewer than two cards.
    #[must_use]
    pub fn encode(&self) -> u64 {
        let cut = self.cut_card_index.map_or(0u8, |n| n + 1);
        let aux_nib = (u8::from(self.banker_forced_third) << 3) | cut;
        let p = &self.player.cards;
        let b = &self.banker.cards;
        let p2 = if p.len() > 2 { p[2].to_u8() } else { 0 };
        let b2 = if b.len() > 2 { b[2].to_u8() } else { 0 };
        u64::from(aux_nib) << 48
            | u64::from(b2) << 40
            | u64::from(p2) << 32
            | u64::from(b[1].to_u8()) << 24
            | u64::from(p[1].to_u8()) << 16
            | u64::from(b[0].to_u8()) << 8
            | u64::from(p[0].to_u8())
    }

    /// Decodes a [`BaccRound`] from the `u64` produced by [`BaccRound::encode`].
    ///
    /// # Panics
    ///
    /// Panics if any of the four mandatory card bytes decode to `None`, which would
    /// indicate a corrupted encoding.
    #[must_use]
    pub fn decode(encoded: u64) -> Self {
        let bytes = encoded.to_le_bytes();
        let p0 = bytes[0];
        let b0 = bytes[1];
        let p1 = bytes[2];
        let b1 = bytes[3];
        let p2 = bytes[4];
        let b2 = bytes[5];
        let aux_nib = bytes[6];
        let banker_forced_third = (aux_nib >> 3) & 1 != 0;
        let cut_raw = aux_nib & 0x7;
        let cut_card_index = if cut_raw == 0 {
            None
        } else {
            Some(cut_raw - 1)
        };
        let mut player = BaccHand::default();
        let mut banker = BaccHand::default();
        player.take(&CardInt::from_u8(p0).expect("p0 must be valid"));
        banker.take(&CardInt::from_u8(b0).expect("b0 must be valid"));
        player.take(&CardInt::from_u8(p1).expect("p1 must be valid"));
        banker.take(&CardInt::from_u8(b1).expect("b1 must be valid"));
        if let Some(card) = CardInt::from_u8(p2) {
            player.take(&card);
        }
        if let Some(card) = CardInt::from_u8(b2) {
            banker.take(&card);
        }
        Self {
            player,
            banker,
            banker_forced_third,
            cut_card_index,
        }
    }

    /// Returns the derived outcome of this round as a [`BaccOutcome`].
    ///
    /// # Panics
    ///
    /// Panics if either hand contains fewer than two cards.
    #[must_use]
    pub fn outcome(&self) -> BaccOutcome {
        let player_value = self.player.value();
        let banker_value = self.banker.value();
        let marker = match player_value.cmp(&banker_value) {
            core::cmp::Ordering::Greater => 0x1,
            core::cmp::Ordering::Less => 0x2,
            core::cmp::Ordering::Equal => 0x3,
        };
        BaccOutcome {
            marker,
            pairs: u8::from(self.player.is_pair()) | (u8::from(self.banker.is_pair()) << 1),
            thirds: u8::from(self.player.has_third()) | (u8::from(self.banker.has_third()) << 1),
            player_value,
            banker_value,
        }
    }

    /// Returns a slice of the player's cards.
    #[must_use]
    pub fn player_cards(&self) -> &[CardInt] {
        self.player.cards()
    }

    /// Returns a slice of the banker's cards.
    #[must_use]
    pub fn banker_cards(&self) -> &[CardInt] {
        self.banker.cards()
    }

    /// Returns `true` if the banker's pre-draw score was 0-2 and the player drew a third card.
    #[must_use]
    pub fn is_forced_third(&self) -> bool {
        self.banker_forced_third
    }

    /// Returns the index of the cut card within this round's card sequence, if it was encountered.
    ///
    /// The index counts dealt card positions starting at 0:
    /// - 0: player card 1
    /// - 1: banker card 1
    /// - 2: player card 2
    /// - 3: banker card 2
    /// - 4: player card 3 (if drawn)
    /// - 5: banker card 3 (if drawn)
    ///
    /// The value also signals shoe exhaustion:
    /// - `Some(0)`: cut card was already past when this round started; this is the last round.
    /// - `Some(1..=5)`: cut card was consumed during this round; exactly one more round will be dealt.
    /// - `None`: cut card was not encountered.
    #[must_use]
    pub fn cut_card_index(&self) -> Option<u8> {
        self.cut_card_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kev::{CardInt, Suit};
    use rstest::rstest;

    fn card(suit: Suit, rank: Rank) -> CardInt {
        CardInt::from_u8((suit as u8) << 4 | rank as u8).expect("valid card")
    }

    fn hand(cards: &[CardInt]) -> BaccHand {
        let mut h = BaccHand::default();
        for c in cards {
            h.take(c);
        }
        h
    }

    // --- pip_value ---

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

    // --- BaccRound and BaccOutcome full-chain roundtrips ---

    #[test]
    fn roundtrip_player_wins_no_pairs_no_thirds() {
        // player: Eight(8) + Ace(1) = 9; banker: Deuce(2) + Trey(3) = 5
        let p = hand(&[card(Suit::Club, Rank::Eight), card(Suit::Heart, Rank::Ace)]);
        let b = hand(&[
            card(Suit::Diamond, Rank::Deuce),
            card(Suit::Spade, Rank::Trey),
        ]);
        let round = BaccRound::new(p, b, false, None);
        let decoded_round = BaccRound::decode(round.encode());
        assert_eq!(decoded_round.player_cards(), round.player_cards());
        assert_eq!(decoded_round.banker_cards(), round.banker_cards());
        assert!(!decoded_round.is_forced_third());
        assert_eq!(decoded_round.cut_card_index(), None);
        let outcome = round.outcome();
        assert_eq!(outcome.marker(), 0x1);
        assert_eq!(outcome.pairs(), 0x0);
        assert_eq!(outcome.thirds(), 0x0);
        assert_eq!(outcome.player_value(), 9);
        assert_eq!(outcome.banker_value(), 5);
        let decoded_outcome = BaccOutcome::decode(outcome.encode());
        assert_eq!(decoded_outcome.marker(), 0x1);
        assert_eq!(decoded_outcome.pairs(), 0x0);
        assert_eq!(decoded_outcome.thirds(), 0x0);
        assert_eq!(decoded_outcome.player_value(), 9);
        assert_eq!(decoded_outcome.banker_value(), 5);
    }

    #[test]
    fn roundtrip_banker_wins_both_thirds() {
        // player: Ace(1) + Deuce(2) + Trey(3) = 6; banker: Five(5) + Six(6) + Seven(7) = 8
        let p = hand(&[
            card(Suit::Club, Rank::Ace),
            card(Suit::Heart, Rank::Deuce),
            card(Suit::Diamond, Rank::Trey),
        ]);
        let b = hand(&[
            card(Suit::Spade, Rank::Five),
            card(Suit::Club, Rank::Six),
            card(Suit::Heart, Rank::Seven),
        ]);
        let round = BaccRound::new(p, b, true, None);
        let decoded_round = BaccRound::decode(round.encode());
        assert_eq!(decoded_round.player_cards(), round.player_cards());
        assert_eq!(decoded_round.banker_cards(), round.banker_cards());
        assert!(decoded_round.is_forced_third());
        assert_eq!(decoded_round.cut_card_index(), None);
        let outcome = round.outcome();
        assert_eq!(outcome.marker(), 0x2);
        assert_eq!(outcome.pairs(), 0x0);
        assert_eq!(outcome.thirds(), 0x3);
        assert_eq!(outcome.player_value(), 6);
        assert_eq!(outcome.banker_value(), 8);
        let decoded_outcome = BaccOutcome::decode(outcome.encode());
        assert_eq!(decoded_outcome.marker(), 0x2);
        assert_eq!(decoded_outcome.pairs(), 0x0);
        assert_eq!(decoded_outcome.thirds(), 0x3);
        assert_eq!(decoded_outcome.player_value(), 6);
        assert_eq!(decoded_outcome.banker_value(), 8);
    }

    #[test]
    fn roundtrip_tie_both_pairs() {
        // player: Trey(3) + Trey(3) = 6; banker: Eight(8) + Eight(8) = 6; tie, both stand
        let p = hand(&[card(Suit::Club, Rank::Trey), card(Suit::Heart, Rank::Trey)]);
        let b = hand(&[
            card(Suit::Diamond, Rank::Eight),
            card(Suit::Spade, Rank::Eight),
        ]);
        let round = BaccRound::new(p, b, false, None);
        let decoded_round = BaccRound::decode(round.encode());
        assert_eq!(decoded_round.player_cards(), round.player_cards());
        assert_eq!(decoded_round.banker_cards(), round.banker_cards());
        assert!(!decoded_round.is_forced_third());
        assert_eq!(decoded_round.cut_card_index(), None);
        let outcome = round.outcome();
        assert_eq!(outcome.marker(), 0x3);
        assert_eq!(outcome.pairs(), 0x3);
        assert_eq!(outcome.thirds(), 0x0);
        assert_eq!(outcome.player_value(), 6);
        assert_eq!(outcome.banker_value(), 6);
        let decoded_outcome = BaccOutcome::decode(outcome.encode());
        assert_eq!(decoded_outcome.marker(), 0x3);
        assert_eq!(decoded_outcome.pairs(), 0x3);
        assert_eq!(decoded_outcome.thirds(), 0x0);
        assert_eq!(decoded_outcome.player_value(), 6);
        assert_eq!(decoded_outcome.banker_value(), 6);
    }

    #[test]
    fn roundtrip_cut_card_index() {
        // player: Trey(3) + Four(4) = 7; banker: Six(6) + Ace(1) = 7; tie, both stand
        let p = hand(&[card(Suit::Club, Rank::Trey), card(Suit::Heart, Rank::Four)]);
        let b = hand(&[card(Suit::Diamond, Rank::Six), card(Suit::Spade, Rank::Ace)]);
        let round = BaccRound::new(p, b, false, Some(3));
        let decoded_round = BaccRound::decode(round.encode());
        assert_eq!(decoded_round.player_cards(), round.player_cards());
        assert_eq!(decoded_round.banker_cards(), round.banker_cards());
        assert!(!decoded_round.is_forced_third());
        assert_eq!(decoded_round.cut_card_index(), Some(3));
        let outcome = round.outcome();
        assert_eq!(outcome.marker(), 0x3);
        assert_eq!(outcome.pairs(), 0x0);
        assert_eq!(outcome.thirds(), 0x0);
        assert_eq!(outcome.player_value(), 7);
        assert_eq!(outcome.banker_value(), 7);
        let decoded_outcome = BaccOutcome::decode(outcome.encode());
        assert_eq!(decoded_outcome.marker(), 0x3);
        assert_eq!(decoded_outcome.pairs(), 0x0);
        assert_eq!(decoded_outcome.thirds(), 0x0);
        assert_eq!(decoded_outcome.player_value(), 7);
        assert_eq!(decoded_outcome.banker_value(), 7);
    }

    #[test]
    fn roundtrip_forced_third_cut_card_zero() {
        let p = hand(&[
            card(Suit::Club, Rank::Deuce),
            card(Suit::Heart, Rank::Trey),
            card(Suit::Diamond, Rank::Four),
        ]);
        let b = hand(&[
            card(Suit::Spade, Rank::Ace),
            card(Suit::Club, Rank::Deuce),
            card(Suit::Heart, Rank::Trey),
        ]);
        let round = BaccRound::new(p, b, true, Some(0));
        let decoded_round = BaccRound::decode(round.encode());
        assert_eq!(decoded_round.player_cards(), round.player_cards());
        assert_eq!(decoded_round.banker_cards(), round.banker_cards());
        assert!(decoded_round.is_forced_third());
        assert_eq!(decoded_round.cut_card_index(), Some(0));
        let outcome = round.outcome();
        assert_eq!(outcome.marker(), 0x1);
        assert_eq!(outcome.pairs(), 0x0);
        assert_eq!(outcome.thirds(), 0x3);
        assert_eq!(outcome.player_value(), 9);
        assert_eq!(outcome.banker_value(), 6);
        let decoded_outcome = BaccOutcome::decode(outcome.encode());
        assert_eq!(decoded_outcome.marker(), 0x1);
        assert_eq!(decoded_outcome.pairs(), 0x0);
        assert_eq!(decoded_outcome.thirds(), 0x3);
        assert_eq!(decoded_outcome.player_value(), 9);
        assert_eq!(decoded_outcome.banker_value(), 6);
    }
}
