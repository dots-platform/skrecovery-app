use anyhow::{anyhow, Context, Result};
use dtrust::utils::init_app;
// use rocket::serde::json::Json;
use futures::{Sink, SinkExt, Stream, StreamExt};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::Keygen;
use round_based::{async_runtime::AsyncProtocol, Msg};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_serde::{formats::Json, Framed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

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

type WrappedStream = FramedRead<OwnedReadHalf, LengthDelimitedCodec>;
type WrappedSink = FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>;

// We use the unit type in place of the message types since we're
// only dealing with one half of the IO
type SerStream = Framed<WrappedStream, Msg<()>, (), Json<Msg<()>, ()>>;
type DeSink = Framed<WrappedSink, (), Msg<()>, Json<(), Msg<()>>>;

fn wrap_stream(stream: TcpStream) -> (SerStream, DeSink) {
    let (read, write) = stream.into_split();
    let stream = WrappedStream::new(read, LengthDelimitedCodec::new());
    let sink = WrappedSink::new(write, LengthDelimitedCodec::new());
    (
        SerStream::new(stream, Json::default()),
        DeSink::new(sink, Json::default()),
    )
}

#[derive(Serialize, Deserialize, Debug)]
struct MyMessage {
    field: String,
}

pub fn join_computation<M>() -> Result<(
    u16,
    impl Stream<Item = Result<Msg<M>>>,
    impl Sink<Msg<M>, Error = anyhow::Error>,
)>
where
    M: Serialize + DeserializeOwned,
{
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;

    println!("rank {:?}", rank);
    println!("func name {:?}", func_name);

    let tcp_stream = TcpStream::from_std(socks[usize::from(rank)])?;
    let (mut incoming, mut outgoing) = wrap_stream(tcp_stream);
    
    Ok((u16::from(rank), incoming, outgoing))
}

pub async fn keygen(args: Cli) -> Result<Vec<u8>> {
    let (_i, incoming, outgoing) = join_computation().context("join computation")?;

    let incoming = incoming.fuse();

    let keygen = Keygen::new(args.index, args.threshold, args.number_of_parties)?;

    let output = AsyncProtocol::new(keygen, incoming, outgoing)
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("protocol execution terminated with error: {}", e))?;

    serde_json::to_vec_pretty(&output).context("serialize output")
}

pub fn signing(args: Cli) -> Result<()> {
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;
    // Keygen
    if func_name == "keygen" {
        let args: Cli = Cli::from_args();

        let mut output_file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args.output)
            .await
            .context("cannot create output file")?;

        let output = keygen(args).await?;

    // Signing
    } else {
        let args: Cli = Cli::from_args();

        let mut output_file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args.output)
            .await
            .context("cannot create output file")?;

        let output = signing(args)?;
    }
    Ok(())
}
