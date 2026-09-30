// dex_aggregator_swap (json quote + typed execute)
//
// Quote a swap on the intear dex-aggregator, print a minimal route
// table, let you pick a route, preview its transactions (dry run), and
// optionally execute the picked route with your near-kit signer.
//
// run:
//   cargo run --bin dex_aggregator_swap_bin -- [token_in] [amount_in] [token_out] [--execute]
//
// env (see .env.example; CLI args win over env):
//   DEX_AGG_TOKEN_IN / DEX_AGG_AMOUNT_IN / DEX_AGG_TOKEN_OUT
//   DEX_AGG_SLIPPAGE DEX_AGG_MAX_WAIT_MS DEX_AGG_DEXES DEX_AGG_REFERRER_ID
//   DEX_AGG_TRADER_ACCOUNT_ID DEX_AGG_SIGNING_PUBLIC_KEY
//   NEAR_NETWORK (must be mainnet to --execute) NEAR_ACCOUNT_ID NEAR_PRIVATE_KEY
//
// examples (quoting needs no credentials — the router is mainnet only):
//   cargo run --bin dex_aggregator_swap_bin
//   cargo run --bin dex_aggregator_swap_bin -- near 100000000000000000000000 usdt.tether-token.near
//   cargo run --bin dex_aggregator_swap_bin -- near 100000000000000000000000 wrap.near
//
// execute (needs a funded mainnet signer in env):
//   set -a; source .env; set +a
//   cargo run --bin dex_aggregator_swap_bin -- near 100000000000000000000000 usdt.tether-token.near --execute
//
// route-not-found demo (unknown token -> router answers `[]`):
//   cargo run --bin dex_aggregator_swap_bin -- near 100000000000000000000000 this-token-does-not-exist-xyz123.near
//
// =================================================
use intear_dex_aggregator_tool_box::fun::dex_aggregator::dex_aggregator_execute_route_fun::execute_route;
use intear_dex_aggregator_tool_box::fun::dex_aggregator::dex_aggregator_get_routes_fun_json::get_routes;
use intear_dex_aggregator_tool_box::lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT;
use intear_dex_aggregator_tool_box::lib::helper::dex_aggregator_format::{
    print_route_preview, print_routes_table,
};
use intear_dex_aggregator_tool_box::lib::types::dex_aggregator_quote_params_type::QUOTE_PARAMS_TYPE;
use std::env;
use std::io::{self, Write};
// =================================================
fn env_or(key: &str, fallback: &str) -> String {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn env_opt(keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|k| env::var(k).ok())
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let execute = raw_args.iter().any(|a| a == "--execute");
    let positional: Vec<&String> = raw_args.iter().filter(|a| *a != "--execute").collect();

    let token_in = positional
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("DEX_AGG_TOKEN_IN", "near"));
    let amount_in = positional
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("DEX_AGG_AMOUNT_IN", "100000000000000000000000"));
    let token_out = positional
        .get(2)
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("DEX_AGG_TOKEN_OUT", "usdt.tether-token.near"));

    let client = DEX_AGGREGATOR_CLIENT::from_env();
    let mut params = QUOTE_PARAMS_TYPE::amount_in(&token_in, &token_out, &amount_in);
    if let Some(s) = env_opt(&["DEX_AGG_SLIPPAGE"]) {
        let slippage: f64 = s
            .parse()
            .map_err(|_| format!("DEX_AGG_SLIPPAGE is not a decimal: {s}"))?;
        params = params.with_fixed_slippage(slippage);
    }
    if let Some(ms) = env_opt(&["DEX_AGG_MAX_WAIT_MS", "DEX_AGGREGATOR_MAX_WAIT_MS"]) {
        let ms: u64 = ms
            .parse()
            .map_err(|_| format!("DEX_AGG_MAX_WAIT_MS is not an integer: {ms}"))?;
        params = params.with_max_wait_ms(ms);
    }
    if let Some(dexes) = env_opt(&["DEX_AGG_DEXES", "DEX_AGGREGATOR_DEXES"]) {
        params = params.with_dexes(&dexes);
    }
    if let Some(referrer) = env_opt(&["DEX_AGG_REFERRER_ID"]) {
        params = params.with_referrer_id(&referrer);
    }
    // Default the trader to the signer so storage-deposit legs are
    // included when you go on to execute.
    if let Some(trader) =
        env_opt(&["DEX_AGG_TRADER_ACCOUNT_ID"]).or_else(|| env_opt(&["NEAR_ACCOUNT_ID"]))
    {
        params = params.with_trader_account_id(&trader);
    }
    if let Some(key) = env_opt(&["DEX_AGG_SIGNING_PUBLIC_KEY"]) {
        params = params.with_signing_public_key(&key);
    }

    println!(
        "Quoting {amount_in} {token_in} -> {token_out} via {} ...",
        client.base_url()
    );
    let routes = get_routes(&client, &params).await?;
    if routes.is_empty() {
        eprintln!("No routes found for {amount_in} {token_in} -> {token_out}.");
        eprintln!("The router answers `[]` when no DEX can serve a pair: check both token ids");
        eprintln!("(testnet tokens are unsupported — the router is mainnet only), try a smaller");
        eprintln!("amount, or leave DEX_AGG_DEXES empty to use every DEX.");
        std::process::exit(1);
    }
    println!("Found {} route(s) (best first):", routes.len());
    print_routes_table(&routes);

    print!("Select route [0]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let index: usize = if input.trim().is_empty() {
        0
    } else {
        input
            .trim()
            .parse()
            .map_err(|_| format!("invalid route number: {}", input.trim()))?
    };
    if index >= routes.len() {
        eprintln!("Route {index} is out of range (0..{}).", routes.len());
        std::process::exit(1);
    }
    let route = &routes[index];
    println!();
    print_route_preview(route);

    if !route.is_directly_executable() {
        eprintln!();
        eprintln!(
            "Route {index} (dex {}) needs a NEAR Intents leg: sign the NEP-413 message and",
            route.dex_id
        );
        eprintln!("POST it to the solver relay manually — this bin cannot auto-execute it.");
        std::process::exit(1);
    }

    if !execute {
        println!();
        println!("Dry run — no transaction sent. Re-run with --execute plus a funded mainnet");
        println!("signer (NEAR_NETWORK=mainnet NEAR_ACCOUNT_ID NEAR_PRIVATE_KEY) to swap.");
        return Ok(());
    }

    if env_or("NEAR_NETWORK", "testnet") != "mainnet" {
        eprintln!(
            "Refusing to execute: the intear router is mainnet only but NEAR_NETWORK is not mainnet."
        );
        eprintln!("Set NEAR_NETWORK=mainnet (with a funded mainnet signer) to execute.");
        std::process::exit(1);
    }
    if env_opt(&["NEAR_PRIVATE_KEY"]).is_none() {
        eprintln!("Refusing to execute: NEAR_PRIVATE_KEY is not set.");
        std::process::exit(1);
    }
    let near = near_kit::Near::from_env()?;
    println!();
    println!("Executing route {index} via {} ...", route.dex_id);
    let outcomes = execute_route(&near, route).await?;
    for (i, outcome) in outcomes.iter().enumerate() {
        let status = if outcome.is_success() {
            "SUCCESS"
        } else {
            "FAILED"
        };
        println!("  [tx {i}] {status} hash {}", outcome.transaction.hash);
        println!(
            "          https://nearblocks.io/txns/{}",
            outcome.transaction.hash
        );
    }
    Ok(())
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
