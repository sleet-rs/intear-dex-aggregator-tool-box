// use near_kit::*;
// =================================================
/// Which side of the swap the caller fixes.
///
/// * `In` — most common: "I sell exactly X", the router quotes what
///   you receive (`amount_in=...` on the wire).
/// * `Out` — "I want exactly Y", the router quotes what you must pay
///   (`amount_out=...` on the wire; Rhea does not serve this mode so
///   fewer routes may come back).
#[derive(Debug, Clone)]
pub enum QUOTE_AMOUNT_TYPE {
    /// Fixed input amount in the input token's smallest unit.
    In(String),
    /// Desired output amount in the output token's smallest unit.
    Out(String),
}

impl QUOTE_AMOUNT_TYPE {
    /// Short description used in errors (`in 1000` / `out 500`).
    pub fn describe(&self) -> String {
        match self {
            QUOTE_AMOUNT_TYPE::In(v) => format!("in {v}"),
            QUOTE_AMOUNT_TYPE::Out(v) => format!("out {v}"),
        }
    }
}
// =================================================
/// Slippage tolerance for a quote.
///
/// * `Fixed` — one decimal, e.g. `0.01` = 1% (`slippage_type=Fixed`).
/// * `Auto` — router picks between `min` and `max` from market
///   conditions (`slippage_type=Auto`).
#[derive(Debug, Clone)]
pub enum SLIPPAGE_TYPE {
    /// Fixed slippage as a decimal (e.g. `0.01` for 1%).
    Fixed {
        /// Slippage decimal, e.g. `0.01`.
        slippage: f64,
    },
    /// Auto slippage between `min_slippage` and `max_slippage`.
    Auto {
        /// Upper bound decimal, e.g. `0.05`.
        max_slippage: f64,
        /// Lower bound decimal, e.g. `0.001`.
        min_slippage: f64,
    },
}

impl Default for SLIPPAGE_TYPE {
    /// Sensible default: fixed 1% slippage.
    fn default() -> Self {
        SLIPPAGE_TYPE::Fixed { slippage: 0.01 }
    }
}
// =================================================
/// Per-trade quote parameters. Everything the router needs that varies
/// per swap lives here; things that stay constant across trades
/// (`referrer_id`, default `dexes`, `max_wait_ms`, api key) live on
/// [`crate::lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT`]
/// and can still be overridden per trade via the `Option` fields below.
///
/// Token id format: `near` for native NEAR, otherwise the NEP-141
/// contract id (a `nep141:` prefix is accepted and passed through).
/// Amounts are decimal strings in the token's smallest unit.
#[derive(Debug, Clone)]
pub struct QUOTE_PARAMS_TYPE {
    /// Token to swap from (`near` or a contract id).
    pub token_in: String,
    /// Token to swap to (`near` or a contract id).
    pub token_out: String,
    /// Fixed input vs desired output amount.
    pub amount: QUOTE_AMOUNT_TYPE,
    /// Slippage tolerance.
    pub slippage: SLIPPAGE_TYPE,
    /// Max time (ms) the router may spend quoting (up to 60000).
    /// `None` falls back to the client's default.
    pub max_wait_ms: Option<u64>,
    /// Comma-separated DEX allow-list (e.g. `"Rhea,Wrap"`).
    /// `None` falls back to the client's default (all DEXes).
    pub dexes: Option<String>,
    /// Account used for storage-deposit legs. `None` omits the field
    /// (pass the signer account when you plan to execute).
    pub trader_account_id: Option<String>,
    /// Public key for NEAR Intents legs (`add_public_key` needed).
    /// `None` omits the field.
    pub signing_public_key: Option<String>,
    /// Per-trade referrer override. `None` falls back to the client's
    /// configured referrer.
    pub referrer_id: Option<String>,
}

impl QUOTE_PARAMS_TYPE {
    /// Quote selling an exact `amount_in` with fixed 1% slippage.
    pub fn amount_in(token_in: &str, token_out: &str, amount_in: &str) -> Self {
        Self {
            token_in: token_in.to_string(),
            token_out: token_out.to_string(),
            amount: QUOTE_AMOUNT_TYPE::In(amount_in.to_string()),
            slippage: SLIPPAGE_TYPE::default(),
            max_wait_ms: None,
            dexes: None,
            trader_account_id: None,
            signing_public_key: None,
            referrer_id: None,
        }
    }

    /// Quote buying an exact `amount_out` with fixed 1% slippage.
    pub fn amount_out(token_in: &str, token_out: &str, amount_out: &str) -> Self {
        Self {
            amount: QUOTE_AMOUNT_TYPE::Out(amount_out.to_string()),
            ..Self::amount_in(token_in, token_out, "")
        }
    }

    /// Set a fixed slippage decimal (e.g. `0.01`).
    pub fn with_fixed_slippage(mut self, slippage: f64) -> Self {
        self.slippage = SLIPPAGE_TYPE::Fixed { slippage };
        self
    }

    /// Set auto slippage bounds.
    pub fn with_auto_slippage(mut self, max_slippage: f64, min_slippage: f64) -> Self {
        self.slippage = SLIPPAGE_TYPE::Auto {
            max_slippage,
            min_slippage,
        };
        self
    }

    /// Override the router wait budget for this trade.
    pub fn with_max_wait_ms(mut self, max_wait_ms: u64) -> Self {
        self.max_wait_ms = Some(max_wait_ms);
        self
    }

    /// Restrict this trade to a comma-separated DEX allow-list.
    pub fn with_dexes(mut self, dexes: &str) -> Self {
        self.dexes = Some(dexes.to_string());
        self
    }

    /// Set the trader account (enables storage-deposit legs).
    pub fn with_trader_account_id(mut self, trader_account_id: &str) -> Self {
        self.trader_account_id = Some(trader_account_id.to_string());
        self
    }

    /// Set the signing public key (needed for Intents legs).
    pub fn with_signing_public_key(mut self, signing_public_key: &str) -> Self {
        self.signing_public_key = Some(signing_public_key.to_string());
        self
    }

    /// Override the client's referrer for this trade.
    pub fn with_referrer_id(mut self, referrer_id: &str) -> Self {
        self.referrer_id = Some(referrer_id.to_string());
        self
    }
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
