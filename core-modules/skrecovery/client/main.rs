use std::env;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use blake2::{Blake2b512, Digest};
use dtrust::client::Client;
use p256::Scalar;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use vsss_rs::{Shamir, Share};

#[path = "../util.rs"]
mod util;
use util::*;
#[async_trait]
pub trait SecretKeyRecoverable {
    // Encrypt the secret key?
    // TODO get rid of num_nodes. I think threshold / num servers should be either read from somewhere ( serverconf.yaml? )
    // or defined in some `constants.rs`.
    async fn upload_sk_and_pwd(&self, id: String, sk: String, pwd: String);
    async fn upload_pwd_guess(&self, id: String, pwd_guess: String);
    async fn aggregate_sk(&self, id: String) -> Vec<u8>;
}

#[async_trait]
impl SecretKeyRecoverable for Client {
    async fn upload_sk_and_pwd(&self, id: String, sk: String, pwd: String) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let field_elts = sk_to_field_elts(&sk);
        let mut shares_vec = Vec::new();
        for _ in 0..NUM_SERVERS {
            shares_vec.push(Vec::new());
        }
        for nzs in field_elts.as_slice() {
            // 32 for field size, 1 for identifier = 33
            let res = Shamir::<THRESHOLD, NUM_SERVERS>::split_secret::<Scalar, ChaCha20Rng, 33>(
                *nzs.as_ref(),
                rng,
            )
            .unwrap();
            for i in 0..NUM_SERVERS {
                shares_vec[i].push(res[i]);
            }
        }
        let mut upload_vec = Vec::new();
        for i in 0..NUM_SERVERS {
            upload_vec.push(serde_json::to_vec(&shares_vec[i]).unwrap());
        }

        self.upload_blob(
            id.to_owned() + "sk.txt",
            upload_vec,
        )
        .await;

        let pwd_nzs = string_hash_to_nzs(&pwd);
        let pwd_shares = Shamir::<THRESHOLD, NUM_SERVERS>::split_secret::<Scalar, ChaCha20Rng, 33>(
            *pwd_nzs.as_ref(),
            rng,
        )
        .unwrap();
        self.upload_blob(
            id.to_owned() + "pwd.txt",
            pwd_shares.map(|x| x.as_ref().to_vec()).to_vec(),
        )
        .await;

        // TODO: you wanna compress these into the same file? maybe take a look at serde, or maybe that's not necessary.
        // idgaf it's pretty inconsequential.
        let salt = rng.gen::<[u8; 32]>();
        let mut hasher = Blake2b512::new();
        hasher.update(salt);
        for nzs in field_elts.as_slice() {
            hasher.update(nzs.to_bytes());
        }
        let res = hasher.finalize().to_vec();
        self.upload_blob(id.to_owned() + "skhash.txt", vec![res; NUM_SERVERS])
            .await;
        self.upload_blob(id.to_owned() + "salt.txt", vec![salt.to_vec(); NUM_SERVERS])
            .await;
    }

    async fn upload_pwd_guess(&self, id: String, pwd_guess: String) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let pwd_guess_nzs = string_hash_to_nzs(&pwd_guess);
        let pwd_guess_shares = Shamir::<{ THRESHOLD }, NUM_SERVERS>::split_secret::<
            Scalar,
            ChaCha20Rng,
            33,
        >(*pwd_guess_nzs.as_ref(), rng)
        .unwrap();
        self.upload_blob(
            id.to_owned() + "guess.txt",
            pwd_guess_shares.map(|x| x.as_ref().to_vec()).to_vec(),
        )
        .await;
    }
    async fn aggregate_sk(&self, id: String) -> Vec<u8> {
        let sk_byte_shares = self.retrieve_blob(id.to_owned() + "recovered_sk.txt").await;
        let sk_byte_shares = sk_byte_shares.iter().map(|x| x.as_slice());
        let mut sk_shares = Vec::new();
        sk_byte_shares.for_each(|x| {
            let shares_vec: Vec<Vec<u8>> = serde_json::from_slice(x).unwrap();
            sk_shares.push(shares_vec);
        });
        // get back (2t, n) shares bc of multiplication
        const RECOVER_THRESHOLD: usize = THRESHOLD;
        let num_chunks = sk_shares[0].len();
        let mut sk_scalars = Vec::new();
        for i in 0..num_chunks {
            let mut scalars = Vec::new();
            for vec in sk_shares.as_slice() {
                scalars.push(Share::try_from(vec[i].as_slice()).unwrap());
            }
            let res = Shamir::<4, NUM_SERVERS>::combine_shares::<Scalar, 33>(&scalars);
            assert!(res.is_ok());
            let sk_scalar = res.unwrap();
            sk_scalars.push(sk_scalar);
        }

        let salts = self.retrieve_blob(id.to_owned() + "salt.txt").await;
        let hashes = self.retrieve_blob(id.to_owned() + "skhash.txt").await;
        //Check that all salts and hashes are the same

        if verify_sk_hash(salts, hashes, sk_scalars.as_slice()) {
            field_elts_to_string(sk_scalars.as_slice()).into_bytes().to_vec()
        }
        else {
            Vec::new()
        }
        
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
        "http://127.0.0.1:50055",
    ];

    let cli_id = "user1";
    let mut client = Client::new(cli_id);

    let app_name = "rust_app";

    client.setup(node_addrs.to_vec(), None);

    match &cmd[..] {
        "seed_prgs" => {
            let in_files = [];

            let out_files = (0..NUM_A) // TODO: should be 4 choose 2
                .map(|x| format!("{}_prg.hex", x))
                .collect::<Vec<_>>();

            client
                .exec(app_name, "seed_prgs", in_files.to_vec(), out_files.to_vec())
                .await?;
        }
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
            client.upload_sk_and_pwd(id, sk, pwd).await;
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
                .upload_pwd_guess(String::from(&id), pwd_guess.clone())
                .await;
            println!("Guess uploaded");

            println!(
                "Recovering sk with pwd guess {}, for user {}",
                pwd_guess, id
            );

            let in_files = [
                (0..NUM_A) // TODO: should be 4 choose 2
                    .map(|x| format!("{}_prg.hex", x))
                    .collect::<Vec<_>>(),
                [
                    id.to_owned() + "sk.txt",
                    id.to_owned() + "pwd.txt",
                    id.to_owned() + "guess.txt",
                ]
                .into(),
            ]
            .concat();

            println!("{:?}", in_files);
            // let out_files = [
            //     (0..NUM_A) // TODO: should be 4 choose 2
            //         .map(|x| format!("{}_prg.hex", x))
            //         .collect::<Vec<_>>(),
            //     [
            //         id.to_owned() + "recovered_sk.txt"
            //     ]
            //     .into(),
            // ]
            // .concat();
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
            let s = client.aggregate_sk(id).await;
            if s.is_empty() {
                println!("Recovered sk incorrect!");
            } else {
                let sk_string = String::from_utf8(s).unwrap();
                println!("Recovered sk: {}", sk_string);
            }
        }

        _ => println!("Missing/wrong arguments"),
    };
    Ok(())
}
