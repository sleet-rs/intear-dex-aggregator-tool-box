# intear-dex-aggregator-tool-box

🦀 Typed Rust tool box for the [intear dex-aggregator](https://docs.intear.tech/docs/dex-aggregator/) (`GET https://router.intear.tech/route`), designed to be used **with [near-kit](https://docs.rs/near-kit/latest/near_kit/) and [near-kit-tool-box-rs](https://github.com/sleet-rs/near-kit-tool-box-rs)**: quote routes through this crate's client, sign + send them with your near-kit `Near` client.

ℹ️ quoting is read-only (no credentials). Executing needs a funded **mainnet** signer — the router is mainnet only.

---

## HOW TO USE

### 1. Configure the client once

Things that stay constant across trades (`referrer_id`, default DEX allow-list, wait budget, api key) live on the client. Everything that differs per trade lives on `QUOTE_PARAMS_TYPE`.

```rust
use intear_dex_aggregator_tool_box::lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT;
use intear_dex_aggregator_tool_box::lib::types::dex_aggregator_quote_params_type::QUOTE_PARAMS_TYPE;

// configure once (or DEX_AGGREGATOR_CLIENT::from_env() for env vars)
let client = DEX_AGGREGATOR_CLIENT::new()
    .with_referrer_id("you.near"); // optional: credited/paid on supported DEXes
```

### 2. Quote per trade

```rust
// everything that differs per trade goes here
let params = QUOTE_PARAMS_TYPE::amount_in(
    "near",                    // token_in (`near` or an NEP-141 contract id)
    "usdt.tether-token.near",  // token_out
    "100000000000000000000000", // amount_in in the token's smallest unit (0.1 NEAR)
)
.with_trader_account_id("you.near"); // enables storage-deposit legs for execution

// all routes, best first (empty vec = no DEX can serve the pair)
let routes = client.get_routes(&params).await?;

// or the single best route (empty `[]` maps to a typed RouteNotFound error)
let best = client.get_best_route(&params).await?;
```

`amount_out` mode, auto slippage, DEX allow-lists and per-trade referrer overrides are all on `QUOTE_PARAMS_TYPE` (see `docs/how_to_use.md`).

### 3. Execute with your near-kit client

`execute_route` takes the same `&near_kit::Near` you already use — whether built directly or via `NEAR_KIT_CLIENT` from near-kit-tool-box-rs (both are `near_kit 0.15.0`, so the types match). Each `NearTransaction` instruction becomes one multi-action transaction with the router's exact gas + deposit, sent sequentially.

```rust
use intear_dex_aggregator_tool_box::fun::dex_aggregator::dex_aggregator_execute_route_fun::execute_route;

let near = near_kit::Near::from_env()?; // NEAR_NETWORK=mainnet + signer for real swaps
let outcomes = execute_route(&near, &best).await?;
for outcome in &outcomes {
    println!("{} {}", outcome.transaction.hash, outcome.is_success());
}
```

Routes with a `NearIntents` leg abort with `IntentsRequiresManualSigning` — sign the NEP-413 message and POST it to the solver relay yourself (see docs).

### 4. Try the interactive bin

```sh
cp .env.example .env   # no credentials needed just to quote
set -a; source .env; set +a

# quote + pick a route + dry-run preview (default pair: 0.1 NEAR -> USDT)
cargo run --bin dex_aggregator_swap_bin

# custom pair / route-not-found demo (unknown token -> router answers `[]`)
cargo run --bin dex_aggregator_swap_bin -- near 100000000000000000000000 usdt.tether-token.near
cargo run --bin dex_aggregator_swap_bin -- near 100000000000000000000000 this-token-does-not-exist-xyz123.near

# execute the picked route (funded mainnet signer required)
cargo run --bin dex_aggregator_swap_bin -- near 100000000000000000000000 usdt.tether-token.near --execute
```

The bin prints a minimal table (`# | dex | estimated | worst-case | plan | exec | out`), prompts for the route number (default = best), previews every decoded transaction, then either stops (dry run) or sends.

---

## ERROR HANDLING

All failures are one typed enum: `DEX_AGGREGATOR_ERROR_TYPE`.

| Variant | Meaning |
|---|---|
| `RouteNotFound` | Router answered `200` with `[]` — no DEX serves the pair/amount. Check token ids (testnet tokens unsupported), try `amount_in` mode, a smaller amount, or more DEXes. |
| `IntentsRequiresManualSigning` | Route needs a NEAR Intents RFQ leg — manual NEP-413 signing required. |
| `InvalidParams` | Empty token id etc., caught before the request. |
| `Http` | Transport failure or non-2xx router status (with body snippet). |
| `Parse` | Body was not a route array (with body snippet). |
| `Base64Decode` / `InvalidNumber` | Malformed instruction payload. |
| `Near` | Wraps `near_kit::Error` from signing / sending. |

Verified live responses:

- known pair (`near -> usdt.tether-token.near`, 0.1 NEAR) → `200` with 2 routes (`Rhea`, `RheaDcl`), estimates + `NearTransaction` instructions.
- unknown token (`near -> this-token-does-not-exist-xyz123.near`) → `200` with `[]` → `RouteNotFound`.

Run the proof yourself: `cargo test` (unit + live router tests + doctest).

---

## LAYOUT

- `src/lib/helper/dex_aggregator_client.rs` — `DEX_AGGREGATOR_CLIENT` (configure once: base url, `referrer_id`, default dexes, wait budget, api key; `get_routes` / `get_best_route`).
- `src/lib/helper/dex_aggregator_decode.rs` — base64 / gas / deposit converters.
- `src/lib/helper/dex_aggregator_format.rs` — minimal table + preview printers.
- `src/lib/types/` — `ROUTE_TYPE` + instruction shapes, `QUOTE_PARAMS_TYPE` + `SLIPPAGE_TYPE`, `DEX_AGGREGATOR_ERROR_TYPE`. Fully typed; flexible deserializers accept the router's number-or-string `gas` / `deposit` / `deadline`.
- `src/fun/dex_aggregator/` — `get_routes` / `get_best_route` (json quote), `execute_route` (near-kit send).
- `src/bin/dex_aggregator_swap_bin.rs` — interactive quote → pick → preview → execute bin.
- `tests/dex_aggregator_routes_live.rs` — known vs unknown token response difference.
- `docs/how_to_use.md` — full walkthrough.
- `.env.example` — every env var the bin reads.

`near-kit` is pinned to `0.15.0` to stay type-compatible with near-kit-tool-box-rs (`NEAR_KIT_CLIENT` returns `near_kit::Near` from that version).

---

## CARGO COMMANDS

```sh
cargo run --bin dex_aggregator_swap_bin -- --help
cargo check
cargo test
cargo fmt
cargo clean
cargo doc --no-deps
```

---

## LINKS

- router docs: https://docs.intear.tech/docs/dex-aggregator/
- aggregator source: https://github.com/INTEARnear/dex-aggregator
- near-kit docs: https://docs.rs/near-kit/latest/near_kit/
- coding style reference: https://github.com/sleet-rs/near-kit-tool-box-rs

==================
<br/>
copyright 2026 by sleet.near
