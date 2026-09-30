use crate::lib::types::dex_aggregator_route_response_type::{
    EXECUTION_INSTRUCTION_TYPE, ROUTE_TYPE,
};
// =================================================
/// Shorten a token id for tables (`usdt.tether-token.near` stays,
/// `nep141:<64-hex>` becomes `nep141:17208628…36133a1`).
pub fn short_token(token: &str) -> String {
    if token.len() > 24 {
        let mut s = String::with_capacity(26);
        s.push_str(&token[..12]);
        s.push('…');
        s.push_str(&token[token.len() - 7..]);
        s
    } else {
        token.to_string()
    }
}

/// One minimal row describing a route for the picker table.
pub fn route_row(index: usize, route: &ROUTE_TYPE) -> String {
    let est = route.estimated_amount.best_amount().unwrap_or("?");
    let worst = route.worst_case_amount.best_amount().unwrap_or("?");
    let mode = route.estimated_amount.mode();
    let exec = if route.is_directly_executable() {
        "tx"
    } else {
        "intents!"
    };
    format!(
        "| {:>2} | {:<9} | {mode}:{est:<26} | {mode}:{worst:<26} | {:>3} tx / {:>2} act | {:<5} | {} |",
        index,
        route.dex_id,
        route.tx_count(),
        route.action_count(),
        exec,
        short_token(&route.token_output),
    )
}

/// Print a minimal route table (best route first, as returned).
pub fn print_routes_table(routes: &[ROUTE_TYPE]) {
    println!(
        "|  # | dex       | estimated                  | worst-case                 | plan            | exec  | out |"
    );
    println!(
        "|----|-----------|----------------------------|----------------------------|-----------------|-------|-----|"
    );
    for (i, route) in routes.iter().enumerate() {
        println!("{}", route_row(i, route));
    }
}

/// Print a decoded preview of every instruction in a route (dry-run
/// view: receiver, method, gas, deposit, args).
pub fn print_route_preview(route: &ROUTE_TYPE) {
    println!("dex: {} | out: {}", route.dex_id, route.token_output);
    for (i, instruction) in route.execution_instructions.iter().enumerate() {
        match instruction {
            EXECUTION_INSTRUCTION_TYPE::NearTransaction(tx) => {
                println!("  [tx {i}] -> {}", tx.receiver_id);
                for action in &tx.actions {
                    match action {
                        crate::lib::types::dex_aggregator_route_response_type::NEAR_TRANSACTION_ACTION_TYPE::FunctionCall(
                            fc,
                        ) => {
                            println!(
                                "    - {} (gas {}, deposit {}): {}",
                                fc.method_name,
                                fc.gas,
                                fc.deposit,
                                fc.preview_args()
                            );
                        }
                    }
                }
            }
            EXECUTION_INSTRUCTION_TYPE::IntentsQuote(q) => {
                println!(
                    "  [intents {i}] manual NEP-413 signing needed (quote {})",
                    q.quote_hash
                );
            }
        }
    }
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
