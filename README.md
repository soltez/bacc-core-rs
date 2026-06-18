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

### Scoreboard (`src/scoreboard.rs`)

- **`BaccScoreboard`** - Tracks all five standard baccarat road displays for a
  running shoe. Call `update(&BaccRound)` after each round.
- **`BaccBeadPlate`** - The bead plate road. Encodes to/from a lowercase hex
  string (2 hex chars per bead word).
- **`BaccBigRoad`** - The big road. Encodes to/from a lowercase hex string.
- **`BaccDerivedRoads`** - The three derived roads (Big Eye Boy, Small Road,
  Cockroach Pig), run-length encoded. Encodes to/from three lowercase hex
  strings.

### Utility (`src/lib.rs`)

- **`pip_value(card: CardInt) -> u8`** - Returns the baccarat pip value of a
  card (Ace=1, 2-9=pip, Ten/Jack/Queen/King=10).

## Encoding

All encode/decode pairs are lossless roundtrips:

| Type             | Encoded form       |
|------------------|--------------------|
| `BaccOutcome`    | `u32`              |
| `BaccRound`      | `u64`              |
| `BaccBeadPlate`  | hex `&str`         |
| `BaccBigRoad`    | hex `&str`         |
| `BaccDerivedRoads` | `[&str; 3]`      |

## Capacity

Scoreboard buffers are sized for a 96-round shoe via the `MAX_ROUNDS` constant
in `src/scoreboard.rs`. To resize for a different shoe length, update only
`MAX_ROUNDS`.

## Dependencies

```toml
arrayvec = { version = "0.7", default-features = false }
kev-rs = "0.2"
```

## License

LGPL-3.0-only
