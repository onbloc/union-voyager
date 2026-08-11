use gno_rpc::rpc_types::TxFee;
use serde::{Deserialize, Serialize};

use super::GasFillerT;
use crate::gas::u128_saturating_mul_f64;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GasFiller {
    #[serde(with = "::serde_utils::string")]
    pub gas_price: f64,
    pub gas_denom: String,
    #[serde(with = "::serde_utils::string")]
    pub gas_multiplier: f64,
    pub max_gas: u64,
    #[serde(default)]
    pub min_gas: u64,
}

impl GasFillerT for GasFiller {
    async fn max_gas(&self) -> u64 {
        self.max_gas
    }

    async fn mk_fee(&self, gas: u64) -> Result<TxFee, crate::BroadcastTxCommitError> {
        // gas limit = provided gas * multiplier, clamped between min_gas and max_gas
        let gas_limit = u128_saturating_mul_f64(gas.into(), self.gas_multiplier)
            .clamp(self.min_gas.into(), self.max_gas.into());

        // price the fee off of gas_limit (what's actually declared as gas_wanted),
        // not the pre-multiplier gas — otherwise a multiplier > 1 understates the
        // effective price-per-gas relative to the configured gas_price
        let amount = u128_saturating_mul_f64(gas_limit, self.gas_price);

        Ok(TxFee {
            gas_wanted: gas_limit.try_into().unwrap_or(i64::MAX),
            gas_fee: format!("{amount}{}", self.gas_denom),
        })
    }
}
