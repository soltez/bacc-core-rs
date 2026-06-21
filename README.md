# bacc-core-rs

Primitive `no_std` types for the bacc ecosystem.

## Overview

`bacc-core-rs` provides the core baccarat data types used across the bacc
ecosystem. It is `no_std` compatible, making it suitable for embedded targets
and Wasm consumers that do not want to pull in an allocator or standard library.

## Types

### Round (`src/round.rs`)

- **`BaccHand`** - A single baccarat hand holding up to three cards. Backed by
  `ArrayVec<CardInt, 3>`. Computes point value and pair/third-card flags.
- **`BaccOutcome`** - The derived outcome of a round: winner, pair flags,
  third-card flags, and final hand values. Encodes to/from a compact `u32`.
- **`BaccRound`** - A fully resolved baccarat round holding both hands plus
  metadata (forced third flag, cut card index). Encodes to/from a `u64`.
  Produces a human-readable TOML fragment via `describe()`.

### Scoreboard (`src/scoreboard.rs`)

- **`BaccScoreboard`** - Tracks all five standard baccarat road displays for a
  running shoe. Call `update(&BaccRound)` after each round. Encodes to/from a
  lowercase hex string (bead plate bytes). All five road grids are derived
  on demand via the simulate methods below -- they are not persisted.

## Key Methods

### `BaccRound`

| Method | Description |
|---|---|
| `encode() -> ArrayString<16>` | Pack round into a `u64` hex string |
| `decode(hex) -> BaccRound` | Reconstruct from hex |
| `outcome() -> BaccOutcome` | Derive winner, pairs, thirds, hand values |
| `describe() -> ArrayString<384>` | Human-readable TOML fragment |

### `BaccScoreboard`

| Method | Description |
|---|---|
| `update(&BaccRound)` | Append a resolved round |
| `encode() -> ArrayString<N>` | Serialize bead plate bytes to hex |
| `decode(hex)` | Reconstruct all five roads from hex |
| `simulate_bead_plate(cols)` | Bead plate grid (column-major, newest `cols` columns) |
| `simulate_big_road()` | Big road grid (dragon-tail layout) |
| `simulate_derived_road(idx)` | One of three derived roads (0=Big Eye Boy, 1=Small Road, 2=Cockroach Pig) |

### Simulate cell format

Each simulate method returns an `ArrayVec` of columns, each column being `[(u8, u8); ROWS]`:

| Road | `bead_byte` | `aux_byte` |
|---|---|---|
| Bead plate | outcome/pair/third flags (bits 7-0) | hand value (bits 15-8) |
| Big road | outcome/pair/third flags | `ttttvvvv` (tie count + hand value) |
| Derived roads | icon: `2` = red, `1` = blue | `0` |

`(0, 0)` is an empty cell in all three.

### `BaccRound::describe()` output

```toml
[round]
outcome = "player"        # "player" | "banker" | "tie"
player.pair = false
player.third_card = false
player.hand_value = 9
banker.pair = false
banker.third_card = false
banker.hand_value = 7
banker.forced_third = false
player.cards = ["Ac", "Jh"]
banker.cards = ["As", "7c"]
cut_card.ordinal = 3      # 1-indexed; omitted when no cut card was encountered
```

Cards use 2-char format: rank (`A K Q J T 9-2`) + suit (`s h d c`).

## Encoding

All encode/decode pairs are lossless roundtrips:

| Type | Encoded form |
|---|---|
| `BaccOutcome` | 4-byte `u32` as 8 hex chars |
| `BaccRound` | 8-byte `u64` as 16 hex chars |
| `BaccScoreboard` | bead plate bytes as hex string |

## Capacity

All buffers are sized for a 96-round shoe via `MAX_ROUNDS` in
`src/scoreboard.rs`. To resize for a different shoe length, update only
`MAX_ROUNDS`.

## Dependencies

```toml
arrayvec = { version = "0.7", default-features = false }
kev-rs = "0.2"
```

## License

LGPL-3.0-only
