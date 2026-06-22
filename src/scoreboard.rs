//! Baccarat scoreboard tracking the five standard road displays.

use arrayvec::{ArrayString, ArrayVec};

use crate::{BaccOutcome, BaccRound};

const MAX_ROUNDS: usize = 96;
const BEAD_PLATE_CAP: usize = MAX_ROUNDS * 2;
const BIG_ROAD_CAP: usize = MAX_ROUNDS * 3;
const DERIVED_ROAD_CAP: usize = MAX_ROUNDS;

/// Conventional road height in rows (industry standard for all five baccarat roads).
pub const ROWS: usize = 6;
/// Maximum number of columns shown in a rendered road grid.
pub const MAX_COL_COUNT: usize = 40;
/// Maximum number of columns shown in a rendered bead plate grid (`MAX_ROUNDS / ROWS`).
pub(crate) const MAX_BEAD_PLATE_COL_COUNT: usize = MAX_ROUNDS / ROWS;
/// Maximum entries a single logical column can contribute to a [`MAX_COL_COUNT`]-wide grid.
///
/// A column fills [`ROWS`] cells going down, then its dragon tail runs right across the
/// remaining columns: `MAX_COL_COUNT - 1` more. Total = `MAX_COL_COUNT + ROWS - 1`.
pub(crate) const MAX_COL_ENTRIES: usize = MAX_COL_COUNT + ROWS - 1;

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
        self.update_bead(Self::bead_word(&round.outcome()));
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

    /// Encodes the scoreboard as a lowercase hex string of bead words.
    ///
    /// Sufficient to fully reconstruct the scoreboard via [`decode`], including
    /// the big road, derived roads, and column heights.
    ///
    /// [`decode`]: BaccScoreboard::decode
    #[must_use]
    pub fn encode(&self) -> ArrayString<{ BEAD_PLATE_CAP * 2 }> {
        crate::bytes_to_hex::<BEAD_PLATE_CAP, { BEAD_PLATE_CAP * 2 }>(&self.bead_plate)
    }

    /// Decodes a [`BaccScoreboard`] from a hex string produced by [`encode`],
    /// replaying each bead word to rebuild all five scoreboards from scratch.
    ///
    /// [`encode`]: BaccScoreboard::encode
    #[must_use]
    pub fn decode(hex: &str) -> Self {
        let bytes: ArrayVec<u8, BEAD_PLATE_CAP> = crate::hex_to_bytes(hex);
        let mut sb = Self::new();
        for word in bytes.chunks_exact(2) {
            sb.update_bead(u16::from_be_bytes([word[0], word[1]]));
        }
        sb
    }

    /// Applies an incremental diff from a server-supplied bead plate hex string.
    ///
    /// If `hex` starts with the current [`encode`] output the new suffix is applied
    /// bead word by bead word. Otherwise the scoreboard is fully reconstructed via
    /// [`decode`] -- covering gap, new shoe, and server-reset cases.
    ///
    /// [`encode`]: BaccScoreboard::encode
    /// [`decode`]: BaccScoreboard::decode
    pub fn apply_hex_diff(&mut self, hex: &str) {
        let client_hex = self.encode();
        if hex.starts_with(client_hex.as_str()) {
            let suffix = &hex[client_hex.len()..];
            let bytes: ArrayVec<u8, BEAD_PLATE_CAP> = crate::hex_to_bytes(suffix);
            for word in bytes.chunks_exact(2) {
                self.update_bead(u16::from_be_bytes([word[0], word[1]]));
            }
        } else {
            *self = Self::decode(hex);
        }
    }

    /// Returns the outcome bits of the most recent non-empty big road column.
    ///
    /// `1` = player, `2` = banker. Returns `0` when the big road is empty or the
    /// only column is a shoe-start tie placeholder (row count 0).
    #[must_use]
    pub fn last_big_road_marker(&self) -> u8 {
        if self.big_road.is_empty() {
            return 0;
        }
        self.big_road[self.big_road.len() - 2] & 0x03
    }

    /// Returns the row counts of the five most recent big road columns.
    ///
    /// Index 0 is the current column. Indices 1-3 are the Big Eye Boy, Small Road,
    /// and Cockroach Pig reference columns. A height of `0` means the column does
    /// not exist.
    #[must_use]
    pub fn col_heights(&self) -> &[u8] {
        &self.col_heights
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

    fn update_bead(&mut self, bead: u16) {
        let is_tie = (bead & 0x3) == 0x3;
        self.update_bead_plate(bead);
        self.update_big_road(bead, is_tie);
        if !is_tie {
            self.update_derived_roads();
        }
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

    /// Returns the bead plate as a `ROWS`-high grid of at most `cols` columns (capped at
    /// `MAX_ROUNDS / ROWS`), newest entries filling the rightmost columns.
    ///
    /// Each cell is `(bead_byte, aux_byte)` where `bead_byte` = outcome/pair/third card flags
    /// (bits 7-0 of the bead word) and `aux_byte` = winner's hand value (bits 15-8).
    /// `(0, 0)` means empty. Oldest entries at index 0.
    /// Pass the result to the renderer -- no further domain calls needed.
    #[must_use]
    pub fn simulate_bead_plate(
        &self,
        cols: usize,
    ) -> ArrayVec<[(u8, u8); ROWS], MAX_BEAD_PLATE_COL_COUNT> {
        let entries = decode_bead_plate(&self.bead_plate, cols);
        let mut grid: ArrayVec<[(u8, u8); ROWS], MAX_BEAD_PLATE_COL_COUNT> = ArrayVec::new();
        for (i, &(bead, aux)) in entries.iter().enumerate() {
            let col = i / ROWS;
            let row = i % ROWS;
            while grid.len() <= col {
                grid.push([(0u8, 0u8); ROWS]);
            }
            grid[col][row] = (bead, aux);
        }
        grid
    }

    /// Returns the big road as a `ROWS`-high grid of at most [`MAX_COL_COUNT`] columns.
    ///
    /// Each column is `[(bead, aux); ROWS]`. `(0, 0)` means empty. Oldest column at index 0.
    /// Pass the result to the renderer — no further domain calls needed.
    #[must_use]
    pub fn simulate_big_road(&self) -> ArrayVec<[(u8, u8); ROWS], MAX_COL_COUNT> {
        simulate(&decode_big_road_cols(&self.big_road), |b| b & 0x03)
    }

    /// Returns derived road `idx` as a `ROWS`-high grid of at most [`MAX_COL_COUNT`] columns.
    ///
    /// `idx`: 0 = Big Eye Boy, 1 = Small Road, 2 = Cockroach Pig.
    /// Each column is `[(icon, 0); ROWS]` where `icon` is `2` (red), `1` (blue), or `0` (empty).
    /// Pass the result to the renderer — no further domain calls needed.
    #[must_use]
    pub fn simulate_derived_road(&self, idx: usize) -> ArrayVec<[(u8, u8); ROWS], MAX_COL_COUNT> {
        simulate(&decode_derived_runs(&self.derived_roads[idx]), |b| b)
    }
}

/// Windows bead plate bytes to the last `cols * ROWS` entries (capped at
/// `MAX_BEAD_PLATE_COL_COUNT * ROWS`), returning them as `(bead_byte, aux_byte)` pairs in
/// chronological order (oldest first).
///
/// `bead_byte` = outcome/pair/third card flags (bits 7-0 of the bead word).
/// `aux_byte` = winner's hand value (bits 15-8 of the bead word).
fn decode_bead_plate(
    bytes: &[u8],
    cols: usize,
) -> ArrayVec<(u8, u8), { MAX_BEAD_PLATE_COL_COUNT * ROWS }> {
    let capacity = cols.min(MAX_BEAD_PLATE_COL_COUNT) * ROWS;
    let start = bytes.len().saturating_sub(capacity * 2);
    let mut entries: ArrayVec<(u8, u8), { MAX_BEAD_PLATE_COL_COUNT * ROWS }> = ArrayVec::new();
    for chunk in bytes[start..].chunks_exact(2) {
        entries.push((chunk[1], chunk[0]));
    }
    entries
}

/// Parses big road bytes (oldest column first) into at most [`MAX_COL_COUNT`] columns.
///
/// Reads the internal byte buffer right-to-left. Columns beyond `MAX_COL_ENTRIES`
/// rows are silently truncated to the oldest `MAX_COL_ENTRIES` rows, as the excess
/// tail falls outside the display window.
fn decode_big_road_cols(
    bytes: &[u8],
) -> ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> {
    let mut columns: ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> = ArrayVec::new();
    let mut pos = bytes.len();
    while pos > 0 && columns.len() < MAX_COL_COUNT {
        pos -= 1;
        let row_count = bytes[pos] as usize;
        if row_count == 0 {
            break;
        }
        // Skip the newest rows that overflow the display window (2 bytes each).
        let skip_count = row_count.saturating_sub(MAX_COL_ENTRIES);
        pos = pos.saturating_sub(skip_count * 2);
        let take = row_count.min(MAX_COL_ENTRIES);
        let mut rows: ArrayVec<(u8, u8), MAX_COL_ENTRIES> = ArrayVec::new();
        for _ in 0..take {
            pos -= 1;
            let bead = bytes[pos];
            pos -= 1;
            let aux_byte = bytes[pos];
            rows.push((bead, aux_byte));
        }
        rows.reverse();
        columns.push(rows);
    }
    columns.reverse();
    columns
}

/// Expands derived road RLE bytes into at most [`MAX_COL_COUNT`] runs.
///
/// Each source byte encodes one run: bits 7-1 = run length, bit 0 = icon (1 = red, 0 = blue).
/// Runs are expanded to `(icon, 0)` pairs where `icon` is `2` (red) or `1` (blue).
fn decode_derived_runs(
    bytes: &[u8],
) -> ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> {
    let skip = bytes.len().saturating_sub(MAX_COL_COUNT);
    let mut columns: ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> = ArrayVec::new();
    for &byte in &bytes[skip..] {
        let icon: u8 = (byte & 1) + 1;
        let run_len = (byte >> 1) as usize;
        let take = run_len.min(MAX_COL_ENTRIES);
        let mut run: ArrayVec<(u8, u8), MAX_COL_ENTRIES> = ArrayVec::new();
        for _ in 0..take {
            run.push((icon, 0u8));
        }
        columns.push(run);
    }
    columns
}

/// Simulates the shared grid-fill algorithm used by the big road and all three derived roads.
///
/// Implements the standard baccarat dragon-tail convention: the cursor goes down until
/// blocked (bottom of grid or occupied cell), then turns right. Two supplementary rules
/// apply during right-turns:
///
/// - **Space rule**: while moving right, if the cell directly below the cursor is empty,
///   resume going down.
/// - **Color rule**: suppress the Space-rule drop when the cell diagonally below-left
///   is the same color as the current entry, preventing same-color overlap.
///
/// `marker_of` extracts the color key from a bead byte. For the big road pass
/// `|b| b & 0x03`; for derived roads pass `|b| b`.
///
/// Returns at most [`MAX_COL_COUNT`] visual columns. Entries that would land beyond
/// column `MAX_COL_COUNT - 1` are silently dropped.
fn simulate<F>(
    columns: &ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT>,
    marker_of: F,
) -> ArrayVec<[(u8, u8); ROWS], MAX_COL_COUNT>
where
    F: Fn(u8) -> u8,
{
    let mut grid: ArrayVec<[(u8, u8); ROWS], MAX_COL_COUNT> = ArrayVec::new();
    let mut next_col = 0usize;

    'outer: for column_rows in columns {
        let start = next_col;
        if start >= MAX_COL_COUNT {
            break;
        }
        while grid.len() <= start {
            grid.push([(0u8, 0u8); ROWS]);
        }
        next_col = start + 1;

        let mut col = start;
        let mut row = 0usize;
        let mut going_down = true;

        for &(bead_byte, aux_byte) in column_rows {
            if col >= MAX_COL_COUNT {
                continue 'outer;
            }
            while col >= grid.len() {
                grid.push([(0u8, 0u8); ROWS]);
            }

            let has_row_below = row + 1 < ROWS;
            let has_col_to_left = col > 0;
            let color_conflict = has_row_below
                && has_col_to_left
                && marker_of(grid[col - 1][row + 1].0) == marker_of(bead_byte);
            let is_cell_below_vacant = has_row_below && grid[col][row + 1].0 == 0;
            let space_below = is_cell_below_vacant && !color_conflict;

            if !going_down && space_below {
                going_down = true;
            }

            grid[col][row] = (bead_byte, aux_byte);

            if row == 0 {
                next_col = next_col.max(col + 1);
            }

            if going_down && space_below {
                row += 1;
            } else {
                going_down = false;
                col += 1;
            }
        }
    }

    grid
}

#[cfg(test)]
mod tests {
    use super::{
        BIG_ROAD_CAP, BaccScoreboard, DERIVED_ROAD_CAP, MAX_BEAD_PLATE_COL_COUNT, MAX_COL_COUNT,
        MAX_COL_ENTRIES, ROWS, decode_bead_plate, decode_big_road_cols, decode_derived_runs,
        simulate,
    };
    use crate::BaccRound;
    use crate::tests::hand;
    use arrayvec::ArrayVec;
    use kev::CardInt;

    const R: (u8, u8) = (2, 0);
    const B: (u8, u8) = (1, 0);
    const E: (u8, u8) = (0, 0);

    fn col(entries: &[(u8, u8)]) -> ArrayVec<(u8, u8), MAX_COL_ENTRIES> {
        entries.iter().copied().collect()
    }

    fn cols(data: &[&[(u8, u8)]]) -> ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> {
        data.iter().map(|&c| col(c)).collect()
    }

    fn rep(entry: (u8, u8), n: usize) -> ArrayVec<(u8, u8), MAX_COL_ENTRIES> {
        (0..n).map(|_| entry).collect()
    }

    fn one_col(
        entry: (u8, u8),
        n: usize,
    ) -> ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> {
        let mut outer = ArrayVec::new();
        outer.push(rep(entry, n));
        outer
    }

    #[test]
    fn decode_bead_plate_empty() {
        let result: arrayvec::ArrayVec<(u8, u8), { MAX_BEAD_PLATE_COL_COUNT * ROWS }> =
            decode_bead_plate(&[], 16);
        assert!(result.is_empty());
    }

    #[test]
    fn decode_bead_plate_fewer_entries_than_window_returns_all() {
        // 3 bead words stored as [hand_val, outcome_flags]: bead_byte=outcome, aux_byte=hand_val
        let bytes = [0x09u8, 0x01, 0x06, 0x02, 0x07, 0x03];
        let result = decode_bead_plate(&bytes, 16);
        assert_eq!(
            result.as_slice(),
            &[(0x01, 0x09), (0x02, 0x06), (0x03, 0x07)]
        );
    }

    #[test]
    fn decode_bead_plate_cols_param_limits_to_newest_entries() {
        // 7 bead words; cols=1 -> capacity=1*ROWS=6 entries -> oldest 1 word excluded
        let bytes = [
            0x01u8, 0x01, 0x02, 0x02, 0x03, 0x03, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07,
        ];
        let result = decode_bead_plate(&bytes, 1);
        assert_eq!(
            result.as_slice(),
            &[
                (0x02, 0x02),
                (0x03, 0x03),
                (0x04, 0x04),
                (0x05, 0x05),
                (0x06, 0x06),
                (0x07, 0x07)
            ]
        );
    }

    #[test]
    fn simulate_bead_plate_packs_column_major() {
        // 7 bead words; cols=16 -> all visible; col0 gets rows 0-5, col1 gets row 0 only
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for _ in 0..7 {
            sb.update(&player_win);
        }
        let grid = sb.simulate_bead_plate(16);
        assert_eq!(grid.len(), 2);
        for row in 0..ROWS {
            assert_ne!(grid[0][row].0, 0);
        }
        assert_ne!(grid[1][0].0, 0);
        for row in 1..ROWS {
            assert_eq!(grid[1][row].0, 0);
        }
    }

    #[test]
    fn simulate_bead_plate_cols_param_excludes_oldest_entries() {
        // 96 entries (full shoe); cols=10 -> only newest 60 entries visible (10 columns x 6 rows)
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for _ in 0..96 {
            sb.update(&player_win);
        }
        let grid = sb.simulate_bead_plate(10);
        assert_eq!(grid.len(), 10);
        for col in &grid {
            for row in 0..ROWS {
                assert_ne!(col[row].0, 0);
            }
        }
    }

    #[test]
    fn simulate_column_selection_uses_successive_columns() {
        let grid = simulate(&cols(&[&[R], &[B], &[R]]), |b| b);
        assert_eq!(grid.len(), 3);
        assert_eq!(grid[0][0], R);
        assert_eq!(grid[1][0], B);
        assert_eq!(grid[2][0], R);
    }

    #[test]
    fn simulate_cursor_turns_right_at_bottom() {
        let grid = simulate(&one_col(R, 7), |b| b);
        assert!(grid.len() >= 2);
        assert_eq!(grid[0], [R, R, R, R, R, R]);
        assert_eq!(grid[1], [E, E, E, E, E, R]);
    }

    #[test]
    fn simulate_space_rule_resumes_going_down_when_space_below() {
        let mut columns: ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> =
            ArrayVec::new();
        columns.push(rep(B, 7));
        columns.push(rep(R, 8));
        let grid = simulate(&columns, |b| b);
        assert_eq!(grid[0], [B, B, B, B, B, B]);
        assert_eq!(grid[1], [R, R, R, R, R, B]);
        assert_eq!(grid[2][4], R);
        assert_eq!(grid[2][5], R);
        assert_eq!(grid[3][5], R);
    }

    #[test]
    fn simulate_color_rule_suppresses_drop_at_same_color_diagonal() {
        let mut columns: ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> =
            ArrayVec::new();
        columns.push(rep(R, 8));
        columns.push(rep(B, 5));
        columns.push(rep(R, 7));
        let grid = simulate(&columns, |b| b);
        assert_eq!(grid[0], [R, R, R, R, R, R]);
        assert_eq!(grid[1], [B, B, B, B, B, R]);
        assert_eq!(grid[2], [R, R, R, R, R, R]);
        assert_eq!(grid[3][4], R);
        assert_eq!(grid[4][4], R);
    }

    #[test]
    fn simulate_double_dragon_two_tails_of_different_color_land_side_by_side() {
        let mut columns: ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> =
            ArrayVec::new();
        columns.push(rep(R, 9));
        columns.push(rep(B, 8));
        let grid = simulate(&columns, |b| b);
        assert_eq!(grid[0], [R, R, R, R, R, R]);
        assert_eq!(grid[1], [B, B, B, B, B, R]);
        assert_eq!(grid[2][4], B);
        assert_eq!(grid[2][5], R);
        assert_eq!(grid[3][4], B);
        assert_eq!(grid[3][5], R);
        assert_eq!(grid[4][4], B);
    }

    #[test]
    fn simulate_quintuple_dragon_extremely_rare_one() {
        let mut columns: ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> =
            ArrayVec::new();
        columns.push(rep(B, 12));
        columns.push(rep(R, 8));
        columns.push(rep(B, 7));
        columns.push(rep(R, 6));
        columns.push(rep(B, 3));
        columns.push(rep(R, 1));
        columns.push(rep(B, 3));
        let grid = simulate(&columns, |b| b);
        assert_eq!(grid[0], [B, B, B, B, B, B]);
        assert_eq!(grid[1], [R, R, R, R, R, B]);
        assert_eq!(grid[2], [B, B, B, B, R, B]);
        assert_eq!(grid[3], [R, R, R, B, R, B]);
        assert_eq!(grid[4], [B, B, R, B, R, B]);
        assert_eq!(grid[5], [R, B, R, B, E, B]);
        assert_eq!(grid[6][0], B);
        assert_eq!(grid[6][2], R);
        assert_eq!(grid[7][0], B);
        assert_eq!(grid[7][1], B);
    }

    #[test]
    fn simulate_sextuple_dragon_sixth_tail_immediately_turn_right() {
        let mut columns: ArrayVec<ArrayVec<(u8, u8), MAX_COL_ENTRIES>, MAX_COL_COUNT> =
            ArrayVec::new();
        columns.push(rep(R, 12));
        columns.push(rep(B, 9));
        columns.push(rep(R, 8));
        columns.push(rep(B, 7));
        columns.push(rep(R, 6));
        columns.push(rep(B, 4));
        columns.push(rep(R, 3));
        let grid = simulate(&columns, |b| b);
        assert_eq!(grid[0], [R, R, R, R, R, R]);
        assert_eq!(grid[1], [B, B, B, B, B, R]);
        assert_eq!(grid[2], [R, R, R, R, B, R]);
        assert_eq!(grid[3], [B, B, B, R, B, R]);
        assert_eq!(grid[4], [R, R, B, R, B, R]);
        assert_eq!(grid[5], [B, R, B, R, B, R]);
        assert_eq!(grid[6], [B, R, B, R, E, R]);
        assert_eq!(grid[7], [B, R, B, E, E, E]);
        assert_eq!(grid[8][0], B);
        assert_eq!(grid[8][1], R);
        assert_eq!(grid[9][0], R);
        assert_eq!(grid[10][0], R);
        assert_eq!(grid[10][1], R);
    }

    #[test]
    fn decode_big_road_cols_empty() {
        assert!(decode_big_road_cols(&[]).is_empty());
    }

    #[test]
    fn decode_big_road_cols_single_column_one_row() {
        let result = decode_big_road_cols(&[0x11, 0x02, 0x01]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].as_slice(), &[(0x02u8, 0x11u8)]);
    }

    #[test]
    fn decode_big_road_cols_single_column_two_rows() {
        let result = decode_big_road_cols(&[0x11, 0x01, 0x12, 0x02, 0x02]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].as_slice(), &[(0x01u8, 0x11u8), (0x02u8, 0x12u8)]);
    }

    #[test]
    fn decode_big_road_cols_two_columns_one_row_each() {
        let result = decode_big_road_cols(&[0x11, 0x01, 0x01, 0x22, 0x02, 0x01]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].as_slice(), &[(0x01u8, 0x11u8)]);
        assert_eq!(result[1].as_slice(), &[(0x02u8, 0x22u8)]);
    }

    #[test]
    fn decode_derived_runs_empty() {
        assert!(decode_derived_runs(&[]).is_empty());
    }

    #[test]
    fn decode_derived_runs_single_blue_run_of_one() {
        let result = decode_derived_runs(&[0x02]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].as_slice(), &[(1u8, 0u8)]);
    }

    #[test]
    fn decode_derived_runs_single_red_run_of_one() {
        let result = decode_derived_runs(&[0x03]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].as_slice(), &[(2u8, 0u8)]);
    }

    #[test]
    fn decode_derived_runs_single_run_length_three() {
        let result = decode_derived_runs(&[0x06]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].as_slice(), &[(1u8, 0u8), (1u8, 0u8), (1u8, 0u8)]);
    }

    #[test]
    fn decode_derived_runs_two_separate_runs() {
        let result = decode_derived_runs(&[0x02, 0x03]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].as_slice(), &[(1u8, 0u8)]);
        assert_eq!(result[1].as_slice(), &[(2u8, 0u8)]);
    }

    #[test]
    fn simulate_big_road_41_alternating_drops_oldest_column() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let banker_win = BaccRound::new(
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for i in 0..41 {
            if i % 2 == 0 {
                sb.update(&player_win);
            } else {
                sb.update(&banker_win);
            }
        }
        let grid = sb.simulate_big_road();
        assert_eq!(grid.len(), MAX_COL_COUNT);
        for col in &grid {
            assert_ne!(col[0].0, 0);
            for row in 1..ROWS {
                assert_eq!(col[row].0, 0);
            }
        }
        assert_eq!(grid[39][0].0 & 0x03, 0x01);
        assert_eq!(grid[38][0].0 & 0x03, 0x02);
    }

    #[test]
    fn simulate_big_road_46_player_wins_caps_streak_and_banker_visible_at_col1_row0() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let banker_win = BaccRound::new(
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for _ in 0..46 {
            sb.update(&player_win);
        }
        sb.update(&banker_win);
        let grid = sb.simulate_big_road();
        assert_eq!(grid.len(), MAX_COL_COUNT);
        // column 0: all ROWS filled with player entries
        for row in 0..ROWS {
            assert_eq!(grid[0][row].0 & 0x03, 0x01);
        }
        // column 1 row 0: banker win (new streak at next_col=1)
        assert_eq!(grid[1][0].0 & 0x03, 0x02);
        // columns 1-39 row 5: player tail entries
        for col in 1..MAX_COL_COUNT {
            assert_eq!(grid[col][5].0 & 0x03, 0x01);
        }
        // columns 2-39 rows 0-4: empty
        for col in 2..MAX_COL_COUNT {
            for row in 0..ROWS - 1 {
                assert_eq!(grid[col][row].0, 0);
            }
        }
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
        assert_eq!(sb.encode().as_str(), "09030612");
        assert_eq!(
            crate::bytes_to_hex::<BIG_ROAD_CAP, { BIG_ROAD_CAP * 2 }>(&sb.big_road).as_str(),
            "161201"
        );
        for round in &rounds[2..] {
            sb.update(round);
        }
        assert_eq!(
            sb.encode().as_str(),
            "090306120902090307030801062109010802090107120731"
        );
        assert_eq!(
            crate::bytes_to_hex::<BIG_ROAD_CAP, { BIG_ROAD_CAP * 2 }>(&sb.big_road).as_str(),
            "161229020208010621090103080201090101071201073101"
        );
        assert_eq!(
            crate::bytes_to_hex::<DERIVED_ROAD_CAP, { DERIVED_ROAD_CAP * 2 }>(&sb.derived_roads[0])
                .as_str(),
            "030605"
        );
        assert_eq!(
            crate::bytes_to_hex::<DERIVED_ROAD_CAP, { DERIVED_ROAD_CAP * 2 }>(&sb.derived_roads[1])
                .as_str(),
            "0403"
        );
        assert_eq!(
            crate::bytes_to_hex::<DERIVED_ROAD_CAP, { DERIVED_ROAD_CAP * 2 }>(&sb.derived_roads[2])
                .as_str(),
            "04"
        );
        sb.clear();
        assert_eq!(sb.encode().as_str(), "");
        assert!(sb.big_road.is_empty());
        assert!(sb.derived_roads[0].is_empty());
        assert!(sb.derived_roads[1].is_empty());
        assert!(sb.derived_roads[2].is_empty());
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
        assert_eq!(sb.encode().as_str(), "0901");
        assert_eq!(
            crate::bytes_to_hex::<BIG_ROAD_CAP, { BIG_ROAD_CAP * 2 }>(&sb.big_road).as_str(),
            "090101"
        );
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
        assert_eq!(
            crate::bytes_to_hex::<BIG_ROAD_CAP, { BIG_ROAD_CAP * 2 }>(&sb.big_road).as_str(),
            "f90101"
        );
    }

    #[test]
    fn apply_hex_diff_empty_self_applies_all_words() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut reference = BaccScoreboard::new();
        for _ in 0..4 {
            reference.update(&player_win);
        }
        let hex = reference.encode();
        let mut sb = BaccScoreboard::new();
        sb.apply_hex_diff(hex.as_str());
        assert_eq!(sb.encode(), hex);
        assert_eq!(sb.big_road, reference.big_road);
    }

    #[test]
    fn apply_hex_diff_extends_existing() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for _ in 0..2 {
            sb.update(&player_win);
        }
        let mut full = BaccScoreboard::new();
        for _ in 0..4 {
            full.update(&player_win);
        }
        let full_hex = full.encode();
        sb.apply_hex_diff(full_hex.as_str());
        assert_eq!(sb.encode(), full_hex);
        assert_eq!(sb.big_road, full.big_road);
    }

    #[test]
    fn apply_hex_diff_full_match_is_noop() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for _ in 0..3 {
            sb.update(&player_win);
        }
        let hex_before = sb.encode();
        sb.apply_hex_diff(hex_before.as_str());
        assert_eq!(sb.encode(), hex_before);
    }

    #[test]
    fn apply_hex_diff_gap_reconstructs() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let banker_win = BaccRound::new(
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for _ in 0..3 {
            sb.update(&player_win);
        }
        let mut server = BaccScoreboard::new();
        for _ in 0..3 {
            server.update(&banker_win);
        }
        let server_hex = server.encode();
        sb.apply_hex_diff(server_hex.as_str());
        assert_eq!(sb.encode(), server_hex);
        assert_eq!(sb.big_road, server.big_road);
    }

    #[test]
    fn apply_hex_diff_server_shorter_reconstructs() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for _ in 0..4 {
            sb.update(&player_win);
        }
        let mut short = BaccScoreboard::new();
        for _ in 0..2 {
            short.update(&player_win);
        }
        let short_hex = short.encode();
        sb.apply_hex_diff(short_hex.as_str());
        assert_eq!(sb.encode(), short_hex);
        assert_eq!(sb.big_road, short.big_road);
    }

    #[test]
    fn apply_hex_diff_empty_hex_resets_to_empty() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        for _ in 0..3 {
            sb.update(&player_win);
        }
        sb.apply_hex_diff("");
        assert_eq!(sb.encode().as_str(), "");
        assert!(sb.big_road.is_empty());
    }

    #[test]
    fn last_big_road_marker_empty_returns_zero() {
        let sb = BaccScoreboard::new();
        assert_eq!(sb.last_big_road_marker(), 0);
    }

    #[test]
    fn last_big_road_marker_player_column() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        sb.update(&player_win);
        assert_eq!(sb.last_big_road_marker(), 0x01);
    }

    #[test]
    fn last_big_road_marker_banker_column() {
        let banker_win = BaccRound::new(
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        sb.update(&banker_win);
        assert_eq!(sb.last_big_road_marker(), 0x02);
    }

    #[test]
    fn last_big_road_marker_tie_only_shoe_start_returns_tie_marker_and_height_is_zero() {
        let tie = BaccRound::new(
            hand(&[CardInt::Card4c, CardInt::Card5h]),
            hand(&[CardInt::Card4d, CardInt::Card5s]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        sb.update(&tie);
        // Shoe-start tie creates a placeholder column with row count 0.
        // Marker bits are 0x03 (tie), not a player/banker marker.
        // Callers must guard via col_heights()[0] > 0 before using the marker.
        assert_eq!(sb.last_big_road_marker(), 0x03);
        assert_eq!(sb.col_heights()[0], 0);
    }

    #[test]
    fn last_big_road_marker_reflects_current_column_after_switch() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let banker_win = BaccRound::new(
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        sb.update(&player_win);
        sb.update(&player_win);
        sb.update(&banker_win);
        assert_eq!(sb.last_big_road_marker(), 0x02);
    }

    #[test]
    fn col_heights_empty_all_zero() {
        let sb = BaccScoreboard::new();
        assert_eq!(sb.col_heights(), &[0u8; 5]);
    }

    #[test]
    fn col_heights_after_one_player_win() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        sb.update(&player_win);
        assert_eq!(sb.col_heights()[0], 1);
        for &h in &sb.col_heights()[1..] {
            assert_eq!(h, 0);
        }
    }

    #[test]
    fn col_heights_exposes_five_entries() {
        let sb = BaccScoreboard::new();
        assert_eq!(sb.col_heights().len(), 5);
    }

    #[test]
    fn col_heights_tracks_column_transitions() {
        let player_win = BaccRound::new(
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            false,
            None,
        );
        let banker_win = BaccRound::new(
            hand(&[CardInt::Card2c, CardInt::Card3h]),
            hand(&[CardInt::Card8c, CardInt::CardAh]),
            false,
            None,
        );
        let mut sb = BaccScoreboard::new();
        sb.update(&player_win);
        sb.update(&player_win);
        sb.update(&banker_win);
        // Current column (banker) has 1 row; previous player column had 2 rows.
        assert_eq!(sb.col_heights()[0], 1);
        assert_eq!(sb.col_heights()[1], 2);
    }

    #[test]
    fn scoreboard_roundtrip_through_encode_decode() {
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
        let mut original = BaccScoreboard::new();
        for round in &rounds {
            original.update(round);
        }
        let reconstructed = BaccScoreboard::decode(original.encode().as_str());
        assert_eq!(original.encode(), reconstructed.encode());
        assert_eq!(original.big_road, reconstructed.big_road);
        assert_eq!(original.derived_roads, reconstructed.derived_roads);
        assert_eq!(original.col_heights, reconstructed.col_heights);
    }
}
