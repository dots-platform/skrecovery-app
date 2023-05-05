use std::env;
use std::error::Error;
use std::iter;

use blake2::{Blake2b512, Digest};
use p256::Scalar;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use vsss_rs::{Shamir, Share};

mod client;
use client::Client;

#[path = "../util.rs"]
mod util;
use util::*;

const APP_NAME: &str = "skrecovery";

async fn upload_sk_and_pwd(client: &mut Client, id: &str, sk: &str, pwd: &str) -> Result<(), Box<dyn Error>> {
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
    let sk_shares: Vec<Vec<u8>> = shares_vec
        .iter()
        .map(|share| serde_json::to_vec(share).unwrap())
        .collect();

    let pwd_nzs = string_hash_to_nzs(&pwd);
    let pwd_shares: Vec<Vec<u8>> = Shamir::<THRESHOLD, NUM_SERVERS>::split_secret::<Scalar, ChaCha20Rng, 33>(
        *pwd_nzs.as_ref(),
        rng,
    )
        .unwrap()
        .iter()
        .map(|share| share.as_ref().to_vec())
        .collect();

    // TODO: you wanna compress these into the same file? maybe take a look at serde, or maybe that's not necessary.
    // idgaf it's pretty inconsequential.
    let salt = rng.gen::<[u8; 32]>();
    let mut hasher = Blake2b512::new();
    hasher.update(salt);
    for nzs in field_elts.as_slice() {
        hasher.update(nzs.to_bytes());
    }
    let hash = hasher.finalize().to_vec();

    client.exec(APP_NAME,
                "upload_sk_and_pwd",
                vec![],
                vec![],
                iter::zip(sk_shares, pwd_shares)
                    .map(|(sk_share, pwd_share)| vec![id.as_bytes().to_owned(), sk_share, pwd_share, salt.to_vec(), hash.clone()])
                    .collect())
        .await?;

    Ok(())
}

fn compute_pwd_guess(pwd_guess: &str) -> Vec<Vec<u8>> {
    let rng = &mut ChaCha20Rng::from_entropy();
    let pwd_guess_nzs = string_hash_to_nzs(&pwd_guess);
    let pwd_guess_shares = Shamir::<{ THRESHOLD }, NUM_SERVERS>::split_secret::<
        Scalar,
        ChaCha20Rng,
        33,
    >(*pwd_guess_nzs.as_ref(), rng)
    .unwrap();
    pwd_guess_shares.map(|x| x.as_ref().to_vec()).to_vec()
}

fn aggregate_sk(outputs: &[&[u8]]) -> Vec<u8> {
    let deserialized: Vec<(Vec<Vec<u8>>, Vec<u8>, Vec<u8>)> = outputs
        .iter()
        .map(|x| serde_json::from_slice::<(Vec<Vec<u8>>, Vec<u8>, Vec<u8>)>(x).unwrap())
        .collect();
    let sk_shares: Vec<&[Vec<u8>]> = deserialized.iter().map(|x| x.0.as_slice()).collect();
    let salts: Vec<&[u8]> = deserialized.iter().map(|x| x.1.as_slice()).collect();
    let hashes: Vec<&[u8]> = deserialized.iter().map(|x| x.2.as_slice()).collect();
    // get back (2t, n) shares bc of multiplication
    const RECOVER_THRESHOLD: usize = THRESHOLD*2;
    let num_chunks = sk_shares[0].len();
    let mut sk_scalars = Vec::new();
    for i in 0..num_chunks {
        let mut scalars = Vec::new();
        for vec in sk_shares.as_slice() {
            scalars.push(Share::try_from(vec[i].as_slice()).unwrap());
        }
        let res = Shamir::<RECOVER_THRESHOLD, NUM_SERVERS>::combine_shares::<Scalar, 33>(&scalars);
        assert!(res.is_ok());
        let sk_scalar = res.unwrap();
        sk_scalars.push(sk_scalar);
    }

    //Check that all salts and hashes are the same

    if verify_sk_hash(&salts, &hashes, sk_scalars.as_slice()) {
        field_elts_to_string(sk_scalars.as_slice()).into_bytes().to_vec()
    }
    else {
        Vec::new()
    }
    
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];
    let node_addrs = [
        "http://127.0.0.1:50050",
        "http://127.0.0.1:50051",
        "http://127.0.0.1:50052",
        "http://127.0.0.1:50053",
        "http://127.0.0.1:50054",
    ];

    let cli_id = "";
    let mut client = Client::new(cli_id);

    client.setup(node_addrs.to_vec(), None);

    match &cmd[..] {
        "seed_prgs" => {
            client
                .exec(APP_NAME, "seed_prgs", vec![], vec![], vec![vec![]; NUM_A])
                .await?;
        }
        "upload_sk_and_pwd" => {
            let id = &args[2];
            let sk = &args[3];
            let pwd = &args[4];
            println!("Uploading sk {}, pwd {} for user {}", sk, pwd, id);
            upload_sk_and_pwd(&mut client, id, sk, pwd).await?;
        }
        "recover_sk" => {
            let id = &args[2];
            let pwd_guess = &args[3];

            println!(
                "Recovering sk with pwd guess {}, for user {}",
                pwd_guess, id
            );

            let pwd_guess_shares = compute_pwd_guess(pwd_guess);

            let responses = client
                .exec(
                    APP_NAME,
                    "skrecovery",
                    vec![],
                    vec![],
                    pwd_guess_shares
                        .into_iter()
                        .map(|pwd_guess_share| vec![id.as_bytes().to_owned(), pwd_guess_share])
                        .collect()
                )
                .await?;
            let outputs: Vec<&[u8]> = responses
                .iter()
                .map(|res| res.output.as_slice())
                .collect();

            println!("Aggregating SK on client");
            let s = aggregate_sk(&outputs);
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
