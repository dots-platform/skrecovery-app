use std::env;
use std::str::FromStr;

use async_trait::async_trait;
use dtrust::client::Client;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;

use ark_ff::fields::Field;
use ark_ff::Zero;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

use ark_bls12_381::Fr as F;

#[path = "../util.rs"]
mod util;
use util::shard_to_bytes;

#[async_trait]
pub trait SecretKeyRecoverable {
    // Encrypt the secret key?
    async fn upload_sk_and_pwd(&self, num_nodes: usize, id: String, sk: String, pwd: String);
    async fn upload_pwd_guess(&self, num_nodes: usize, id: String, pwd_guess: F);
    async fn aggregate_sk(&self, num_nodes: usize, id: String) -> Vec<u8>;
}

#[async_trait]
impl SecretKeyRecoverable for Client {
    // TODO I think we have to use
    async fn upload_sk_and_pwd(&self, num_nodes: usize, id: String, sk: String, pwd: String) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let sk_field = F::from_str(&sk).unwrap();
        let mut sk_shards_bytes = shard_to_bytes::<F>(sk_field, num_nodes, rng);
        sk_shards_bytes.push(Vec::new());
        self.upload_blob(id.to_owned() + "sk.txt", sk_shards_bytes)
            .await;

        let pwd_field = F::from_random_bytes(pwd.as_bytes()).unwrap();
        println!("pwd_field: {}", pwd_field);
        let mut pwd_shards_bytes = shard_to_bytes::<F>(pwd_field, num_nodes, rng);
        pwd_shards_bytes.push(Vec::new());
        self.upload_blob(id + "pwd.txt", pwd_shards_bytes).await;
    }

    async fn upload_pwd_guess(&self, num_nodes: usize, id: String, pwd_guess: F) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let mut guess_shards_bytes = shard_to_bytes::<F>(pwd_guess, num_nodes, rng);
        guess_shards_bytes.push(Vec::new());
        self.upload_blob(id + "guess.txt", guess_shards_bytes).await;
    }
    async fn aggregate_sk(&self, num_nodes: usize, id: String) -> Vec<u8> {
        let sk_shard_bytes = self.retrieve_blob(id + "recovered_sk.txt").await;
        let f = sk_shard_bytes[..num_nodes]
            .iter()
            .map(|v| F::deserialize_uncompressed(v.as_slice()).unwrap())
            .fold(F::zero(), |x, y| x + y);

        let mut buf = Vec::new();
        assert!(f.serialize_uncompressed(&mut buf).is_ok());
        buf
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];

    // Set up for 3 participating servers, and one beaver triple dealing server
    let node_addrs = [
        "http://127.0.0.1:50051",
        "http://127.0.0.1:50052",
        "http://127.0.0.1:50053",
        "http://127.0.0.1:50054",
    ];

    let cli_id = "user1";
    let mut client = Client::new(cli_id);

    let app_name = "rust_app";

    let num_nodes = node_addrs.len() - 1;
    client.setup(node_addrs.to_vec(), None);

    match &cmd[..] {
        "upload_sk_and_pwd" => {
            let id: String = match args[2].parse() {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("error: user-id not a string");
                    panic!("");
                }
            };
            let sk: String = match args[3].parse() {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("error: sk not a string");
                    panic!("");
                }
            };
            let pwd: String = match args[4].parse() {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("error: pwd not a string");
                    panic!("");
                }
            };
            println!("Uploading sk {}, pwd {} for user {}", sk, pwd, id);
            client.upload_sk_and_pwd(num_nodes, id, sk, pwd).await;
        }
        "recover_sk" => {
            let id: String = match args[2].parse() {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("error: user-id not a string");
                    panic!("");
                }
            };
            let pwd_guess = match args[3].parse::<String>() {
                Ok(s) => F::from_random_bytes(s.as_bytes()).unwrap(),
                Err(_) => {
                    eprintln!("error: pwd guess not a string");
                    panic!("");
                }
            };
            println!("Uploading guess ...");
            client
                .upload_pwd_guess(num_nodes, String::from(&id), pwd_guess)
                .await;
            println!("Guess uploaded");

            println!(
                "Recovering sk with pwd guess {}, for user {}",
                pwd_guess, id
            );

            let in_files = [
                id.to_owned() + "sk.txt",
                id.to_owned() + "pwd.txt",
                id.to_owned() + "guess.txt",
            ];

            let out_files = [id.to_owned() + "recovered_sk.txt"];

            client
                .exec(
                    app_name,
                    "skrecovery",
                    in_files.to_vec(),
                    out_files.to_vec(),
                )
                .await?;

            println!("Aggregating SK on client");
            let s = client.aggregate_sk(num_nodes, id).await;

            let f = F::deserialize_uncompressed(s.as_slice()).unwrap();

            println!("Recovered sk: {}", f);
        }

        _ => println!("Missing/wrong arguments"),
    };
    Ok(())
}
