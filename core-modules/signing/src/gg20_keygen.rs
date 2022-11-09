// Credits: ZenGo (https://github.com/ZenGo-X/multi-party-ecdsa)

use crate::gg20_sm_client;

use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use std::path::PathBuf;
use structopt::StructOpt;

use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::Keygen;
use round_based::async_runtime::AsyncProtocol;

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(short, long, default_value = "http://localhost:8000/")]
    pub address: surf::Url,
    #[structopt(short, long, default_value = "default-keygen")]
    pub room: String,
    #[structopt(short, long)]
    pub output: PathBuf,

    #[structopt(short, long)]
    pub index: u16,
    #[structopt(short, long)]
    pub threshold: u16,
    #[structopt(short, long)]
    pub number_of_parties: u16,
}

pub async fn keygen(args: Cli) -> Result<Vec<u8>> {
    let (_i, incoming, outgoing) = gg20_sm_client::join_computation(args.address, &args.room)
        .await
        .context("join computation")?;

    let incoming = incoming.fuse();
    tokio::pin!(incoming);
    tokio::pin!(outgoing);

    let keygen = Keygen::new(args.index, args.threshold, args.number_of_parties)?;

    let output = AsyncProtocol::new(keygen, incoming, outgoing)
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("protocol execution terminated with error: {}", e))?;
    
    serde_json::to_vec_pretty(&output).context("serialize output")
}
