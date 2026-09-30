// use near_kit::*;
// =================================================
/// Every failure mode of this toolbox in one typed enum.
///
/// The most common one to match on is `RouteNotFound`: the router
/// answers `200` with `[]` when no DEX can serve a pair (unknown token,
/// no liquidity, `amount_out` mode on Rhea-only pairs, ...). It is
/// deliberately distinct from transport / parse errors so callers can
/// e.g. fall back to another pair instead of retrying.
#[derive(Debug, thiserror::Error)]
pub enum DEX_AGGREGATOR_ERROR_TYPE {
    /// No DEX returned a route for this pair + amount. Check the token
    /// ids (testnet tokens are unsupported — the router is mainnet
    /// only), try a smaller amount, `amount_in` mode, or more `dexes`.
    #[error("no route found for {token_in} -> {token_out} ({amount_desc}): {hint}")]
    RouteNotFound {
        /// Requested input token.
        token_in: String,
        /// Requested output token.
        token_out: String,
        /// `in <amount>` or `out <amount>`.
        amount_desc: String,
        /// Actionable hint for the caller.
        hint: String,
    },

    /// The route needs a NEAR Intents request-for-quote leg, which
    /// cannot be executed as a plain transaction. Sign
    /// `message_to_sign` as a NEP-413 message and POST it to the
    /// solver relay (`https://solver-relay-v2.chaindefuser.com/rpc`),
    /// then continue with the remaining instructions manually.
    #[error(
        "route uses NEAR Intents (dex {dex_id}, quote {quote_hash}): sign the NEP-413 message and submit it to the solver relay manually"
    )]
    IntentsRequiresManualSigning {
        /// DEX serving the leg (usually `NearIntents`).
        dex_id: String,
        /// Quote identifier for the solver relay.
        quote_hash: String,
    },

    /// Quote parameters were inconsistent (e.g. empty token id).
    #[error("invalid quote params: {0}")]
    InvalidParams(String),

    /// HTTP transport failure or non-2xx router status.
    #[error("dex aggregator request failed: {0}")]
    Http(String),

    /// Router answered with a body that is not a route array.
    #[error("failed to parse dex aggregator response: {0}")]
    Parse(String),

    /// A base64 `args` payload in an instruction did not decode.
    #[error("base64 decode failed for {context}: {source}")]
    Base64Decode {
        /// Which field failed (receiver + method).
        context: String,
        /// Underlying decode error.
        source: base64::DecodeError,
    },

    /// A numeric field (`deposit`, ...) did not parse.
    #[error("invalid number in route ({context}): {value}")]
    InvalidNumber {
        /// Which field failed.
        context: String,
        /// Offending value.
        value: String,
    },

    /// A NEAR transaction (or client setup) failed via near-kit.
    #[error("near transaction failed: {0}")]
    Near(#[from] near_kit::Error),
}

impl From<reqwest::Error> for DEX_AGGREGATOR_ERROR_TYPE {
    /// Wrap any reqwest transport / status error as `Http`.
    fn from(err: reqwest::Error) -> Self {
        DEX_AGGREGATOR_ERROR_TYPE::Http(err.to_string())
    }
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
