use std::env;

use dtrust::client::Client;
use async_trait::async_trait;
use rand::{thread_rng, RngCore};

use ark_ff::UniformRand;
use ark_ff::fields::Field;
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_bls12_381::{Bls12_381, Fr};
pub trait SkConfig {
    type F: Field;
}

pub struct MyConfig {
    num: u8
}

impl SkConfig for MyConfig {
    type F = Fr;
}
#[async_trait]
pub trait SecretKeyRecoverable {
    // Encrypt the secret key?
    async fn upload_sk_and_pwd<Cfg: SkConfig>(&self, id: String, sk: String, pwd: String);
    async fn recover_sk<Cfg: SkConfig>(&self, id: String, pwd_guess: String) -> String;
}

fn shard<F: Field>(n: F, num_shards: usize, rng: &mut impl RngCore) -> Vec<F>{
    // Initialize random number array, sum
    let random_vals = (0..num_shards).map(|_| F::rand(rng)).collect::<Vec<F>>();
    let sum = random_vals.iter().sum();
    // Find the inverse of sum
    let sum_inv = match F::inverse(&sum) {
        Some(s) => s,
        None => panic!("some random numbers summed to zero L")
    };
    // Multiple all n random numbers by sk * sum^-1
    let shards = random_vals.iter().map(|x| *x * sum_inv * n).collect::<Vec<F>>();
    // Return shards
    shards
}

/// There's a more rust-y way to do implement these conversions - use the From trait
fn to_bytes<F: Field>(n: &F) -> Vec<u8> {
    let v = Vec::new();
    assert!(n.serialize_uncompressed(&mut v).is_ok());
    v
}

fn from_string<F: Field>(s: String) -> F {
    F::deserialize_uncompressed(s.as_bytes()).unwrap()
}

#[async_trait]
impl SecretKeyRecoverable for Client
{
    async fn upload_sk_and_pwd<Cfg: SkConfig>(&self, id: String, sk_str: String, pwd_str: String) {
        let rng = &mut thread_rng();
        let sk_field = from_string::<Cfg::F>(sk_str);
        let sk_shards = shard::<Cfg::F>(sk_field, 2, rng);
        let sk_shards_bytes = sk_shards.iter().map(to_bytes::<Cfg::F>)
            .collect::<Vec<_>>();
        let sk_fname = id.to_owned() + "sk";
        // maybe this naming scheme isn't secure ...
        self.upload_blob(sk_fname, sk_shards_bytes).await
        // let pwd_field = from_string::<Cfg::F>(pwd_str);
        // let pwd_shards = shard::<Cfg::F>(pwd_field, 2, rng);
        // let pwd_shards_bytes = pwd_shards.iter().map(to_bytes::<Cfg::F>).collect::<Vec<_>>();
        // // maybe this naming scheme isn't secure ...
        // self.upload_blob(id + "pwd", pwd_shards_bytes);
    }

    async fn recover_sk<Cfg: SkConfig>(&self, id: String, pwd_guess: String) -> String {
        todo!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];

    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];

    let cli_id = "user1";
    let mut client = Client::new(cli_id);

    client.setup(node_addrs.to_vec());

    match &cmd[..]{
        "upload_sk_and_pwd" => {
            let id: String = match args[2].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: user-id not a string");
                    panic!("");
                },
            };
            let sk: String = match args[3].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: sk not a string");
                    panic!("");
                },
            };
            let pwd: String = match args[4].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: pwd not a string");
                    panic!("");
                },
            };
            println!("Uploading sk {}, pwd {} for user {}", sk, pwd, id);
            client.upload_sk_and_pwd::<MyConfig>(String::from(id), sk, pwd).await;
        }
        "recover_sk" => {
            let id: String = match args[2].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: user-id not a string");
                    panic!("");
                },
            };
            let pwd_guess: String = match args[3].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: pwd guess not a string");
                    panic!("");
                },
            };
            println!("Recovering sk with pwd guess {}, for user {}", pwd_guess, id);
            client.recover_sk::<MyConfig>(String::from(id), pwd_guess).await;
        }

        _ => println!("Missing/wrong arguments")
    };
    Ok(())
}