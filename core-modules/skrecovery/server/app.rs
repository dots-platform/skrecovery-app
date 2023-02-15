use dtrust::utils::init_app;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::io;
use std::io::prelude::*;

// we use this field for all arithmetic
// TODO: move this field choice into a config somewhere. Also, put the number of nodes in this config
// Also the file names we use for client/server communication.
use std::net::TcpStream;

#[path = "../util.rs"]
mod util;
//use util::shard_to_bytes;

const F_SIZE: usize = 32;

use elliptic_curve::{ff::PrimeField, generic_array::GenericArray, Field};
use p256::{NonZeroScalar, Scalar, SecretKey};
use vsss_rs::{Shamir, Share};

fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;

    println!("rank {} starting", rank);

    let num_parties = socks.len() - 1;

    match &func_name[..] {
        "skrecovery" => {
            // compute R(PW-PWG) share locally
            let rng = &mut ChaCha20Rng::from_entropy();

            let shares = in_files
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

            let mut v = vec![id];
            v.extend(sk_share.to_bytes());

            // TODO check that all ids are the same maybe this isn't necessary

            let random_scalar = Scalar::random(rng);
            let z = random_scalar * (pwd_share - pwd_guess_share) + Scalar::one();

            let field_to_write: Scalar = sk_share * z;
            let mut result = vec![id];
            result.extend(field_to_write.to_bytes());

            assert_eq!(out_files.len(), 1);
            let mut out_file = &out_files[0];
            out_file.write_all(&result)?;
            Ok(())
        }
        _ => panic!(),
    }
}
