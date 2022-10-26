// Credits: ZenGo (https://github.com/ZenGo-X/multi-party-ecdsa)

use std::path::PathBuf;

use structopt::StructOpt;


#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(short, long, default_value = "http://localhost:8000/")]
    pub address: surf::Url,
    #[structopt(short, long, default_value = "default-signing")]
    pub room: String,
    #[structopt(short, long)]
    pub local_share: PathBuf,

    #[structopt(short, long, use_delimiter(true))]
    pub parties: Vec<u16>,
    #[structopt(short, long)]
    pub data_to_sign: String,
}

