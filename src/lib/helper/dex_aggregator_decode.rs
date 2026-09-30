use crate::lib::types::dex_aggregator_error_type::DEX_AGGREGATOR_ERROR_TYPE;
use near_kit::{Gas, NearToken};
// =================================================
/// Decode a base64 `args` payload from an execution instruction into
/// raw bytes ready for `Action::function_call`.
///
/// `context` names the failing field (receiver + method) for errors.
pub fn decode_instruction_args(
    base64_args: &str,
    context: &str,
) -> Result<Vec<u8>, DEX_AGGREGATOR_ERROR_TYPE> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(base64_args)
        .map_err(|source| DEX_AGGREGATOR_ERROR_TYPE::Base64Decode {
            context: context.to_string(),
            source,
        })
}

/// Convert a router `gas` value (plain u64 on the wire) into near-kit
/// [`Gas`].
pub fn gas_from_route(gas: u64) -> Gas {
    Gas::from_gas(gas)
}

/// Parse a router `deposit` (yoctoNEAR decimal string) into near-kit
/// [`NearToken`].
pub fn deposit_from_route(deposit: &str) -> Result<NearToken, DEX_AGGREGATOR_ERROR_TYPE> {
    deposit
        .trim()
        .parse::<u128>()
        .map(NearToken::from_yoctonear)
        .map_err(|_| DEX_AGGREGATOR_ERROR_TYPE::InvalidNumber {
            context: "deposit (expected yoctoNEAR decimal string)".to_string(),
            value: deposit.to_string(),
        })
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
