use std::env;

use async_trait::async_trait;
use dtrust::client::Client;
use elliptic_curve::{ff::PrimeField};
use p256::{NonZeroScalar, Scalar, SecretKey};
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::str::FromStr;
use vsss_rs::{Shamir, Share};

#[path = "../util.rs"]
mod util;
use util::*;

const SEED: u64 =49;
#[async_trait]
pub trait SecretKeyRecoverable {
    // Encrypt the secret key?
    async fn upload_sk_and_pwd(&self, num_nodes: usize, id: String, sk: String, pwd: String);
    async fn upload_pwd_guess(&self, num_nodes: usize, id: String, pwd_guess: String);
    async fn aggregate_sk(&self, num_nodes: usize, id: String) -> Vec<u8>;
}

#[async_trait]
impl SecretKeyRecoverable for Client {
    async fn upload_sk_and_pwd(&self, num_nodes: usize, id: String, sk: String, pwd: String) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let sk_str = "AD302A6F48F74DD6F9D257F7149E4D06CD8936FE200AF67E08EF88D1CBA4525D";
        let nzs = NonZeroScalar::from_str(&sk_str).unwrap();

        // 32 for field size, 1 for identifier = 33
        let res = Shamir::<THRESHOLD, NUM_SERVERS>::split_secret::<Scalar, ChaCha20Rng, 33>(
            *nzs.as_ref(),
            rng,
        )
        .unwrap();

        println!("slice stuff {:?}, {}", res[0].value(), res[0].value().len());
        self.upload_blob(
            id.to_owned() + "sk.txt",
            res.map(|x| x.as_ref().to_vec()).to_vec(),
        )
        .await;

        let pwd_str = "1D46DC341A3190D7724B5692E77DEAA1CC02782980AFF034DB20289F4E5E3151"; // TODO: generate from plaintext
        let pwd_nzs = NonZeroScalar::from_str(&pwd_str).unwrap();
        // let seed = &pwd_str.as_bytes()[..32];
        let pwd_rng = &mut ChaCha20Rng::seed_from_u64(SEED);
        let pwd_shares = Shamir::<THRESHOLD, NUM_SERVERS>::split_secret::<Scalar, ChaCha20Rng, 33>(
            *pwd_nzs.as_ref(),
            pwd_rng,
        )
        .unwrap();
        self.upload_blob(
            id + "pwd.txt",
            pwd_shares.map(|x| x.as_ref().to_vec()).to_vec(),
        )
        .await;
        // TODO: generate salt r and h = H(r, k) and upload
    }

    async fn upload_pwd_guess(&self, num_nodes: usize, id: String, pwd_guess: String) {
        let pwd_rng = &mut ChaCha20Rng::seed_from_u64(SEED);
        let pwd_guess = NonZeroScalar::from_str(&pwd_guess).unwrap();
        let pwd_guess_shares = Shamir::<{THRESHOLD }, NUM_SERVERS>::split_secret::<
            Scalar,
            ChaCha20Rng,
            33,
        >(*pwd_guess.as_ref(), pwd_rng)
        .unwrap();
        self.upload_blob(
            id + "guess.txt",
            pwd_guess_shares.map(|x| x.as_ref().to_vec()).to_vec(),
        )
        .await;
    }
    async fn aggregate_sk(&self, num_nodes: usize, id: String) -> Vec<u8> {
        let sk_byte_shares = self.retrieve_blob(id + "recovered_sk.txt").await;
        let sk_byte_shares = sk_byte_shares.iter().map(|x| x.as_slice());
        let mut sk_shares = Vec::new();
        sk_byte_shares.for_each(|x| sk_shares.push(Share::try_from(x).unwrap()));
        // get back (2t, n) shares bc of multiplication
        const RECOVER_THRESHOLD: usize = THRESHOLD * 2;
        let res = Shamir::<RECOVER_THRESHOLD, NUM_SERVERS>::combine_shares::<Scalar, 33>(&sk_shares);
        assert!(res.is_ok());
        let scalar = res.unwrap();
        let nzs_dup = NonZeroScalar::from_repr(scalar.to_repr()).unwrap();
        let sk = SecretKey::from(nzs_dup);
        sk.to_be_bytes().to_vec()
        // TODO: check salt and hash
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];
    let node_addrs = [
        "http://127.0.0.1:50051",
        "http://127.0.0.1:50052",
        "http://127.0.0.1:50053",
        "http://127.0.0.1:50054",
        "http://127.0.0.1:50055"
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
                Ok(s) => s,
                Err(_) => {
                    eprintln!("error: pwd guess not a string");
                    panic!("");
                }
            };
            println!("Uploading guess ...");
            client
                .upload_pwd_guess(num_nodes, String::from(&id), pwd_guess.clone())
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

            let f = SecretKey::from_be_bytes(s.as_slice()).unwrap();

            println!("Recovered sk: {}", f.to_nonzero_scalar());
        }

        _ => println!("Missing/wrong arguments"),
    };
    Ok(())
}
