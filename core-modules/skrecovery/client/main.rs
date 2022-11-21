use std::env;

use dtrust::client::Client;
use async_trait::async_trait;
use rand::{RngCore};
use rand_chacha::ChaCha20Rng;
use rand::prelude::*;

use ark_ff::UniformRand;
use ark_ff::fields::Field;

use ark_bls12_381::{Fr};
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
    async fn upload_pwd_guess<Cfg: SkConfig>(&self, id: String, pwd_guess: String);
}

fn shard<F: Field>(n: F, num_shards: usize, rng: &mut impl RngCore) -> Vec<F>{
    // Initialize random number array, sum
    let random_vals = (0..num_shards).map(|_| F::rand(rng)).collect::<Vec<F>>();
    let sum = random_vals.iter().sum();
    // Find the inverse of sum
    let sum_inv = match F::inverse(&sum) {
        Some(s) => s,
        None => panic!("some random numbers summed to zero, go buy a lottery ticket")
    };
    // Multiple all n random numbers by sk * sum^-1
    let shards = random_vals.iter().map(|x| *x * sum_inv * n).collect::<Vec<F>>();
    // Return shards
    shards
}

/// There's a more rust-y way to do implement these conversions - use the From trait
fn to_bytes<F: Field>(n: &F) -> Vec<u8> {
    let mut v = Vec::new();
    assert!(n.serialize_uncompressed(&mut v).is_ok());
    v
}

// fn from_string<F: Field>(s: String) -> F {
//     match F::deserialize_uncompressed(s.as_bytes()) {
//         Ok(f) => f,
//         Err(_) => {
//             eprintln!("error desrerializing field element");
//             panic!("");
//         },
//     }
// }

#[async_trait]
impl SecretKeyRecoverable for Client
{
    async fn upload_sk_and_pwd<Cfg: SkConfig>(&self, id: String, _sk_str: String, _pwd_str: String) {
        let rng = &mut ChaCha20Rng::from_entropy();
        let sk_field = <Cfg::F>::rand(rng); //from_string::<Cfg::F>(sk_str);
        println!("sk_field: {}", sk_field);
        let sk_shards = shard::<Cfg::F>(sk_field, 2, rng);
        let sk_shards_bytes = sk_shards.iter().map(to_bytes::<Cfg::F>)
            .collect::<Vec<_>>();
        let sk_fname = id.to_owned() + "sk";
        // maybe this naming scheme isn't secure ...
        self.upload_blob(sk_fname, sk_shards_bytes).await;
        let pwd_field = <Cfg::F>::from_random_bytes("is this secure lmao".as_bytes()).unwrap();
        println!("pwd_field: {}", pwd_field);
        let pwd_shards = shard::<Cfg::F>(pwd_field, 2, rng);
        let pwd_shards_bytes = pwd_shards.iter().map(to_bytes::<Cfg::F>).collect::<Vec<_>>();
        self.upload_blob(id + "pwd", pwd_shards_bytes).await;
    }

    async fn upload_pwd_guess<Cfg: SkConfig>(&self, _id: String, _pwd_guess: String) {
        todo!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];

    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];

    let cli_id = "user1"; // TODO cli_id should be inputted.
    let mut client = Client::new(cli_id);

    let app_name = "rust_app";

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
            client.upload_pwd_guess::<MyConfig>(String::from(&id), String::from(&pwd_guess)).await;

            println!("Recovering sk with pwd guess {}, for user {}", pwd_guess, id);
            
            let in_files = [String::from(id.to_owned() + "sk"),
                String::from(id.to_owned() + "pwd"), 
                String::from(id.to_owned() + "pwdguess")];

            client
                .exec(app_name, "skrecovery", in_files.to_vec(), Vec::new())
                .await?;
            
        }

        _ => println!("Missing/wrong arguments")
    };
    Ok(())
}