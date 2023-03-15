use std::env;

use async_trait::async_trait;
use blake2::{Blake2s256, Digest};
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
#[async_trait]
pub trait SecretKeyRecoverable {
    // Encrypt the secret key?
    // TODO get rid of num_nodes. I think threshold / num servers should be either read from somewhere ( serverconf.yaml? )
    // or defined in some `constants.rs`.
    async fn upload_sk_and_pwd(&self, id: String, sk: String, pwd: String, hasher: &mut Blake2s256);
    async fn upload_pwd_guess(&self, id: String, pwd_guess: String, hasher: &mut Blake2s256);
    async fn aggregate_sk(&self, id: String) -> Vec<u8>;
}

#[async_trait]
impl SecretKeyRecoverable for Client {
    async fn upload_sk_and_pwd(&self, id: String, sk: String, pwd: String, hasher: &mut Blake2s256) {
        // TODO BIG: generate field elements from plaintext.
        let rng = &mut ChaCha20Rng::from_entropy();
        let sk_str = "AD302A6F48F74DD6F9D257F7149E4D06CD8936FE200AF67E08EF88D1CBA4525D";
        let nzs = NonZeroScalar::from_str(&sk_str).unwrap();

        // 32 for field size, 1 for identifier = 33
        let res = Shamir::<THRESHOLD, NUM_SERVERS>::split_secret::<Scalar, ChaCha20Rng, 33>(
            *nzs.as_ref(),
            rng,
        )
        .unwrap();

        self.upload_blob(
            id.to_owned() + "sk.txt",
            res.map(|x| x.as_ref().to_vec()).to_vec(),
        )
        .await;

        let pwd_nzs = util::string_hash_to_nzs(&pwd, hasher);
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
        /*let salt = rng.gen::<[u8; 32]>();
        let mut hasher = Blake2b512::new();
        hasher.update(salt);
        hasher.update(nzs.to_bytes());
        let res = hasher.finalize().to_vec();
        self.upload_blob(id.to_owned() + "skhash.txt", vec![res; NUM_SERVERS])
            .await;
        self.upload_blob(id.to_owned() + "salt.txt", vec![salt.to_vec(); NUM_SERVERS])
            .await; */
    }

    async fn upload_pwd_guess(&self, id: String, pwd_guess: String, hasher: &mut Blake2s256) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let pwd_guess_nzs = util::string_hash_to_nzs(&pwd_guess, hasher);
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
        sk_byte_shares.for_each(|x| sk_shares.push(Share::try_from(x).unwrap()));
        // get back (2t, n) shares bc of multiplication
        const RECOVER_THRESHOLD: usize = THRESHOLD;
        let res = Shamir::<4, NUM_SERVERS>::combine_shares::<Scalar, 33>(&sk_shares);
        assert!(res.is_ok());
        let scalar = res.unwrap();
        let sk = NonZeroScalar::from_repr(scalar.to_repr()).unwrap();

        // let salts = self.retrieve_blob(id.to_owned() + "salt.txt").await;
        // let hashes = self.retrieve_blob(id.to_owned() + "skhash.txt").await;
        // // TODO Check that all salts and hashes are the same

        // let mut hasher = Blake2b512::new();
        // hasher.update(&salts[0]);
        // hasher.update(&sk.to_bytes());
        // let hash_result = hasher.finalize();
        // assert_eq!(hashes[0], hash_result.to_vec());
        sk.to_bytes().to_vec()
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

    // hash function to convert password string to 256-bit array, used to convert to field element
    let mut string_hasher = Blake2s256::new();

    match &cmd[..] {
        "seed_prgs" => {
            let in_files = [];

            let out_files = (0..6) // TODO: should be 4 choose 2
                .map(|x: u8| format!("{}_prg.hex", x))
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
            client.upload_sk_and_pwd(id, sk, pwd, &mut string_hasher).await;
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
                .upload_pwd_guess(String::from(&id), pwd_guess.clone(), &mut string_hasher)
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

            let f = SecretKey::from_be_bytes(s.as_slice()).unwrap();

            println!("Recovered sk: {}", f.to_nonzero_scalar());
        }

        _ => println!("Missing/wrong arguments"),
    };
    Ok(())
}
