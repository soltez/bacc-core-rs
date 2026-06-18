//! Baccarat scoreboard tracking the five standard road displays.

use arrayvec::{ArrayString, ArrayVec};
use core::fmt::Write as _;

use crate::{BaccOutcome, BaccRound};

const MAX_ROUNDS: usize = 96;
const BEAD_PLATE_CAP: usize = MAX_ROUNDS * 2;
const BIG_ROAD_CAP: usize = MAX_ROUNDS * 3;
const DERIVED_ROAD_CAP: usize = MAX_ROUNDS;

fn bytes_to_hex<const IN: usize, const OUT: usize>(bytes: &ArrayVec<u8, IN>) -> ArrayString<OUT> {
    let mut s = ArrayString::new();
    for &b in bytes.as_slice() {
        write!(s, "{b:02x}").expect("hex fits capacity");
    }
    s
}

fn hex_to_bytes<const N: usize>(hex: &str) -> ArrayVec<u8, N> {
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

/// The bead plate road as a sequence of bead words.
///
/// Each bead word is two bytes: bits 15-8 = winner's hand value, bits 5-4 = third card flags,
/// bits 3-2 = pair flags, bits 1-0 = outcome marker.
pub struct BaccBeadPlate(ArrayVec<u8, BEAD_PLATE_CAP>);

impl BaccBeadPlate {
    /// Encodes the bead plate as a lowercase hex string, oldest bead at the left.
    #[must_use]
    pub fn encode(&self) -> ArrayString<{ BEAD_PLATE_CAP * 2 }> {
        bytes_to_hex::<BEAD_PLATE_CAP, { BEAD_PLATE_CAP * 2 }>(&self.0)
    }

    /// Decodes a [`BaccBeadPlate`] from a lowercase hex string produced by [`BaccBeadPlate::encode`].
    #[must_use]
    pub fn decode(hex: &str) -> Self {
        Self(hex_to_bytes(hex))
    }
}

/// The big road as a sequence of column records.
///
/// Each column record occupies (2n + 1) bytes where n is the row count:
/// two bytes per row (hand value nibble + outcome byte) followed by one row-count byte.
pub struct BaccBigRoad(ArrayVec<u8, BIG_ROAD_CAP>);

impl BaccBigRoad {
    /// Encodes the big road as a lowercase hex string, oldest column at the left.
    #[must_use]
    pub fn encode(&self) -> ArrayString<{ BIG_ROAD_CAP * 2 }> {
        bytes_to_hex::<BIG_ROAD_CAP, { BIG_ROAD_CAP * 2 }>(&self.0)
    }

    /// Decodes a [`BaccBigRoad`] from a lowercase hex string produced by [`BaccBigRoad::encode`].
    #[must_use]
    pub fn decode(hex: &str) -> Self {
        Self(hex_to_bytes(hex))
    }
}

/// The three derived roads: Big Eye Boy, Small Road, and Cockroach Pig.
///
/// Each road is run-length encoded: bits 7-1 = run length, bit 0 = icon (1 = red, 0 = blue).
pub struct BaccDerivedRoads([ArrayVec<u8, DERIVED_ROAD_CAP>; 3]);

impl BaccDerivedRoads {
    /// Encodes all three derived roads as lowercase hex strings.
    #[must_use]
    pub fn encode(&self) -> [ArrayString<{ DERIVED_ROAD_CAP * 2 }>; 3] {
        [
            bytes_to_hex::<DERIVED_ROAD_CAP, { DERIVED_ROAD_CAP * 2 }>(&self.0[0]),
            bytes_to_hex::<DERIVED_ROAD_CAP, { DERIVED_ROAD_CAP * 2 }>(&self.0[1]),
            bytes_to_hex::<DERIVED_ROAD_CAP, { DERIVED_ROAD_CAP * 2 }>(&self.0[2]),
        ]
    }

    /// Decodes a [`BaccDerivedRoads`] from three lowercase hex strings produced by
    /// [`BaccDerivedRoads::encode`].
    #[must_use]
    pub fn decode(hex: [&str; 3]) -> Self {
        Self([
            hex_to_bytes(hex[0]),
            hex_to_bytes(hex[1]),
            hex_to_bytes(hex[2]),
        ])
    }
}

/// Tracks the five standard baccarat scoreboards for a running shoe.
///
/// Call [`update`] after each round to advance all five boards.
///
/// [`update`]: BaccScoreboard::update
pub struct BaccScoreboard {
    // Bead words in chronological order (oldest at index 0, newest at the end).
    // Each word is two bytes: bits 11-8 = winner's hand value, bits 5-4 = third card flags,
    // bits 3-2 = pair flags, bits 1-0 = outcome.
    bead_plate: ArrayVec<u8, BEAD_PLATE_CAP>,
    // Columns in chronological order (oldest at index 0, newest at the end).
    // Each column occupies (2n + 1) bytes (n = row count):
    //   byte 0     - ttttvvvv (bits 7-4 = tie count, bits 3-0 = hand value) of the first row
    //   byte 1     - xx33ppww (third card flags, pair flags, outcome) of the first row
    //   ...        - newer rows follow in the same two-byte pattern
    //   byte 2n-2  - ttttvvvv of the most recent row
    //   byte 2n-1  - xx33ppww of the most recent row
    //   byte 2n    - row count n
    big_road: ArrayVec<u8, BIG_ROAD_CAP>,
    // Row counts of the five most recent big road columns (index 0 = current).
    col_heights: [u8; 5],
    // [Big Eye Boy, Small Road, Cockroach Pig] - one run-length-encoded register each.
    // Each byte: bits 7-1 = run length, bit 0 = icon (1 = red, 0 = blue).
    derived_roads: [ArrayVec<u8, DERIVED_ROAD_CAP>; 3],
}

impl Default for BaccScoreboard {
    fn default() -> Self {
        Self::new()
    }
}

impl BaccScoreboard {
    /// Creates a new [`BaccScoreboard`] with all scoreboards zeroed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bead_plate: ArrayVec::new(),
            big_road: ArrayVec::new(),
            col_heights: [0; 5],
            derived_roads: [ArrayVec::new(), ArrayVec::new(), ArrayVec::new()],
        }
    }

    /// Updates all five scoreboards immediately after a completed round.
    pub fn update(&mut self, round: &BaccRound) {
        let outcome = round.outcome();
        let bead = Self::bead_word(&outcome);
        let is_tie = outcome.marker() == 0x3;
        self.update_bead_plate(bead);
        self.update_big_road(bead, is_tie);
        if !is_tie {
            self.update_derived_roads();
        }
    }

    /// Resets all five scoreboards to zero.
    pub fn clear(&mut self) {
        self.bead_plate.clear();
        self.big_road.clear();
        self.col_heights = [0; 5];
        for road in &mut self.derived_roads {
            road.clear();
        }
    }

    /// Returns the bead plate road.
    #[must_use]
    pub fn bead_plate(&self) -> BaccBeadPlate {
        BaccBeadPlate(self.bead_plate.clone())
    }

    /// Returns the big road.
    #[must_use]
    pub fn big_road(&self) -> BaccBigRoad {
        BaccBigRoad(self.big_road.clone())
    }

    /// Returns the three derived roads - Big Eye Boy, Small Road, Cockroach Pig.
    #[must_use]
    pub fn derived_roads(&self) -> BaccDerivedRoads {
        BaccDerivedRoads(self.derived_roads.clone())
    }

    /// Converts a [`BaccOutcome`] into a two-byte bead word for the bead plate.
    ///
    /// | Bits  | Content                                                            |
    /// |-------|--------------------------------------------------------------------|
    /// | 15-8  | Winner's hand value (0-9)                                          |
    /// | 7-6   | Unused (0)                                                         |
    /// | 5-4   | Third card flags (`01` = player, `10` = banker, `11` = both)      |
    /// | 3-2   | Pair flags (`01` = player, `10` = banker, `11` = both)            |
    /// | 1-0   | Outcome (`1` = player, `2` = banker, `3` = tie)                   |
    ///
    /// Banker wins use the banker's hand value; player wins and ties use the player's hand value.
    fn bead_word(outcome: &BaccOutcome) -> u16 {
        let hand_val = if outcome.marker() == 0x2 {
            outcome.banker_value()
        } else {
            outcome.player_value()
        };
        let low_byte = outcome.marker() | (outcome.pairs() << 2) | (outcome.thirds() << 4);
        u16::from(hand_val) << 8 | u16::from(low_byte)
    }

    fn update_bead_plate(&mut self, bead: u16) {
        self.bead_plate
            .try_extend_from_slice(&bead.to_be_bytes())
            .expect("bead_plate capacity not exceeded");
    }

    fn update_big_road(&mut self, bead: u16, is_tie: bool) {
        let [val, out] = bead.to_be_bytes();
        if self.big_road.is_empty() {
            if is_tie {
                self.big_road.push(0x10);
                self.big_road.push(out);
                self.big_road.push(0);
            } else {
                self.big_road.push(val);
                self.big_road.push(out);
                self.big_road.push(1);
                self.col_heights = [1, 0, 0, 0, 0];
            }
            return;
        }
        let len = self.big_road.len();
        let last_outcome = self.big_road[len - 2] & 0x3;
        let is_shoe_tie_start = last_outcome == 0x3;
        let is_column_hit = last_outcome == (out & 0x3);
        if is_tie {
            let b = self.big_road[len - 3];
            self.big_road[len - 3] = if b < 0xF0 { b + 0x10 } else { b };
        } else if is_shoe_tie_start {
            self.big_road[len - 3] |= val;
            self.big_road[len - 2] = out;
            self.big_road[len - 1] = 1;
            self.col_heights[0] = 1;
        } else if is_column_hit {
            let row_cnt = self.big_road.pop().expect("big_road is non-empty");
            self.big_road.push(val);
            self.big_road.push(out);
            self.big_road.push(row_cnt + 1);
            self.col_heights[0] += 1;
        } else {
            self.big_road.push(val);
            self.big_road.push(out);
            self.big_road.push(1);
            self.col_heights.copy_within(0..4, 1);
            self.col_heights[0] = 1;
        }
    }

    fn push_derived_road_icon(&mut self, road_idx: usize, icon: u8) {
        let road = &mut self.derived_roads[road_idx];
        if road.is_empty() {
            road.push(2 | icon);
        } else {
            let last_icon = road.last().unwrap() & 1;
            if icon == last_icon {
                *road.last_mut().unwrap() += 2;
            } else {
                road.push(2 | icon);
            }
        }
    }

    fn update_derived_roads(&mut self) {
        for i in 1..=3usize {
            let has_ref_col = self.col_heights[i + 1] > 0;
            let has_growing_col = self.col_heights[i] > 0 && self.col_heights[0] > 1;
            if !(has_ref_col || has_growing_col) {
                continue;
            }
            let icon: u8 = if self.col_heights[0] == 1 {
                u8::from(self.col_heights[i] == self.col_heights[i + 1])
            } else {
                u8::from(self.col_heights[0] != self.col_heights[i] + 1)
            };
            self.push_derived_road_icon(i - 1, icon);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BaccBeadPlate, BaccBigRoad, BaccDerivedRoads, BaccScoreboard, bytes_to_hex, hex_to_bytes,
        hex_to_nib,
    };
    use crate::BaccRound;
    use crate::tests::hand;
    use arrayvec::ArrayVec;
    use kev::CardInt;

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

    #[test]
    fn hex_to_nib_digit() {
        assert_eq!(hex_to_nib(b'0'), 0);
        assert_eq!(hex_to_nib(b'9'), 9);
    }

    #[test]
    fn hex_to_nib_lowercase() {
        assert_eq!(hex_to_nib(b'a'), 10);
        assert_eq!(hex_to_nib(b'f'), 15);
    }

    #[test]
    fn hex_to_nib_uppercase() {
        assert_eq!(hex_to_nib(b'A'), 10);
        assert_eq!(hex_to_nib(b'F'), 15);
    }

    #[test]
    fn hex_to_nib_invalid() {
        assert_eq!(hex_to_nib(b'x'), 0);
    }

    #[test]
    fn bead_plate_encode_decode_roundtrip() {
        let hex = "090306120902090307030801062109010802090107120731";
        assert_eq!(BaccBeadPlate::decode(hex).encode().as_str(), hex);
    }

    #[test]
    fn big_road_encode_decode_roundtrip() {
        let hex = "161229020208010621090103080201090101071201073101";
        assert_eq!(BaccBigRoad::decode(hex).encode().as_str(), hex);
    }

    #[test]
    fn derived_roads_encode_decode_roundtrip() {
        let hex = ["030605", "0403", "04"];
        let encoded = BaccDerivedRoads::decode(hex).encode();
        assert_eq!(encoded[0].as_str(), hex[0]);
        assert_eq!(encoded[1].as_str(), hex[1]);
        assert_eq!(encoded[2].as_str(), hex[2]);
    }

    #[test]
    fn all_scoreboards_accumulate_correctly_over_12_rounds() {
        // Round  1: P=[9d, Qh]     value=9 natural, B=[9c, Ts]     value=9 natural -> tie,         bead_word=0x0903
        // Round  2: P=[3c, Kd, 8c] value=1,         B=[6s, Jh]     value=6         -> banker wins, bead_word=0x0612
        // Round  3: P=[5d, 7c]     value=2,         B=[9h, Tc]     value=9 natural -> banker wins, bead_word=0x0902
        // Round  4: P=[Qs, 9d]     value=9 natural, B=[4s, 5s]     value=9 natural -> tie,         bead_word=0x0903
        // Round  5: P=[Ac, 6s]     value=7,         B=[7h, Kc]     value=7         -> tie,         bead_word=0x0703
        // Round  6: P=[Ah, 7s]     value=8 natural, B=[Ad, 6c]     value=7         -> player wins, bead_word=0x0801
        // Round  7: P=[6h, Qd]     value=6,         B=[2c, Kh, Ts] value=2         -> player wins, bead_word=0x0621
        // Round  8: P=[Ks, 9c]     value=9 natural, B=[8h, 7d]     value=5         -> player wins, bead_word=0x0901
        // Round  9: P=[9s, 2d]     value=1,         B=[8s, Tc]     value=8 natural -> banker wins, bead_word=0x0802
        // Round 10: P=[9h, Jd]     value=9 natural, B=[4d, 6d]     value=0         -> player wins, bead_word=0x0901
        // Round 11: P=[3s, Th, Js] value=3,         B=[9s, 8d]     value=7         -> banker wins, bead_word=0x0712
        // Round 12: P=[4h, Qc, 3h] value=7,         B=[Td, 3d, 2s] value=5         -> player wins, bead_word=0x0731
        let rounds = [
            BaccRound::new(
                hand(&[CardInt::Card9d, CardInt::CardQh]),
                hand(&[CardInt::Card9c, CardInt::CardTs]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::Card3c, CardInt::CardKd, CardInt::Card8c]),
                hand(&[CardInt::Card6s, CardInt::CardJh]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::Card5d, CardInt::Card7c]),
                hand(&[CardInt::Card9h, CardInt::CardTc]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::CardQs, CardInt::Card9d]),
                hand(&[CardInt::Card4s, CardInt::Card5s]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::CardAc, CardInt::Card6s]),
                hand(&[CardInt::Card7h, CardInt::CardKc]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::CardAh, CardInt::Card7s]),
                hand(&[CardInt::CardAd, CardInt::Card6c]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::Card6h, CardInt::CardQd]),
                hand(&[CardInt::Card2c, CardInt::CardKh, CardInt::CardTs]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::CardKs, CardInt::Card9c]),
                hand(&[CardInt::Card8h, CardInt::Card7d]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::Card9s, CardInt::Card2d]),
                hand(&[CardInt::Card8s, CardInt::CardTc]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::Card9h, CardInt::CardJd]),
                hand(&[CardInt::Card4d, CardInt::Card6d]),
                false,
                None,
            ),
            BaccRound::new(
                hand(&[CardInt::Card3s, CardInt::CardTh, CardInt::CardJs]),
                hand(&[CardInt::Card9s, CardInt::Card8d]),
                false,
                Some(3),
            ),
            BaccRound::new(
                hand(&[CardInt::Card4h, CardInt::CardQc, CardInt::Card3h]),
                hand(&[CardInt::CardTd, CardInt::Card3d, CardInt::Card2s]),
                false,
                None,
            ),
        ];
        let mut sb = BaccScoreboard::new();
        sb.update(&rounds[0]);
        sb.update(&rounds[1]);
        assert_eq!(sb.bead_plate().encode().as_str(), "09030612");
        assert_eq!(sb.big_road().encode().as_str(), "161201");
        for round in &rounds[2..] {
            sb.update(round);
        }
        assert_eq!(
            sb.bead_plate().encode().as_str(),
            "090306120902090307030801062109010802090107120731"
        );
        assert_eq!(
            sb.big_road().encode().as_str(),
            "161229020208010621090103080201090101071201073101"
        );
        let dr = sb.derived_roads().encode();
        assert_eq!(dr[0].as_str(), "030605");
        assert_eq!(dr[1].as_str(), "0403");
        assert_eq!(dr[2].as_str(), "04");
        sb.clear();
        assert_eq!(sb.bead_plate().encode().as_str(), "");
        assert_eq!(sb.big_road().encode().as_str(), "");
        let dr = sb.derived_roads().encode();
        assert_eq!(dr[0].as_str(), "");
        assert_eq!(dr[1].as_str(), "");
        assert_eq!(dr[2].as_str(), "");
    }

    #[test]
    fn first_round_non_tie_starts_big_road_column() {
        // player: Eight(8) + Ace(1) = 9 natural; banker: Deuce(2) + Trey(3) = 5 -> player wins
        let mut sb = BaccScoreboard::new();
        sb.update(&BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        ));
        assert_eq!(sb.bead_plate().encode().as_str(), "0901");
        assert_eq!(sb.big_road().encode().as_str(), "090101");
    }

    #[test]
    fn big_road_tie_count_saturates_at_15() {
        // Round 1: player wins -> opens column, big_road[0] = 0x09
        // Rounds 2-16: 15 ties -> big_road[0] increments to 0xF9 (tie count = 15)
        // Round 17: 16th tie -> saturation: big_road[0] stays at 0xF9
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let tie = BaccRound::new(
            hand(&[CardInt::Card4c, CardInt::Card5h]),
            hand(&[CardInt::Card4d, CardInt::Card5s]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        sb.update(&player_win);
        for _ in 0..16 {
            sb.update(&tie);
        }
        assert_eq!(sb.big_road().encode().as_str(), "f90101");
    }
}
