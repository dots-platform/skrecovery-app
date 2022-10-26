use signing::gg20_keygen::*;
use anyhow::{Result, Context};
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Cli = Cli::from_args();

    let mut output_file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args.output)
            .await
            .context("cannot create output file")?;
    
    let output = keygen(args).await?;

    tokio::io::copy(&mut output.as_slice(), &mut output_file)
    .await
    .context("save output to file")?;

    Ok(())
}