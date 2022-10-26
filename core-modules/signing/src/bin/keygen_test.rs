use anyhow::{Context, Result};
use reqwest::Url;
use signing::gg20_keygen::*;
use signing::gg20_sm_manager::*;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let share_1 = PathBuf::from("local-share1.json");
    let share_2 = PathBuf::from("local-share2.json");
    let share_3 = PathBuf::from("local-share3.json");
    let default_address_1 = Url::parse("http://localhost:8000/")?;
    let default_address_2 = Url::parse("http://localhost:8000/")?;
    let default_address_3 = Url::parse("http://localhost:8000/")?;
    let buffer_time = 5;

    tokio::spawn(async move {
        run_manager().await;
    });

    tokio::time::sleep(std::time::Duration::from_secs(buffer_time)).await;

    let t1 = tokio::spawn(async move {
        let args = Cli {
            address: default_address_1,
            room: String::from("default-keygen"),
            output: share_1,
            index: 1,
            threshold: 1,
            number_of_parties: 3,
        };
        let output = keygen(args).await;
        output
    });

    tokio::time::sleep(std::time::Duration::from_secs(buffer_time)).await;

    let t2 = tokio::spawn(async move {
        let args = Cli {
            address: default_address_2,
            room: String::from("default-keygen"),
            output: share_2,
            index: 2,
            threshold: 1,
            number_of_parties: 3,
        };
        let output = keygen(args).await;
        output
    });
    tokio::time::sleep(std::time::Duration::from_secs(buffer_time)).await;

    let t3 = tokio::spawn(async move {
        let args = Cli {
            address: default_address_3,
            room: String::from("default-keygen"),
            output: share_3,
            index: 3,
            threshold: 1,
            number_of_parties: 3,
        };
        let output = keygen(args).await;
        output
    });
    tokio::time::sleep(std::time::Duration::from_secs(buffer_time)).await;

    let mut output_file_1 = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(PathBuf::from("local-share1.json"))
        .await
        .context("cannot create output file")?;

    let output = t1.await?.unwrap();

    tokio::io::copy(&mut output.as_slice(), &mut output_file_1)
        .await
        .context("save output to file")?;

    let mut output_file_2 = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(PathBuf::from("local-share2.json"))
        .await
        .context("cannot create output file")?;

    let output = t2.await?.unwrap();

    tokio::io::copy(&mut output.as_slice(), &mut output_file_2)
        .await
        .context("save output to file")?;

    let mut output_file_3 = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(PathBuf::from("local-share3.json"))
        .await
        .context("cannot create output file")?;

    let output = t3.await?.unwrap();

    tokio::io::copy(&mut output.as_slice(), &mut output_file_3)
        .await
        .context("save output to file")?;

    Ok(())
}
