// Live router tests: known tokens return routes, unknown tokens
// return `[]` (mapped to a typed RouteNotFound by get_best_route).
//
// These hit https://router.intear.tech and need network access.
// run: cargo test --test dex_aggregator_routes_live
// =================================================
use intear_dex_aggregator_tool_box::lib::helper::dex_aggregator_client::DEX_AGGREGATOR_CLIENT;
use intear_dex_aggregator_tool_box::lib::types::dex_aggregator_error_type::DEX_AGGREGATOR_ERROR_TYPE;
use intear_dex_aggregator_tool_box::lib::types::dex_aggregator_quote_params_type::QUOTE_PARAMS_TYPE;
// =================================================
/// Known liquid pair: must return at least one route with a usable
/// estimate and executable instructions.
#[tokio::test]
async fn known_tokens_return_routes() {
    let client = DEX_AGGREGATOR_CLIENT::new();
    let params =
        QUOTE_PARAMS_TYPE::amount_in("near", "usdt.tether-token.near", "100000000000000000000000");
    let routes = client
        .get_routes(&params)
        .await
        .expect("router request for a known pair must succeed");
    assert!(
        !routes.is_empty(),
        "known pair near -> usdt.tether-token.near must return routes"
    );
    let best = &routes[0];
    assert!(!best.dex_id.is_empty());
    assert!(
        best.estimated_amount.best_amount().is_some(),
        "best route must carry an estimated amount"
    );
    assert!(
        !best.execution_instructions.is_empty(),
        "best route must carry execution instructions"
    );
}

/// Tokens that do not exist: the router answers `200` with `[]`, not
/// an HTTP error. `get_routes` surfaces the empty vec, `get_best_route`
/// maps it to a typed `RouteNotFound`.
#[tokio::test]
async fn unknown_tokens_return_empty_not_http_error() {
    let client = DEX_AGGREGATOR_CLIENT::new();
    let params = QUOTE_PARAMS_TYPE::amount_in(
        "near",
        "this-token-does-not-exist-xyz123.near",
        "100000000000000000000000",
    );
    let routes = client
        .get_routes(&params)
        .await
        .expect("unknown tokens must NOT fail the http call — router answers []");
    assert!(
        routes.is_empty(),
        "unknown token must yield zero routes, got {}",
        routes.len()
    );
    let err = client
        .get_best_route(&params)
        .await
        .expect_err("get_best_route must map [] to RouteNotFound");
    match err {
        DEX_AGGREGATOR_ERROR_TYPE::RouteNotFound {
            token_in,
            token_out,
            ..
        } => {
            assert_eq!(token_in, "near");
            assert_eq!(token_out, "this-token-does-not-exist-xyz123.near");
        }
        other => panic!("expected RouteNotFound, got: {other}"),
    }
}
// =================================================
// =================================================
// copyright 2026 by sleet.near
