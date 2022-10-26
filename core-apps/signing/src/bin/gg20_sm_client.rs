use signing::gg20_sm_client::*;
use anyhow::{Result, Context};
use structopt::StructOpt;
use futures::{Sink, Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Cli = Cli::from_args();
    let client = SmClient::new(args.address, &args.room).context("create SmClient")?;
    match args.cmd {
        Cmd::Broadcast { message } => client
            .broadcast(&message)
            .await
            .context("broadcast message")?,
        Cmd::IssueIdx => {
            let index = client.issue_index().await.context("issue index")?;
            println!("Index: {}", index);
        }
        Cmd::Subscribe => {
            let messages = client.subscribe().await.context("subsribe")?;
            tokio::pin!(messages);
            while let Some(message) = messages.next().await {
                println!("{:?}", message);
            }
        }
    }
    Ok(())
}