use dtrust::utils::init_app;
use itertools::{Combinations, Itertools};
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

use elliptic_curve::{ff::PrimeField, generic_array::GenericArray, Field};
use p256::{NonZeroScalar, Scalar, SecretKey, U256};
use vsss_rs::{Shamir, Share};

fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;

    println!("rank {} starting", rank);

    let num_parties = socks.len();

    match &func_name[..] {
        "skrecovery" => {
            // compute R(PW-PWG) share locally
            let rng = &mut ChaCha20Rng::from_entropy();

            // TODO: replace 6 with 4 choose 2
            let shares = in_files[NUM_A..]
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

            let id = shares[0].0;
            let sk_share = shares[0].1;
            let pwd_share = shares[1].1;
            let pwd_guess_share = shares[2].1;

            let dummy_rng = &mut ChaCha20Rng::seed_from_u64(12u64);
            let debug_nzs = NonZeroScalar::from_uint(U256::from(256u64)).unwrap();
            let debug_shares = Shamir::<THRESHOLD, NUM_SERVERS>::split_secret::<
                Scalar,
                ChaCha20Rng,
                33,
            >(*debug_nzs.as_ref(), dummy_rng)
            .unwrap();

            // Thanks Emma for showing us this neat trick!
            // https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=96317e8e38cc956da308026e5328948ebd9d49ad

            let r_A = (0..num_parties).map(|x| {
                if x != rank as usize {
                    let mut buf = [0u8; 8];
                    let mut prg_file = &in_files[x];
                    if prg_file.read_exact(&mut buf).is_err() {
                        panic!("Error reading prg seed {}", x)
                    }
                    let curr_seed = u64::from_le_bytes(buf);
                    let mut rng = ChaCha20Rng::seed_from_u64(curr_seed);
                    let next_seed = rng.gen::<u64>();
                    let mut prg_out_file = &out_files[x];
                    if prg_out_file.write_all(&next_seed.to_le_bytes()).is_err() {
                        panic!("Error writing new prg seed {}", x)
                    }
                    Scalar::random(rng)
                } else {
                    Scalar::zero() // dummy value, never getting read
                }
            });

            // let f_A =
            let my_share = debug_shares[rank as usize]
                .as_field_element::<Scalar>()
                .unwrap();

            // TODO check that all ids are the same maybe this isn't necessary

            let random_scalar = Scalar::random(rng);
            let field_to_write = (pwd_share - pwd_guess_share) * random_scalar + sk_share;
            let mut result = vec![id];
            result.extend(field_to_write.to_bytes());

            assert_eq!(out_files.len(), 1);
            let mut out_file = &out_files[0];
            out_file.write_all(&result)?;
            Ok(())
        }
        "seed_prgs" => {
            let other_parties = (0..num_parties - 1)
                .map(|x| if x < rank as usize { x } else { x + 1 })
                .collect::<Vec<usize>>();
            println!("{:?}", other_parties);
            let a_size = NUM_SERVERS - THRESHOLD;
            let my_prg_seed = rank as u64;

            // boy oh boy i hope this combinations thing is deterministic, because if it isn't everything breaks
            for (i, mut v) in other_parties.iter().combinations(a_size - 1).enumerate() {
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

                // TODO TODO TODO: 
                // some issue with "writing to something that's not a socket"
                let sender = v.iter().copied().sum::<usize>() % a_size; // index of the thing to be sent
                println!("{} {:?}, {}", rank, v, sender);
                let mut out_file = &out_files[i];
                if *v[sender] == rank as usize {
                    out_file.write_all(my_prg_seed.to_string().as_bytes())?;
                    for j in 0..a_size {
                        if *v[j] != sender {
                            socks[*v[j]].write_all(my_prg_seed.to_string().as_bytes())?;
                        }
                    }
                } else {
                    let mut buf = [0u8; 8];
                    socks[*v[sender]].read(&mut buf)?;
                    out_file.write_all(&buf)?;
                }
            }

            Ok(())
        }
        _ => panic!(),
    }
}
