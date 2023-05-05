use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::error::Error;
use std::fs::File;
use std::io::Error as IoError;
use std::io::prelude::*;

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

            File::create(format!("{}sk.txt", &user_id))?.write_all(sk_shares)?;
            File::create(format!("{}pwd.txt", &user_id))?.write_all(pwd_share)?;
            File::create(format!("{}skhash.txt", &user_id))?.write_all(skhash)?;
            File::create(format!("{}salt.txt", &user_id))?.write_all(salt)?;

            Ok(())
        },
        "skrecovery" => {
            let user_id = String::from_utf8(args[0].clone())
                .expect("User ID is not UTF-8 encoded");

            // compute R(PW-PWG) share locally
            
            let mut buf = Vec::new();
            File::open(format!("{}sk.txt", user_id))?.read_to_end(&mut buf)?;
            let sk_shares: Vec<Share<33>> = serde_json::from_slice(&buf)?;

            let mut buf = Vec::new();
            File::open(format!("{}pwd.txt", user_id))?.read_to_end(&mut buf)?;
            let pwd_share: Scalar = Share::<33>::try_from(buf.as_slice())?.as_field_element().unwrap();
            let pwd_guess_share: Scalar = Share::<33>::try_from(args[1].as_ref())?.as_field_element().unwrap();

            // Thanks Emma for showing us this neat trick!
            // https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=96317e8e38cc956da308026e5328948ebd9d49ad

            let a_size = NUM_SERVERS - THRESHOLD;
            let my_as = generate_a(num_parties, a_size, rank);
            let r_a = (0..NUM_A).map(|i| {
                let mut buf = Vec::new();
                let mut prg_file = File::open(format!("{}_prg.hex", i))?;
                prg_file.read_to_end(&mut buf)?;
                let curr_seed = u64::from_le_bytes([0u8; 8]); // CHANGE
                let rng = ChaCha20Rng::seed_from_u64(curr_seed);
                // let next_seed = rng.gen::<u64>();
                // let mut prg_out_file = &out_files[i];
                // println!("new seed len {}", next_seed.to_le_bytes().len());
                // if prg_out_file.write_all(&next_seed.to_le_bytes()).is_err() {
                //     panic!("Error writing new prg seed {}", i)
                // }
                // assert!(prg_out_file.flush().is_ok());
                Ok(Scalar::random(rng))
            }).collect::<Result<Vec<Scalar>, IoError>>()?;

            let f_a = my_as.iter().map(|a| {
                let mut fa_j = Scalar::one();
                let factors = n_sub_a(num_parties, a.to_vec());
                for f in factors {
                    fa_j *= Scalar::from_uint_reduced(U256::from(rank as u32)) - Scalar::from_uint_reduced(U256::from(f as u8));
                }
                fa_j + Scalar::one()
            });
            let my_share = f_a.zip(r_a).fold(Scalar::zero(), |prev, f_and_r| prev + f_and_r.0 * f_and_r.1); 

            // TODO check that all ids are the same maybe this isn't necessary

            //let random_scalar = Scalar::random(rng);
            let mut result_vec = Vec::new();
            for sk_share in sk_shares {
                let id = sk_share.identifier();
                let share: Scalar = sk_share.as_field_element().unwrap();
                let field_to_write = (pwd_share - pwd_guess_share) * my_share + share;
                let mut result = vec![id];
                result.extend(field_to_write.to_bytes());
                result_vec.push(result);
            }

            let salt = {
                let mut buf = vec![];
                File::open(format!("{}salt.txt", user_id))?.read_to_end(&mut buf)?;
                buf
            };
            let skhash = {
                let mut buf = vec![];
                File::open(format!("{}skhash.txt", user_id))?.read_to_end(&mut buf)?;
                buf
            };

            let result_vec_to_output = serde_json::to_vec(&(result_vec, salt, skhash)).unwrap();
            req.output(&result_vec_to_output)?;

            Ok(())
        }
        "seed_prgs" => {
            let a_size = NUM_SERVERS - THRESHOLD;

            for (i, v) in generate_a(num_parties, a_size, rank as usize).iter().enumerate() {
                let sender = 0; // I think this also works bc the set elements are all in increasing order 
                println!("{} {:?}, {}", rank, v, sender);
                let mut out_file = File::create(format!("{}_prg.hex", i))?;
                let my_prg_seed = (v[0] * 256 + v[1] * 16 + v[2]) as u64; // change later?

                println!("{}", my_prg_seed.to_le_bytes().len());

                if v[sender] == rank as usize {
                    out_file.write_all(&my_prg_seed.to_le_bytes())?;
                    for j in 0..a_size {
                        if v[j] != rank as usize {
                            req.msg_send(&my_prg_seed.to_le_bytes(), v[j], 0)?;
                        }
                    }
                } else {
                    let mut buf = [0u8; 8];
                    req.msg_recv(&mut buf, v[sender], 0)?;
                    out_file.write_all(&buf)?;
                }
            }

            Ok(())
        }
        _ => panic!(),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let env = libdots::env::init()?;

    loop {
        let req = libdots::request::accept()?;
        handle_request(&env, &req)?;
    }
}
