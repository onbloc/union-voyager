use gno_rpc::rpc_types::TxFee;
use num_rational::BigRational;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::gas::{GasFillerT, u128_saturating_mul_f64};

/// Queries gno's own protocol-level dynamic gas price (`auth/gasprice`), which
/// the chain recomputes every block from actual gas usage vs. a target block
/// utilization ratio (an EIP-1559-style base fee, not a `min-gas-prices`
/// per-node config).
#[derive(Debug, Clone)]
pub struct GasFiller {
    max_gas: u64,
    gas_multiplier: f64,
    denom_override: Option<String>,
    client: gno_rpc::Client,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub max_gas: u64,
    #[serde(with = "::serde_utils::string_opt")]
    pub gas_multiplier: Option<f64>,
    /// Used only when `auth/gasprice` reports a zero/empty price (no denom) —
    /// never overrides a denom the chain actually reported.
    pub denom: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GasPriceResponse {
    #[serde(with = "::serde_utils::string")]
    gas: i64,
    price: String,
}

impl GasFiller {
    /// Reuses the chain module's own RPC connection rather than opening a
    /// second one to the same node.
    pub fn new(config: Config, client: gno_rpc::Client) -> Self {
        Self {
            max_gas: config.max_gas,
            gas_multiplier: config.gas_multiplier.unwrap_or(1.0),
            denom_override: config.denom,
            client,
        }
    }

    /// Returns `(gas_units, price_amount, price_denom)`, i.e. the gno-side
    /// `GasPrice{ Gas, Price }` — the effective price per unit of gas is
    /// `price_amount / gas_units`, not `price_amount` alone.
    pub(crate) async fn get_gas_price(
        &self,
    ) -> Result<(i64, u128, String), crate::BroadcastTxCommitError> {
        let response = self
            .client
            .abci_query("auth/gasprice", &[], None, false)
            .await?;

        if let Some(error) = response.response.response_base.error {
            return Err(crate::BroadcastTxCommitError::TxFailed {
                error,
                log: response.response.response_base.log,
            });
        }

        // unlike `.app/simulate` (which sets `Value`), the `auth` module's
        // custom querier writes its result to `ResponseBase.Data`
        let value = response.response.response_base.data.unwrap_or_default();

        parse_gas_price_response(&value)
    }
}

fn parse_gas_price_response(
    value: &[u8],
) -> Result<(i64, u128, String), crate::BroadcastTxCommitError> {
    let gas_price = serde_json::from_slice::<GasPriceResponse>(value)?;

    let split_at = gas_price
        .price
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(gas_price.price.len());

    let (amount, denom) = gas_price.price.split_at(split_at);

    // an empty price string is gno's zero-value ("no price set yet", e.g.
    // before any gas-usage history exists) — not malformed data
    let amount: u128 = if gas_price.price.is_empty() {
        0
    } else {
        amount
            .parse()
            .map_err(|_| crate::BroadcastTxCommitError::InvalidGasPrice {
                price: gas_price.price.clone(),
            })?
    };

    // a nonzero price with a non-positive `gas` denominator is a contradiction
    // per gno's own `GasPrice{ Gas, Price }` semantics ("price per Gas units");
    // silently treating it as 1 would compute a fee that's not what the chain
    // actually reported
    if amount > 0 && gas_price.gas <= 0 {
        return Err(crate::BroadcastTxCommitError::InvalidGasPrice {
            price: gas_price.price,
        });
    }

    Ok((gas_price.gas, amount, denom.to_owned()))
}

impl GasFillerT for GasFiller {
    async fn max_gas(&self) -> u64 {
        self.max_gas
    }

    #[instrument(
        skip_all,
        fields(
            self.max_gas = %self.max_gas,
            self.gas_multiplier = %self.gas_multiplier,
            gas = %gas,
        )
    )]
    async fn mk_fee(&self, gas: u64) -> Result<TxFee, crate::BroadcastTxCommitError> {
        // gas limit = provided gas * multiplier, clamped to max_gas
        let gas_limit = u128_saturating_mul_f64(gas.into(), self.gas_multiplier)
            .try_into()
            .unwrap_or(self.max_gas)
            .min(self.max_gas);

        let (price_gas_units, price_amount, price_denom) = self.get_gas_price().await?;

        // only fall back to the override when the chain didn't report a denom —
        // never override a denom the chain actually reported
        let denom = if price_denom.is_empty() {
            self.denom_override.clone().unwrap_or_default()
        } else {
            price_denom
        };

        // price is expressed as price_amount per price_gas_units, per gno's
        // `GasPrice{ Gas, Price }` semantics. `.max(1)` only guards the
        // legitimate 0/0 (no price set) case from a divide-by-zero — a
        // nonzero price with a non-positive gas denominator is already
        // rejected as InvalidGasPrice by get_gas_price above.
        let price_per_gas = BigRational::new(price_amount.into(), price_gas_units.max(1).into());

        let amount = price_per_gas * BigRational::from_integer(gas_limit.into());
        let amount = amount.ceil().to_integer().try_into().unwrap_or(u128::MAX);

        debug!(gas_limit, amount, %denom, "computed fee from dynamic gas price");

        Ok(TxFee {
            gas_wanted: gas_limit.try_into().unwrap_or(i64::MAX),
            gas_fee: format!("{amount}{denom}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_normal_price() {
        let (gas, amount, denom) =
            parse_gas_price_response(br#"{"gas": "1000", "price": "5ugnot"}"#).unwrap();

        assert_eq!((gas, amount, denom.as_str()), (1000, 5, "ugnot"));
    }

    #[test]
    fn parses_zero_price_as_no_price_set() {
        let (gas, amount, denom) =
            parse_gas_price_response(br#"{"gas": "0", "price": ""}"#).unwrap();

        assert_eq!((gas, amount, denom.as_str()), (0, 0, ""));
    }

    #[test]
    fn rejects_malformed_price() {
        let err = parse_gas_price_response(br#"{"gas": "1000", "price": "abcugnot"}"#).unwrap_err();

        assert!(matches!(
            err,
            crate::BroadcastTxCommitError::InvalidGasPrice { price } if price == "abcugnot"
        ));
    }

    #[test]
    fn rejects_nonzero_price_with_non_positive_gas() {
        let err = parse_gas_price_response(br#"{"gas": "0", "price": "5ugnot"}"#).unwrap_err();

        assert!(matches!(
            err,
            crate::BroadcastTxCommitError::InvalidGasPrice { price } if price == "5ugnot"
        ));

        let err = parse_gas_price_response(br#"{"gas": "-1", "price": "5ugnot"}"#).unwrap_err();

        assert!(matches!(
            err,
            crate::BroadcastTxCommitError::InvalidGasPrice { price } if price == "5ugnot"
        ));
    }

    #[tokio::test]
    #[ignore = "hits a live gno node"]
    async fn get_gas_price_against_live_node() {
        let client = gno_rpc::Client::new("http://23.20.153.250:26657")
            .await
            .unwrap();

        let filler = GasFiller::new(
            Config {
                max_gas: 1_000_000_000,
                gas_multiplier: None,
                denom: None,
            },
            client,
        );

        let (gas, amount, denom) = filler.get_gas_price().await.unwrap();

        println!("gas={gas} amount={amount} denom={denom}");

        assert!(gas > 0);
        assert!(!denom.is_empty());
    }
}
