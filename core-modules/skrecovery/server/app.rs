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

            let shares = in_files.iter().map(|mut f| {
                let mut buf = [0; 33];
                if f.read_exact(&mut buf).is_err() {
                    panic!("Error reading file");
                }
                let share = Share::from(buf);

                (share.identifier(), share.as_field_element())

            }).collect::<Vec<(u8, Scalar)>>();
            
            let sk_shard = shares[0].1;
            let pwd_shard = shares[1].1;
            let pwd_guess_shard = shares[2].1;

            // TODO check that all ids are the same maybe this is extra idk

            let random_scalar = Scalar::random(&mut rng);
            let z = random_scalar * (pwd_shard - pwd_guess_shard);

            let field_to_write: Scalar = sk_shard + z;
            let mut result = Vec::new();

            // TODO: replace with correct way to serialize scalar
            assert!(field_to_write.serialize_uncompressed(&mut result).is_ok());
            assert_eq!(out_files.len(), 1);
            let mut out_file = &out_files[0];
            out_file.write_all(&result)?;

            // TODO: send back salt and hash
            Ok(())
        }
        _ => panic!(),
    }
}
