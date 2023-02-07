use std::env;

use async_trait::async_trait;
use dtrust::client::Client;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;

use elliptic_curve::{ff::PrimeField, generic_array::GenericArray};
use p256::{NonZeroScalar, Scalar, SecretKey};
use vsss_rs::Shamir;

#[path = "../util.rs"]
mod util;
use util::shard_to_bytes;

#[async_trait]
pub trait SecretKeyRecoverable {
    // Encrypt the secret key?
    async fn upload_sk_and_pwd(&self, num_nodes: usize, id: String, sk: String, pwd: String);
    async fn upload_pwd_guess(&self, num_nodes: usize, id: String, pwd_guess: String);
    async fn aggregate_sk(&self, num_nodes: usize, id: String) -> Vec<u8>;
}

#[async_trait]
impl SecretKeyRecoverable for Client {
    // TODO I think we have to use
    async fn upload_sk_and_pwd(&self, num_nodes: usize, id: String, sk: String, pwd: String) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let sk = SecretKey::from(sk);
        let nzs = sk.to_nonzero_scalar();
        // 32 for field size, 1 for identifier = 33
        let res = Shamir::<2, 4>::split_secret::<Scalar, ChaCha20Rng, 33>(*nzs.as_ref(), &mut rng)?;
        self.upload_blob(id.to_owned() + "sk.txt", res.map(|x| x.value()))
            .await;

        let pwd = Scalar::from(pwd);
        let pwd_shares = Shamir::<2, 4>::split_secret::<Scalar, ChaCha20Rng, 33>(*pwd.as_ref(), &mut rng)?;
        Shamir::<2,4>::
        self.upload_blob(id + "pwd.txt", pwd_shares.map(|x| x.value())).await;
    }

    async fn upload_pwd_guess(&self, num_nodes: usize, id: String, pwd_guess: String) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let pwd_guess = Scalar::from(pwd_guess);
        let pwd_guess_shares = Shamir::<2, 4>::split_secret::<Scalar, ChaCha20Rng, 33>(*pwd_guess.as_ref(), &mut rng)?;
        self.upload_blob(id + "guess.txt", pwd_guess_shares.map(|x| x.value())).await;
    }
    async fn aggregate_sk(&self, num_nodes: usize, id: String) -> Vec<u8> {
        let sk_shares = self.retrieve_blob(id + "recovered_sk.txt").await
            .map(|x| Shamir::Share{x});

        let res = Shamir::<2, 4>::combine_shares::<Scalar, 33>(&sk_shares);
        assert!(res.is_ok());
        let scalar = res.unwrap();
        let nzs_dup =  NonZeroScalar::from_repr(scalar.to_repr()).unwrap();
        let sk = SecretKey::from(nzs_dup);
        sk.to_be_bytes();
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

            let f = SecretKey::from_be_bytes(s.as_slice());

            println!("Recovered sk: {}", f);
        }

        _ => println!("Missing/wrong arguments"),
    };
    Ok(())
}
