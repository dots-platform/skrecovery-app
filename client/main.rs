// Copyright 2023 The Dots Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env;
use std::error::Error;
use std::iter;

use blake2::{Blake2b512, Digest};
use dotspb::dec_exec::dec_exec_client::DecExecClient;
use futures::future;
use p256::Scalar;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use tonic::transport::Channel;
use tonic::Request;
use uuid::Uuid;
use vsss_rs::{Shamir, Share};

#[path = "../util.rs"]
mod util;
use util::*;

const APP_NAME: &str = "skrecovery";

fn uuid_to_uuidpb(id: Uuid) -> dotspb::dec_exec::Uuid {
    dotspb::dec_exec::Uuid {
        hi: (id.as_u128() >> 64) as u64,
        lo: id.as_u128() as u64,
    }
}

async fn seed_prgs(clients: &mut [DecExecClient<Channel>]) -> Result<(), Box<dyn Error>> {
    let request_id = Uuid::new_v4();
    future::join_all(
            clients.iter_mut()
                .map(|client|
                    client.exec(Request::new(dotspb::dec_exec::App {
                        app_name: APP_NAME.to_owned(),
                        app_uid: 0,
                        request_id: Some(uuid_to_uuidpb(request_id)),
                        client_id: "".to_owned(),
                        func_name: "seed_prgs".to_owned(),
                        in_files: vec![],
                        out_files: vec![],
                        args: vec![],
                    }))
                )
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    Ok(())
}

async fn upload_sk_and_pwd(clients: &mut [DecExecClient<Channel>], id: &str, sk: &str, pwd: &str) -> Result<(), Box<dyn Error>> {
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

    let request_id = Uuid::new_v4();
    future::join_all(
            iter::zip(clients, iter::zip(sk_shares, pwd_shares))
                .map(|(client, (sk_share, pwd_share))|
                    client.exec(Request::new(dotspb::dec_exec::App {
                        app_name: APP_NAME.to_owned(),
                        app_uid: 0,
                        request_id: Some(uuid_to_uuidpb(request_id)),
                        client_id: "".to_owned(),
                        func_name: "upload_sk_and_pwd".to_owned(),
                        in_files: vec![],
                        out_files: vec![],
                        args: vec![id.as_bytes().to_owned(), sk_share, pwd_share, salt.to_vec(), hash.clone()],
                    }))
                )
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

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

async fn recover_sk(clients: &mut [DecExecClient<Channel>], id: &str, pwd_guess: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let pwd_guess_shares = compute_pwd_guess(pwd_guess);

    let request_id = Uuid::new_v4();
    let res = future::join_all(
            iter::zip(clients, pwd_guess_shares)
                .map(|(client, pwd_guess_share)|
                    client.exec(Request::new(dotspb::dec_exec::App {
                        app_name: APP_NAME.to_owned(),
                        app_uid: 0,
                        request_id: Some(uuid_to_uuidpb(request_id)),
                        client_id: "".to_owned(),
                        func_name: "skrecovery".to_owned(),
                        in_files: vec![],
                        out_files: vec![],
                        args: vec![id.as_bytes().to_owned(), pwd_guess_share],
                    }))
                )
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|res| res.into_inner())
        .collect::<Vec<_>>();
    let outputs: Vec<&[u8]> = res
        .iter()
        .map(|res| res.output.as_slice())
        .collect();

    let s = aggregate_sk(&outputs);

    Ok(s)
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

    let mut clients = future::join_all(
            node_addrs
                .iter()
                .map(|addr| DecExecClient::connect(addr.clone()))
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    match &cmd[..] {
        "seed_prgs" => {
            seed_prgs(&mut clients).await?;
        }
        "upload_sk_and_pwd" => {
            let id = &args[2];
            let sk = &args[3];
            let pwd = &args[4];
            println!("Uploading sk {}, pwd {} for user {}", sk, pwd, id);
            upload_sk_and_pwd(&mut clients, id, sk, pwd).await?;
        }
        "recover_sk" => {
            let id = &args[2];
            let pwd_guess = &args[3];

            println!(
                "Recovering sk with pwd guess {}, for user {}",
                pwd_guess, id
            );

            let s = recover_sk(&mut clients, id, pwd_guess).await?;

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
