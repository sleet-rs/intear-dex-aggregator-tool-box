use crate::lib::helper::dex_aggregator_decode::{
    decode_instruction_args, deposit_from_route, gas_from_route,
};
use crate::lib::types::dex_aggregator_error_type::DEX_AGGREGATOR_ERROR_TYPE;
use crate::lib::types::dex_aggregator_route_response_type::{
    EXECUTION_INSTRUCTION_TYPE, NEAR_TRANSACTION_ACTION_TYPE, ROUTE_TYPE,
};
use near_kit::{Action, Near};
// =================================================
/// Execute one quoted [`ROUTE_TYPE`] with your near-kit [`Near`]
/// client (from near-kit directly or from `NEAR_KIT_CLIENT` in
/// near-kit-tool-box-rs — both are the same type).
///
/// Each `NearTransaction` instruction becomes one multi-action NEAR
/// transaction: every `FunctionCall` action is rebuilt with its
/// base64-decoded args plus the router's exact `gas` and `deposit`,
/// then sent sequentially in instruction order. One
/// [`near_kit::FinalExecutionOutcome`] is returned per sent
/// transaction.
///
/// An instruction with `continue_if_failed: true` is skipped (with an
/// `eprintln!` note) when its transaction fails; otherwise the first
/// failure aborts and is returned.
///
/// `IntentsQuote` legs cannot be sent as plain transactions — the
/// first such leg aborts with `IntentsRequiresManualSigning` (sign the
/// NEP-413 message + POST to the solver relay yourself).
///
/// Requires a funded mainnet signer on `near` (the router is mainnet
/// only; refresh quotes a couple of seconds before executing since
/// routes carry short deadlines).
pub async fn execute_route(
    near: &Near,
    route: &ROUTE_TYPE,
) -> Result<Vec<near_kit::FinalExecutionOutcome>, DEX_AGGREGATOR_ERROR_TYPE> {
    let mut outcomes = Vec::new();
    for instruction in &route.execution_instructions {
        match instruction {
            EXECUTION_INSTRUCTION_TYPE::NearTransaction(tx) => {
                let mut builder = near.transaction(tx.receiver_id.as_str());
                for action in &tx.actions {
                    match action {
                        NEAR_TRANSACTION_ACTION_TYPE::FunctionCall(fc) => {
                            let context = format!("{}.{}", tx.receiver_id, fc.method_name);
                            let args = decode_instruction_args(&fc.args, &context)?;
                            builder = builder.add_action(Action::function_call(
                                fc.method_name.clone(),
                                args,
                                gas_from_route(fc.gas),
                                deposit_from_route(&fc.deposit)?,
                            ));
                        }
                    }
                }
                match builder.send().await {
                    Ok(outcome) => outcomes.push(outcome),
                    Err(err) => {
                        if tx.continue_if_failed == Some(true) {
                            eprintln!(
                                "note: tx to {} failed but continue_if_failed is set, skipping: {err}",
                                tx.receiver_id
                            );
                        } else {
                            return Err(DEX_AGGREGATOR_ERROR_TYPE::Near(err));
                        }
                    }
                }
            }
            EXECUTION_INSTRUCTION_TYPE::IntentsQuote(quote) => {
                return Err(DEX_AGGREGATOR_ERROR_TYPE::IntentsRequiresManualSigning {
                    dex_id: route.dex_id.clone(),
                    quote_hash: quote.quote_hash.clone(),
                });
            }
        }
    }
    Ok(outcomes)
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
