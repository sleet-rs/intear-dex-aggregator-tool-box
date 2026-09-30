use crate::lib::types::dex_aggregator_error_type::DEX_AGGREGATOR_ERROR_TYPE;
use crate::lib::types::dex_aggregator_quote_params_type::{
    QUOTE_AMOUNT_TYPE, QUOTE_PARAMS_TYPE, SLIPPAGE_TYPE,
};
use crate::lib::types::dex_aggregator_route_response_type::ROUTE_TYPE;
// =================================================
/// Default router base url (`https://router.intear.tech`).
pub const DEX_AGGREGATOR_BASE_URL_CONST: &str = "https://router.intear.tech";
/// Default per-request wait budget (ms).
pub const DEX_AGGREGATOR_DEFAULT_MAX_WAIT_MS_CONST: u64 = 2000;
/// Known DEX ids the router may quote (allow-list values for `dexes`).
pub const DEX_AGGREGATOR_KNOWN_DEXES_CONST: &str =
    "Rhea,Aidols,Wrap,RheaDcl,MetaPool,Linear,XRhea,RNear,Plach";
// =================================================
/// Reusable client for the intear dex-aggregator `GET /route` API.
///
/// Configure once (`referrer_id`, default `dexes`, wait budget, api
/// key); every trade then only passes a
/// [`QUOTE_PARAMS_TYPE`] with the fields that differ per swap.
///
/// The client holds no signer — signing stays with your near-kit
/// [`near_kit::Near`] (e.g. `NEAR_KIT_CLIENT` from near-kit-tool-box-rs),
/// which you hand to `execute_route` separately. This keeps quoting
/// (read-only, no credentials) cleanly apart from executing (needs a
/// mainnet signer).
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), intear_dex_aggregator_tool_box::lib::types::dex_aggregator_error_type::DEX_AGGREGATOR_ERROR_TYPE> {
/// use intear_dex_aggregator_tool_box::lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT;
/// use intear_dex_aggregator_tool_box::lib::types::dex_aggregator_quote_params_type::QUOTE_PARAMS_TYPE;
///
/// let client = DEX_AGGREGATOR_CLIENT::new().with_referrer_id("you.near");
/// let params = QUOTE_PARAMS_TYPE::amount_in("near", "usdt.tether-token.near", "100000000000000000000000");
/// let routes = client.get_routes(&params).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct DEX_AGGREGATOR_CLIENT {
    base_url: String,
    referrer_id: Option<String>,
    default_dexes: Option<String>,
    default_max_wait_ms: u64,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl Default for DEX_AGGREGATOR_CLIENT {
    /// Default client: public router, no referrer, all DEXes.
    fn default() -> Self {
        Self::new()
    }
}

impl DEX_AGGREGATOR_CLIENT {
    /// Create a client for the public router with default settings.
    pub fn new() -> Self {
        Self {
            base_url: DEX_AGGREGATOR_BASE_URL_CONST.to_string(),
            referrer_id: None,
            default_dexes: None,
            default_max_wait_ms: DEX_AGGREGATOR_DEFAULT_MAX_WAIT_MS_CONST,
            api_key: None,
            http: reqwest::Client::new(),
        }
    }

    /// Point at a custom router base url (default
    /// `https://router.intear.tech`).
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.trim_end_matches('/').to_string();
        self
    }

    /// Credit this account as referrer on every quote (per-trade
    /// `QUOTE_PARAMS_TYPE::referrer_id` still wins when set).
    pub fn with_referrer_id(mut self, referrer_id: &str) -> Self {
        self.referrer_id = Some(referrer_id.to_string());
        self
    }

    /// Restrict every quote to a comma-separated DEX allow-list
    /// (e.g. `"Rhea,RheaDcl,Plach"`). `None` (default) uses all DEXes.
    pub fn with_default_dexes(mut self, dexes: &str) -> Self {
        self.default_dexes = Some(dexes.to_string());
        self
    }

    /// Default wait budget (ms) used when a trade sets no
    /// `max_wait_ms`.
    pub fn with_max_wait_ms(mut self, max_wait_ms: u64) -> Self {
        self.default_max_wait_ms = max_wait_ms;
        self
    }

    /// API key for unlimited usage (sent as `&key=...`).
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.api_key = Some(api_key.to_string());
        self
    }

    /// Build a client from env vars (all optional, sane defaults):
    /// `DEX_AGGREGATOR_BASE_URL`, `DEX_AGGREGATOR_REFERRER_ID`,
    /// `DEX_AGGREGATOR_DEXES`, `DEX_AGGREGATOR_MAX_WAIT_MS`,
    /// `DEX_AGGREGATOR_API_KEY`. (`DEX_AGG_REFERRER_ID` / `DEX_AGG_DEXES`
    /// / `DEX_AGG_MAX_WAIT_MS` short aliases are accepted too.)
    pub fn from_env() -> Self {
        let mut client = Self::new();
        if let Ok(v) = std::env::var("DEX_AGGREGATOR_BASE_URL") {
            if !v.trim().is_empty() {
                client = client.with_base_url(v.trim());
            }
        }
        let referrer = std::env::var("DEX_AGGREGATOR_REFERRER_ID")
            .or_else(|_| std::env::var("DEX_AGG_REFERRER_ID"))
            .unwrap_or_default();
        if !referrer.trim().is_empty() {
            client = client.with_referrer_id(referrer.trim());
        }
        let dexes = std::env::var("DEX_AGGREGATOR_DEXES")
            .or_else(|_| std::env::var("DEX_AGG_DEXES"))
            .unwrap_or_default();
        if !dexes.trim().is_empty() {
            client = client.with_default_dexes(dexes.trim());
        }
        let wait = std::env::var("DEX_AGGREGATOR_MAX_WAIT_MS")
            .or_else(|_| std::env::var("DEX_AGG_MAX_WAIT_MS"))
            .unwrap_or_default();
        if let Ok(ms) = wait.trim().parse::<u64>() {
            client = client.with_max_wait_ms(ms);
        }
        if let Ok(key) = std::env::var("DEX_AGGREGATOR_API_KEY") {
            if !key.trim().is_empty() {
                client = client.with_api_key(key.trim());
            }
        }
        client
    }

    /// Router base url in use.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Configured default referrer, if any.
    pub fn referrer_id(&self) -> Option<&str> {
        self.referrer_id.as_deref()
    }

    /// Build the query pairs for one trade, layering per-trade values
    /// over client defaults.
    fn query_pairs(&self, params: &QUOTE_PARAMS_TYPE) -> Vec<(String, String)> {
        let mut q = vec![
            ("token_in".to_string(), params.token_in.clone()),
            ("token_out".to_string(), params.token_out.clone()),
        ];
        match &params.amount {
            QUOTE_AMOUNT_TYPE::In(v) => q.push(("amount_in".to_string(), v.clone())),
            QUOTE_AMOUNT_TYPE::Out(v) => q.push(("amount_out".to_string(), v.clone())),
        }
        q.push((
            "max_wait_ms".to_string(),
            params
                .max_wait_ms
                .unwrap_or(self.default_max_wait_ms)
                .to_string(),
        ));
        match params.slippage {
            SLIPPAGE_TYPE::Fixed { slippage } => {
                q.push(("slippage_type".to_string(), "Fixed".to_string()));
                q.push(("slippage".to_string(), slippage.to_string()));
            }
            SLIPPAGE_TYPE::Auto {
                max_slippage,
                min_slippage,
            } => {
                q.push(("slippage_type".to_string(), "Auto".to_string()));
                q.push(("max_slippage".to_string(), max_slippage.to_string()));
                q.push(("min_slippage".to_string(), min_slippage.to_string()));
            }
        }
        let dexes = params.dexes.clone().or_else(|| self.default_dexes.clone());
        if let Some(d) = dexes {
            if !d.trim().is_empty() {
                q.push(("dexes".to_string(), d));
            }
        }
        if let Some(t) = &params.trader_account_id {
            if !t.trim().is_empty() {
                q.push(("trader_account_id".to_string(), t.clone()));
            }
        }
        if let Some(k) = &params.signing_public_key {
            if !k.trim().is_empty() {
                q.push(("signing_public_key".to_string(), k.clone()));
            }
        }
        let referrer = params
            .referrer_id
            .clone()
            .or_else(|| self.referrer_id.clone());
        if let Some(r) = referrer {
            if !r.trim().is_empty() {
                q.push(("referrer_id".to_string(), r));
            }
        }
        if let Some(key) = &self.api_key {
            q.push(("key".to_string(), key.clone()));
        }
        q
    }

    /// Fetch all quoted routes for a trade (best first). Returns an
    /// empty vec when no DEX can serve the pair — use
    /// [`DEX_AGGREGATOR_CLIENT::get_best_route`] when you want that
    /// case as a typed
    /// [`DEX_AGGREGATOR_ERROR_TYPE::RouteNotFound`](crate::lib::types::dex_aggregator_error_type::DEX_AGGREGATOR_ERROR_TYPE::RouteNotFound).
    pub async fn get_routes(
        &self,
        params: &QUOTE_PARAMS_TYPE,
    ) -> Result<Vec<ROUTE_TYPE>, DEX_AGGREGATOR_ERROR_TYPE> {
        if params.token_in.trim().is_empty() || params.token_out.trim().is_empty() {
            return Err(DEX_AGGREGATOR_ERROR_TYPE::InvalidParams(
                "token_in and token_out must both be non-empty".to_string(),
            ));
        }
        let url = format!("{}/route", self.base_url);
        let res = self
            .http
            .get(&url)
            .query(&self.query_pairs(params))
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            let snippet: String = body.chars().take(300).collect();
            return Err(DEX_AGGREGATOR_ERROR_TYPE::Http(format!(
                "router answered {status}: {snippet}"
            )));
        }
        serde_json::from_str::<Vec<ROUTE_TYPE>>(&body).map_err(|e| {
            let snippet: String = body.chars().take(300).collect();
            DEX_AGGREGATOR_ERROR_TYPE::Parse(format!("{e} (body: {snippet})"))
        })
    }

    /// Fetch the single best route for a trade. Maps the router's empty
    /// array (`[]`) to a typed `RouteNotFound` error so callers can
    /// distinguish "unknown / unserved pair" from transport failures.
    pub async fn get_best_route(
        &self,
        params: &QUOTE_PARAMS_TYPE,
    ) -> Result<ROUTE_TYPE, DEX_AGGREGATOR_ERROR_TYPE> {
        let mut routes = self.get_routes(params).await?;
        if routes.is_empty() {
            return Err(DEX_AGGREGATOR_ERROR_TYPE::RouteNotFound {
                token_in: params.token_in.clone(),
                token_out: params.token_out.clone(),
                amount_desc: params.amount.describe(),
                hint: "router returned []. Check both token ids (testnet tokens are unsupported — the router is mainnet only), try amount_in mode, a smaller amount, or more dexes.".to_string(),
            });
        }
        Ok(routes.remove(0))
    }
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
