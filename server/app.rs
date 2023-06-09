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

use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::error::Error;
use std::fs;
use std::io::Error as IoError;
use std::thread;

use libdots::env::Env;
use libdots::request::Request;

#[path = "../util.rs"]
mod util;
use util::{NUM_SERVERS, THRESHOLD, NUM_A};

use elliptic_curve::{Field, ops::Reduce};
use p256::{Scalar, U256};
use vsss_rs::Share;

fn generate_a(num_parties: usize, a_size: usize, rank: usize) -> Vec<Vec<usize>> {
    let other_parties = (0..num_parties - 1)
    .map(|x| if x < rank as usize { x } else { x + 1 })
    .collect::<Vec<usize>>();

    let mut result = Vec::new();
    for mut v in other_parties.iter().combinations(a_size - 1){
        let r = rank as usize;
        for j in 0..a_size - 1 {
            if *v[j] > rank as usize {
                v.insert(j, &r);
                break;
            }
        }
        if v.len() != a_size {
            v.push(&r);
        }
        result.push(v.iter().map(|x| **x).collect::<Vec<_>>());
    }
    result
}

fn n_sub_a(n: usize, a: Vec<usize>) -> Vec<usize> {
    let mut result = (0..n).collect::<Vec<_>>();
    for i in a.iter().rev() {
        result.remove(*i);
    }
    result
}

#[test]
fn test_n_sub_a() {
    let n = 6;
    let a = vec![0, 4, 5];
    assert_eq!(n_sub_a(n, a), vec![1,2,3]);
}

fn handle_request(env: &Env, req: &Request) -> Result<(), Box<dyn Error>> {
    let rank = env.get_world_rank();
    let num_parties = env.get_world_size();
    let func_name = &req.func_name;
    let args = &req.args;

    println!("rank {} starting", rank);

    match &func_name[..] {
        "upload_sk_and_pwd" => {
            let user_id = String::from_utf8(args[0].clone())
                .expect("User ID is not UTF-8 encoded");
            let sk_shares = &args[1];
            let pwd_share = &args[2];
            let salt = &args[3];
            let skhash = &args[4];

            fs::write(format!("{}sk.txt", &user_id), sk_shares)?;
            fs::write(format!("{}pwd.txt", &user_id), pwd_share)?;
            fs::write(format!("{}skhash.txt", &user_id), skhash)?;
            fs::write(format!("{}salt.txt", &user_id), salt)?;

            Ok(())
        },
        "skrecovery" => {
            let user_id = String::from_utf8(args[0].clone())
                .expect("User ID is not UTF-8 encoded");

            // compute R(PW-PWG) share locally
            
            let sk_shares_data = fs::read(format!("{}sk.txt", user_id))?;
            let sk_shares: Vec<Share<33>> = serde_json::from_slice(&sk_shares_data)?;

            let pwd_share_data = fs::read(format!("{}pwd.txt", user_id))?;
            let pwd_share: Scalar = Share::<33>::try_from(pwd_share_data.as_slice())?.as_field_element().unwrap();
            let pwd_guess_share: Scalar = Share::<33>::try_from(args[1].as_ref())?.as_field_element().unwrap();

            // Thanks Emma for showing us this neat trick!
            // https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=96317e8e38cc956da308026e5328948ebd9d49ad

            let a_size = NUM_SERVERS - THRESHOLD;
            let my_as = generate_a(num_parties, a_size, rank);
            let r_a = (0..NUM_A).map(|i| {
                let prg_data = fs::read(format!("{}_prg.json", i))?;
                let prg_string = String::from_utf8(prg_data).unwrap();
                println!("{}", prg_string);
                let mut rng: ChaCha20Rng = serde_json::from_str(&prg_string)?;
                let r_a = Scalar::random(rng.clone());
                let _change_rng = rng.gen::<u64>(); // change prg state before storing again
                let serialized_rng = serde_json::to_string(&rng)?;
                fs::write(format!("{}_prg.json", i), serialized_rng.as_bytes())?;
                Ok(r_a)
            }).collect::<Result<Vec<Scalar>, IoError>>()?;

            let f_a = my_as.iter().map(|a| {
                let mut fa_j = Scalar::one();
                let factors = n_sub_a(num_parties, a.to_vec());
                for f in factors {
                    fa_j *= Scalar::from_uint_reduced(U256::from(rank as u32)) - Scalar::from_uint_reduced(U256::from(f as u8));
                }
                fa_j
            });
            let random_hiding = f_a.zip(r_a).fold(Scalar::zero(), |prev, f_and_r| prev + f_and_r.0 * f_and_r.1); 

            let mut result_vec = Vec::new();
            for sk_share in sk_shares {
                let id = sk_share.identifier();
                let share: Scalar = sk_share.as_field_element().unwrap();
                let field_to_write = (pwd_share - pwd_guess_share) * random_hiding + share;
                let mut result = vec![id];
                result.extend(field_to_write.to_bytes());
                result_vec.push(result);
            }

            let salt = fs::read(format!("{}salt.txt", user_id))?;
            let skhash = fs::read(format!("{}skhash.txt", user_id))?;

            let result_vec_to_output = serde_json::to_vec(&(result_vec, salt, skhash)).unwrap();
            req.output(&result_vec_to_output)?;

            Ok(())
        }
        "seed_prgs" => {
            let a_size = NUM_SERVERS - THRESHOLD;

            for (i, v) in generate_a(num_parties, a_size, rank as usize).iter().enumerate() {
                let sender = 0; // I think this also works bc the set elements are all in increasing order 
                println!("{} {:?}, {}", rank, v, sender);
                let rng = &mut ChaCha20Rng::from_entropy();
                let my_prg_seed = rng.gen::<u64>(); // change later?

                println!("{}", my_prg_seed.to_le_bytes().len());

                let prg_seed: u64;
                if v[sender] == rank as usize {
                    for j in 0..a_size {
                        if v[j] != rank as usize {
                            req.msg_send(&my_prg_seed.to_le_bytes(), v[j], 0)?;
                        }
                    }
                    prg_seed = my_prg_seed;
                } else {
                    let mut buf = [0u8; 8];
                    req.msg_recv(&mut buf, v[sender], 0)?;
                    prg_seed = u64::from_le_bytes(buf);
                }
                // Store rng state instead of seed in file, which updates after each recovery attempt
                let mut rng = ChaCha20Rng::seed_from_u64(prg_seed);
                let serialized_rng = serde_json::to_string(&rng)?;
                fs::write(format!("{}_prg.json", i), serialized_rng.as_bytes())?;
            }

            Ok(())
        }
        _ => panic!(),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let env = libdots::env::init()?;

    thread::scope(|s| -> Result<(), Box<dyn Error>> {
        loop {
            let env = &env;
            let req = libdots::request::accept()?;
            s.spawn(move || {
                handle_request(env, &req).unwrap();
            });
        }
    })?;

    Ok(())
}
