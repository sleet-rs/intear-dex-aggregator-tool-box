use serde::{Deserialize, Deserializer};
// =================================================
/// Decode any JSON value into a `u64` (number or numeric string).
fn value_to_u64<E: serde::de::Error>(value: &serde_json::Value) -> Result<u64, E> {
    match value {
        serde_json::Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_i64().and_then(|i| u64::try_from(i).ok()))
            .ok_or_else(|| E::custom(format!("invalid u64 number: {n}"))),
        serde_json::Value::String(s) => s
            .parse::<u64>()
            .map_err(|_| E::custom(format!("invalid u64 string: {s}"))),
        other => Err(E::custom(format!("expected u64, got: {other}"))),
    }
}

/// Deserialize a `u64` that the router may encode as either a JSON
/// number (`5000000000000`) or a JSON string (`"5000000000000"`).
/// Used for `gas` values in execution instructions.
fn de_u64_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    value_to_u64(&value)
}

/// Deserialize an optional `u64` (`deadline`) that may be `null`, a
/// number, or a string.
fn de_opt_u64_string_or_number<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => value_to_u64(&v).map(Some),
    }
}

/// Deserialize a yocto-amount `String` that the router may encode as
/// either a JSON string (`"100000000000000000000000"`) or a raw JSON
/// number (`1`). Used for `deposit` values.
fn de_string_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match &value {
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "expected amount string, got: {other}"
        ))),
    }
}
// =================================================
/// Estimated / worst-case amount of one side of a quoted route.
///
/// The router fills exactly one field depending on the quote mode:
/// `amount_out` for `amount_in` requests (most common), `amount_in`
/// for `amount_out` requests. Both are decimal strings in the token's
/// smallest unit.
#[derive(Debug, Clone, Deserialize)]
pub struct ROUTE_AMOUNT_TYPE {
    /// Quoted output amount (present for `amount_in` requests).
    #[serde(default)]
    pub amount_out: Option<String>,
    /// Required input amount (present for `amount_out` requests).
    #[serde(default)]
    pub amount_in: Option<String>,
}

impl ROUTE_AMOUNT_TYPE {
    /// The quoted amount regardless of quote mode (`out` wins when both
    /// are somehow present).
    pub fn best_amount(&self) -> Option<&str> {
        self.amount_out.as_deref().or(self.amount_in.as_deref())
    }

    /// Which side this estimate describes: `"out"` or `"in"`.
    pub fn mode(&self) -> &'static str {
        if self.amount_out.is_some() {
            "out"
        } else {
            "in"
        }
    }
}
// =================================================
/// A single `FunctionCall` action inside a `NearTransaction`
/// instruction, exactly as the router returns it.
///
/// `args` is base64-encoded (usually JSON); decode it with
/// [`crate::lib::helper::dex_aggregator_decode::decode_instruction_args`]
/// before previewing, or pass it through
/// [`FUNCTION_CALL_ACTION_TYPE::decoded_args`] when executing.
#[derive(Debug, Clone, Deserialize)]
pub struct FUNCTION_CALL_ACTION_TYPE {
    /// Contract method to call (e.g. `ft_transfer_call`).
    pub method_name: String,
    /// Base64-encoded call args.
    pub args: String,
    /// Gas units (number or string on the wire).
    #[serde(deserialize_with = "de_u64_string_or_number")]
    pub gas: u64,
    /// Attached deposit in yoctoNEAR (string or number on the wire).
    #[serde(deserialize_with = "de_string_string_or_number")]
    pub deposit: String,
}

impl FUNCTION_CALL_ACTION_TYPE {
    /// Decode the base64 `args` into raw bytes ready for
    /// `Action::function_call`.
    pub fn decoded_args(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(&self.args)
    }

    /// Decode `args` and try to interpret them as JSON for previews.
    /// Returns the raw string when the bytes are not valid JSON.
    pub fn preview_args(&self) -> String {
        match self.decoded_args() {
            Ok(bytes) => match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(v) => {
                    let s = v.to_string();
                    if s.len() > 300 {
                        format!("{}…", &s[..300])
                    } else {
                        s
                    }
                }
                Err(_) => format!("<{} non-utf8/json bytes>", bytes.len()),
            },
            Err(e) => format!("<undecodable args: {e}>"),
        }
    }
}
// =================================================
/// One action inside a `NearTransaction` instruction. The router only
/// ever emits `FunctionCall` today; the enum keeps us forward
/// compatible if transfer / deposit variants appear later.
#[derive(Debug, Clone, Deserialize)]
pub enum NEAR_TRANSACTION_ACTION_TYPE {
    /// Call a contract method.
    FunctionCall(FUNCTION_CALL_ACTION_TYPE),
}
// =================================================
/// A `NearTransaction` execution instruction: send one standard NEAR
/// transaction with these actions, in order.
#[derive(Debug, Clone, Deserialize)]
pub struct NEAR_TRANSACTION_INSTRUCTION_TYPE {
    /// Contract receiving the transaction.
    pub receiver_id: String,
    /// Actions to include in the single transaction, in order.
    pub actions: Vec<NEAR_TRANSACTION_ACTION_TYPE>,
    /// When true, keep executing later instructions even if this
    /// transaction fails. Defaults to false (stop on first error).
    #[serde(default)]
    pub continue_if_failed: Option<bool>,
}
// =================================================
/// A NEAR Intents (request-for-quote) instruction. This cannot be
/// executed as a plain transaction: sign `message_to_sign` as a NEP-413
/// message and POST it to the solver relay, then continue with the
/// remaining instructions. See the dex-aggregator docs for details.
#[derive(Debug, Clone, Deserialize)]
pub struct INTENTS_QUOTE_TYPE {
    /// NEP-413 message payload to sign.
    pub message_to_sign: serde_json::Value,
    /// Quote identifier for the solver relay.
    pub quote_hash: String,
}
// =================================================
/// One step of a quoted route. Process instructions sequentially.
#[derive(Debug, Clone, Deserialize)]
pub enum EXECUTION_INSTRUCTION_TYPE {
    /// Execute as a standard NEAR transaction (supported directly by
    /// [`crate::fun::dex_aggregator::dex_aggregator_execute_route_fun::execute_route`]).
    NearTransaction(NEAR_TRANSACTION_INSTRUCTION_TYPE),
    /// NEAR Intents quote — needs manual NEP-413 signing, not executed
    /// automatically.
    IntentsQuote(INTENTS_QUOTE_TYPE),
}
// =================================================
/// One quoted swap route, i.e. one element of the array returned by
/// `GET /route`.
///
/// When no DEX can serve a pair the router answers `200` with an empty
/// array (`[]`) — that maps to
/// [`crate::lib::types::dex_aggregator_error_type::DEX_AGGREGATOR_ERROR_TYPE::RouteNotFound`]
/// in [`crate::lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT::get_best_route`].
#[derive(Debug, Clone, Deserialize)]
pub struct ROUTE_TYPE {
    /// Route expiry (usually `null`); refresh a couple of seconds before.
    #[serde(default, deserialize_with = "de_opt_u64_string_or_number")]
    pub deadline: Option<u64>,
    /// True for AMM quotes (slippage applies), false for guaranteed
    /// request-for-quote legs.
    pub has_slippage: bool,
    /// Expected swap amount.
    pub estimated_amount: ROUTE_AMOUNT_TYPE,
    /// Minimum guaranteed amount after slippage.
    pub worst_case_amount: ROUTE_AMOUNT_TYPE,
    /// Which DEX serves this route (`Rhea`, `RheaDcl`, `Plach`, ...).
    pub dex_id: String,
    /// Instructions to execute in order.
    pub execution_instructions: Vec<EXECUTION_INSTRUCTION_TYPE>,
    /// Output token of this route. May differ from the requested
    /// `token_out` when the exact output location is not known yet —
    /// request a second `Wrap`-only quote from here to the desired
    /// token in that case.
    pub token_output: String,
    /// Whether the output still needs unwrapping. Absent in older
    /// responses — defaults to false.
    #[serde(default)]
    pub needs_unwrap: bool,
}

impl ROUTE_TYPE {
    /// Number of on-chain transactions this route needs (one per
    /// `NearTransaction` instruction; Intents legs need off-chain work).
    pub fn tx_count(&self) -> usize {
        self.execution_instructions
            .iter()
            .filter(|i| matches!(i, EXECUTION_INSTRUCTION_TYPE::NearTransaction(_)))
            .count()
    }

    /// Total number of function-call actions across all instructions.
    pub fn action_count(&self) -> usize {
        self.execution_instructions
            .iter()
            .map(|i| match i {
                EXECUTION_INSTRUCTION_TYPE::NearTransaction(tx) => tx.actions.len(),
                EXECUTION_INSTRUCTION_TYPE::IntentsQuote(_) => 0,
            })
            .sum()
    }

    /// True when every instruction is a plain transaction this toolbox
    /// can execute automatically.
    pub fn is_directly_executable(&self) -> bool {
        self.execution_instructions
            .iter()
            .all(|i| matches!(i, EXECUTION_INSTRUCTION_TYPE::NearTransaction(_)))
    }
}
// =================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_live_rhea_route_shape() {
        let raw = r#"{
            "deadline": null,
            "has_slippage": true,
            "estimated_amount": {"amount_out": "498715"},
            "worst_case_amount": {"amount_out": "493727"},
            "dex_id": "Rhea",
            "execution_instructions": [{
                "NearTransaction": {
                    "receiver_id": "wrap.near",
                    "actions": [{
                        "FunctionCall": {
                            "method_name": "near_deposit",
                            "args": "e30=",
                            "gas": 5000000000000,
                            "deposit": "100000000000000000000000"
                        }
                    }]
                }
            }],
            "needs_unwrap": false,
            "token_output": "nep141:abc.near"
        }"#;
        let route: ROUTE_TYPE = serde_json::from_str(raw).unwrap();
        assert_eq!(route.dex_id, "Rhea");
        assert_eq!(route.estimated_amount.best_amount(), Some("498715"));
        assert_eq!(route.tx_count(), 1);
        assert!(route.is_directly_executable());
        assert!(!route.needs_unwrap);
    }

    #[test]
    fn parses_string_gas_and_missing_needs_unwrap() {
        let raw = r#"{
            "deadline": null,
            "has_slippage": true,
            "estimated_amount": {"amount_in": "20403236107106561706676"},
            "worst_case_amount": {"amount_in": "20480303116855422752311"},
            "dex_id": "Plach",
            "execution_instructions": [{
                "NearTransaction": {
                    "receiver_id": "dex.intear.near",
                    "continue_if_failed": true,
                    "actions": [{
                        "FunctionCall": {
                            "method_name": "deposit_near",
                            "args": "e30=",
                            "gas": "280000000000000",
                            "deposit": "20480303116855422752311"
                        }
                    }]
                }
            }],
            "token_output": "near"
        }"#;
        let route: ROUTE_TYPE = serde_json::from_str(raw).unwrap();
        assert_eq!(route.estimated_amount.mode(), "in");
        assert!(!route.needs_unwrap);
        match &route.execution_instructions[0] {
            EXECUTION_INSTRUCTION_TYPE::NearTransaction(tx) => {
                assert_eq!(tx.continue_if_failed, Some(true));
                match &tx.actions[0] {
                    NEAR_TRANSACTION_ACTION_TYPE::FunctionCall(fc) => {
                        assert_eq!(fc.gas, 280_000_000_000_000);
                        assert_eq!(fc.decoded_args().unwrap(), b"{}");
                    }
                }
            }
            _ => panic!("expected NearTransaction"),
        }
    }

    #[test]
    fn parses_intents_quote_instruction() {
        let raw = r#"{
            "deadline": null,
            "has_slippage": false,
            "estimated_amount": {"amount_out": "10"},
            "worst_case_amount": {"amount_out": "10"},
            "dex_id": "NearIntents",
            "execution_instructions": [{
                "IntentsQuote": {
                    "message_to_sign": {"foo": "bar"},
                    "quote_hash": "abc123"
                }
            }],
            "token_output": "near"
        }"#;
        let route: ROUTE_TYPE = serde_json::from_str(raw).unwrap();
        assert!(!route.is_directly_executable());
        assert_eq!(route.tx_count(), 0);
    }

    #[test]
    fn empty_array_means_no_route() {
        let routes: Vec<ROUTE_TYPE> = serde_json::from_str("[]").unwrap();
        assert!(routes.is_empty());
    }
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
