# HOW TO USE — intear-dex-aggregator-tool-box

Full walkthrough: configure once, quote per trade, pick a route, execute with near-kit.

---

## 0. Prerequisites

- Rust stable (`cargo --version`).
- No NEAR credentials needed for quoting. Executing needs a funded **mainnet** account + full-access key. The intear router is **mainnet only** (testnet tokens return `[]`).

```sh
git clone https://github.com/sleet-rs/intear-dex-aggregator-tool-box
cd intear-dex-aggregator-tool-box
cp .env.example .env   # fill in only what you need; never commit .env
set -a; source .env; set +a
```

---

## 1. Client: configure once

`DEX_AGGREGATOR_CLIENT` holds everything that is constant across your trades. The per-trade referrer override still wins when set on the params.

```rust
use intear_dex_aggregator_tool_box::lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT;

let client = DEX_AGGREGATOR_CLIENT::new()
    .with_referrer_id("you.near")          // optional: tracked, sometimes paid, on Rhea/Aidols/Plach
    .with_default_dexes("Rhea,RheaDcl,Plach,Wrap") // optional: default is every DEX
    .with_max_wait_ms(5000)                // optional: default 2000 (max 60000)
    .with_api_key("...");                  // optional: bypasses per-IP rate limits

// or read DEX_AGGREGATOR_* env vars:
let client = DEX_AGGREGATOR_CLIENT::from_env();
```

## 2. Params: everything that differs per trade

```rust
use intear_dex_aggregator_tool_box::lib::types::dex_aggregator_quote_params_type::QUOTE_PARAMS_TYPE;

// sell exactly 0.1 NEAR for USDT, fixed 1% slippage (the default)
let params = QUOTE_PARAMS_TYPE::amount_in("near", "usdt.tether-token.near", "100000000000000000000000")
    .with_trader_account_id("you.near"); // include storage-deposit legs for execution

// buy exactly 5 USDT instead (Rhea has no amount_out support, so expect fewer routes)
let params = QUOTE_PARAMS_TYPE::amount_out("near", "usdt.tether-token.near", "5000000");

// other per-trade knobs:
let params = QUOTE_PARAMS_TYPE::amount_in("near", "usdt.tether-token.near", "100000000000000000000000")
    .with_fixed_slippage(0.005)                 // 0.5%
    .with_auto_slippage(0.05, 0.001)            // or: router picks within bounds
    .with_max_wait_ms(10_000)
    .with_dexes("Rhea,Wrap")                    // per-trade DEX allow-list
    .with_trader_account_id("you.near")         // storage-deposit legs
    .with_signing_public_key("ed25519:...")     // Intents legs (needs an add_public_key tx)
    .with_referrer_id("partner.near");          // per-trade referrer override
```

Token id format: `near` for native NEAR, otherwise the NEP-141 contract id (`usdt.tether-token.near`, or `nep141:usdt.tether-token.near` — passed through as-is). Amounts are decimal strings in the token's smallest unit.

## 3. Quote: typed routes

```rust
// every route, best first:
let routes = client.get_routes(&params).await?;
for route in &routes {
    println!("{} out={:?}", route.dex_id, route.estimated_amount.best_amount());
}

// or just the best one — `[]` becomes a typed RouteNotFound:
let best = client.get_best_route(&params).await?;
```

Useful `ROUTE_TYPE` helpers: `tx_count()`, `action_count()`, `is_directly_executable()`, `estimated_amount.best_amount()`, `worst_case_amount.best_amount()`. When `token_output` differs from your requested `token_out`, request a second `Wrap`-only quote from `token_output` to your target and run it after.

## 4. Pick: minimal table

```rust
use intear_dex_aggregator_tool_box::lib::helper::dex_aggregator_format::{print_routes_table, print_route_preview};

print_routes_table(&routes);   // | # | dex | estimated | worst-case | plan | exec | out |
print_route_preview(&routes[0]); // decoded receiver / method / gas / deposit / args
```

The `dex_aggregator_swap_bin` bin does exactly this interactively (quote → table → stdin pick → preview → optional `--execute`).

## 5. Execute: sign with your near-kit client

Bring any `&near_kit::Near` — built directly, or via `NEAR_KIT_CLIENT` from near-kit-tool-box-rs (same `near-kit 0.15.0` type, so it plugs straight in).

```rust
use intear_dex_aggregator_tool_box::fun::dex_aggregator::dex_aggregator_execute_route_fun::execute_route;

let near = near_kit::Near::from_env()?; // NEAR_NETWORK=mainnet, NEAR_ACCOUNT_ID, NEAR_PRIVATE_KEY
let outcomes = execute_route(&near, &best).await?;
for outcome in &outcomes {
    println!("https://nearblocks.io/txns/{}", outcome.transaction.hash);
}
```

Semantics: instructions run sequentially; each `NearTransaction` is one multi-action transaction using the router's exact gas + deposit. `continue_if_failed: true` legs are skipped (with a stderr note) on failure; anything else aborts. Refresh quotes just before sending — routes carry short deadlines.

## 6. Intents legs (manual)

A route containing an `IntentsQuote` instruction cannot be auto-executed: `execute_route` returns `IntentsRequiresManualSigning`. Sign `message_to_sign` as a NEP-413 message, POST it to `https://solver-relay-v2.chaindefuser.com/rpc` (see the [NEAR Intents solver-relay docs](https://docs.near-intents.org/near-intents/market-makers/bus/solver-relay)), then continue with the remaining instructions yourself.

## 7. Referral fees

Set the referrer once on the client (or per trade). Payouts differ per DEX: Rhea needs manual approval from the Rhea team (else tracking only), Aidols pays wNEAR per trade (needs `wrap.near` storage), Plach fees are configurable up to 5% via the Plach manage CLI. See the [router referral docs](https://docs.intear.tech/docs/dex-aggregator/#referral-fees).

---

## Testing the response difference

```sh
cargo test                      # unit (shape parsing) + live router tests + doctest
echo "" | cargo run --bin dex_aggregator_swap_bin   # known pair -> 2 routes + preview
echo "" | cargo run --bin dex_aggregator_swap_bin -- near 100000000000000000000000 this-token-does-not-exist-xyz123.near
# ^ unknown token -> `No routes found ...` (router answered `[]` with HTTP 200)
```

==================
<br/>
copyright 2026 by sleet.near
