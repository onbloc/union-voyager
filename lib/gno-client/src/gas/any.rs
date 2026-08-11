use gno_rpc::rpc_types::TxFee;
use serde::{Deserialize, Serialize};

use crate::gas::{GasFillerT, dynamic_gas_price, fixed};

#[derive(Debug, Clone)]
pub enum GasFiller {
    Fixed(TxFee),
    Simulate(fixed::GasFiller),
    DynamicGasPrice(dynamic_gas_price::GasFiller),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "config")]
pub enum Config {
    Fixed(TxFee),
    Simulate(fixed::GasFiller),
    DynamicGasPrice(dynamic_gas_price::Config),
}

impl Config {
    /// `client` is the chain module's own RPC connection, reused here rather
    /// than opening a second connection for `DynamicGasPrice`.
    pub fn into_gas_filler(self, client: gno_rpc::Client) -> GasFiller {
        match self {
            Config::Fixed(fee) => GasFiller::Fixed(fee),
            Config::Simulate(filler) => GasFiller::Simulate(filler),
            Config::DynamicGasPrice(config) => {
                GasFiller::DynamicGasPrice(dynamic_gas_price::GasFiller::new(config, client))
            }
        }
    }
}

impl GasFillerT for GasFiller {
    async fn max_gas(&self) -> u64 {
        match self {
            GasFiller::Fixed(fee) => fee.gas_wanted.try_into().unwrap_or(u64::MAX),
            GasFiller::Simulate(f) => f.max_gas().await,
            GasFiller::DynamicGasPrice(f) => f.max_gas().await,
        }
    }

    async fn mk_fee(&self, gas: u64) -> Result<TxFee, crate::BroadcastTxCommitError> {
        match self {
            GasFiller::Fixed(fee) => Ok(fee.clone()),
            GasFiller::Simulate(f) => f.mk_fee(gas).await,
            GasFiller::DynamicGasPrice(f) => f.mk_fee(gas).await,
        }
    }
}
