use dtrust::utils::init_app;
use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::io;
use std::io::prelude::*;

// TODO: move this field choice into a config somewhere. Also, put the number of nodes in this config
// Also the file names we use for client/server communication.
use std::net::TcpStream;

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
fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;

    println!("rank {} starting", rank);

    let num_parties = socks.len();

    match &func_name[..] {
        "skrecovery" => {
            // compute R(PW-PWG) share locally
            
            let shares_file = &in_files[NUM_A..];
            let mut sk_shares_file = &shares_file[0];
            let mut buf = Vec::new();
            if sk_shares_file.read_to_end(&mut buf).is_err() {
                panic!("Error reading file");
            }
            let sk_shares: Vec<Share<33>> = serde_json::from_slice(buf.as_slice()).unwrap();

            let pwd_shares = shares_file[1..]
                .iter()
                .map(|mut f| {
                    let mut buf = Vec::new();
                    if f.read_to_end(&mut buf).is_err() {
                        panic!("Error reading file");
                    }
                    let share = Share::<33>::try_from(buf.as_slice()).unwrap();
                    (share.identifier(), share.as_field_element().unwrap())
                })
                .collect::<Vec<(u8, Scalar)>>();
            
            let pwd_share = pwd_shares[0].1;
            let pwd_guess_share = pwd_shares[1].1;

            // Thanks Emma for showing us this neat trick!
            // https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=96317e8e38cc956da308026e5328948ebd9d49ad

            let a_size = NUM_SERVERS - THRESHOLD;
            let my_as = generate_a(num_parties, a_size, rank as usize);
            let r_a = (0..NUM_A).map(|i| {
                let mut buf = Vec::new();
                let mut prg_file = &in_files[i];
                let e = prg_file.read_to_end(&mut buf);
                println!("prg file {:?}", prg_file.metadata().unwrap().len());
                if e.is_err() {
                    panic!("Error reading prg seed {}, {}", i, e.unwrap_err());
                }
                let curr_seed = u64::from_le_bytes([0u8; 8]); // CHANGE
                let rng = ChaCha20Rng::seed_from_u64(curr_seed);
                // let next_seed = rng.gen::<u64>();
                // let mut prg_out_file = &out_files[i];
                // println!("new seed len {}", next_seed.to_le_bytes().len());
                // if prg_out_file.write_all(&next_seed.to_le_bytes()).is_err() {
                //     panic!("Error writing new prg seed {}", i)
                // }
                // assert!(prg_out_file.flush().is_ok());
                Scalar::random(rng)
            });

            let f_a = my_as.iter().map(|a| {
                let mut fa_j = Scalar::one();
                let factors = n_sub_a(num_parties, a.to_vec());
                for f in factors {
                    fa_j *= Scalar::from_uint_reduced(U256::from(rank)) - Scalar::from_uint_reduced(U256::from(f as u8));
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

            let result_vec_to_write = serde_json::to_vec(&result_vec).unwrap();
            assert_eq!(out_files.len(), 1);
            let mut out_file = &out_files[0];        
            out_file.write_all(result_vec_to_write.as_slice())?;


            Ok(())
        }
        "seed_prgs" => {
            let a_size = NUM_SERVERS - THRESHOLD;

            for (i, v) in generate_a(num_parties, a_size, rank as usize).iter().enumerate() {
                let sender = 0; // I think this also works bc the set elements are all in increasing order 
                println!("{} {:?}, {}", rank, v, sender);
                let mut out_file = &out_files[i ];
                let my_prg_seed = (v[0] * 256 + v[1] * 16 + v[2]) as u64; // change later?

                println!("{}", my_prg_seed.to_le_bytes().len());

                if v[sender] == rank as usize {
                    out_file.write_all(&my_prg_seed.to_le_bytes())?;
                    for j in 0..a_size {
                        if v[j] != rank as usize {
                            socks[v[j]].write_all(&my_prg_seed.to_le_bytes())?;
                        }
                    }
                } else {
                    let mut buf = [0u8; 8];
                    socks[v[sender]].read_exact(&mut buf)?; 
                    out_file.write_all(&buf)?;
                }
            }

            Ok(())
        }
        _ => panic!(),
    }
}
