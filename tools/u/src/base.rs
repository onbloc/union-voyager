use anyhow::Result;
use clap::Subcommand;

pub mod v1;

#[derive(Debug, Subcommand)]
pub enum Cmd {
    #[clap(subcommand)]
    V1(v1::Cmd),
}

impl Cmd {
    pub async fn run(self) -> Result<()> {
        match self {
            Cmd::V1(cmd) => cmd.run().await,
        }
    }
}
