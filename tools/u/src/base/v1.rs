use alloy::providers::ProviderBuilder;
use anyhow::Result;
use base_client::{
    finalized_l2_block_number_of_l1_block_number, finalized_l2_block_of_game_index,
    latest_game_of_l1_block_number,
};
use clap::Subcommand;
use unionlabs::primitives::{H160, U256};

use crate::print_json;

#[derive(Debug, Subcommand)]
pub enum Cmd {
    FinalizedL2BlockNumberOfL1BlockNumber {
        #[arg(long, short = 'r')]
        l1_rpc_url: String,
        #[arg(long, short = 'a')]
        l1_dispute_game_factory_address: H160,
        #[arg(long, short = 'b')]
        l1_block_number: u64,
    },
    FinalizedL2BlockOfGameIndex {
        #[arg(long, short = 'r')]
        l1_rpc_url: String,
        #[arg(long, short = 'a')]
        l1_dispute_game_factory_address: H160,
        #[arg(long, short = 'b')]
        l1_block_number: u64,
        #[arg(long, short = 'g')]
        game_index: U256,
    },
    LatestGameOfL1BlockNumber {
        #[arg(long, short = 'r')]
        l1_rpc_url: String,
        #[arg(long, short = 'a')]
        l1_dispute_game_factory_address: H160,
        #[arg(long, short = 'b')]
        l1_block_number: u64,
    },
}

impl Cmd {
    pub async fn run(self) -> Result<()> {
        match self {
            Cmd::FinalizedL2BlockNumberOfL1BlockNumber {
                l1_rpc_url,
                l1_dispute_game_factory_address,
                l1_block_number,
            } => {
                let block_number = finalized_l2_block_number_of_l1_block_number(
                    &ProviderBuilder::new().connect(&l1_rpc_url).await?,
                    l1_dispute_game_factory_address,
                    l1_block_number,
                )
                .await?;

                print_json(&block_number);
            }
            Cmd::FinalizedL2BlockOfGameIndex {
                l1_rpc_url,
                l1_dispute_game_factory_address: l1_dispute_game_factory_proxy,
                l1_block_number,
                game_index,
            } => {
                let l2_block_number = finalized_l2_block_of_game_index(
                    &ProviderBuilder::new().connect(&l1_rpc_url).await?,
                    l1_block_number,
                    l1_dispute_game_factory_proxy,
                    game_index,
                )
                .await?;

                print_json(&l2_block_number);
            }
            Cmd::LatestGameOfL1BlockNumber {
                l1_rpc_url,
                l1_dispute_game_factory_address,
                l1_block_number,
            } => {
                let l2_block = latest_game_of_l1_block_number(
                    &ProviderBuilder::new().connect(&l1_rpc_url).await?,
                    l1_block_number,
                    l1_dispute_game_factory_address,
                )
                .await?;

                print_json(&l2_block);
            }
        }

        Ok(())
    }
}
