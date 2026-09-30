use crate::lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT;
use crate::lib::types::dex_aggregator_error_type::DEX_AGGREGATOR_ERROR_TYPE;
use crate::lib::types::dex_aggregator_quote_params_type::QUOTE_PARAMS_TYPE;
use crate::lib::types::dex_aggregator_route_response_type::ROUTE_TYPE;
// =================================================
/// Fetch all quoted routes for a trade (best first) via raw router
/// JSON, using a pre-configured [`DEX_AGGREGATOR_CLIENT`].
///
/// Returns an empty vec when no DEX can serve the pair (unknown token,
/// no liquidity, ...). No signer required.
///
/// `params` carries everything that differs per trade; client-level
/// defaults (`referrer_id`, `dexes`, wait budget) apply underneath.
pub async fn get_routes(
    client: &DEX_AGGREGATOR_CLIENT,
    params: &QUOTE_PARAMS_TYPE,
) -> Result<Vec<ROUTE_TYPE>, DEX_AGGREGATOR_ERROR_TYPE> {
    client.get_routes(params).await
}

/// Fetch the single best route for a trade via raw router JSON.
///
/// Maps the router's empty array (`[]`) to a typed `RouteNotFound`
/// error so "unknown / unserved pair" is distinguishable from
/// transport failures.
pub async fn get_best_route(
    client: &DEX_AGGREGATOR_CLIENT,
    params: &QUOTE_PARAMS_TYPE,
) -> Result<ROUTE_TYPE, DEX_AGGREGATOR_ERROR_TYPE> {
    client.get_best_route(params).await
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
