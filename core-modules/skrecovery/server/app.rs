use dtrust::utils::init_app;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::io;
use std::io::prelude::*;

// we use this field for all arithmetic
// TODO: move this field choice into a config somewhere. Also, put the number of nodes in this config
// Also the file names we use for client/server communication.
use ark_bls12_381::Fr as F;
use ark_ff::One;
use ark_ff::UniformRand;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use std::net::TcpStream;

#[path = "../util.rs"]
mod util;
use util::shard_to_bytes;

const F_SIZE: usize = 32;

fn read_beaver_elt(socks: &mut [TcpStream], server_id: usize) -> Result<F, SerializationError> {
    let mut buf = [0u8; F_SIZE];
    socks[server_id].read(&mut buf)?;
    F::deserialize_uncompressed(buf.as_slice())
}

fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;

    println!("rank {} starting", rank);

    let beaver_server = socks.len() - 1;
    let num_parties = socks.len() - 1;

    match &func_name[..] {
        "skrecovery" => {
            if rank == beaver_server as u8 {
                let rng = &mut ChaCha20Rng::from_entropy();
                let a = F::rand(rng);
                let b = F::rand(rng);
                let c = a * b;

                let a_shards = shard_to_bytes::<F>(a, num_parties, rng);
                let b_shards = shard_to_bytes::<F>(b, num_parties, rng);
                let c_shards = shard_to_bytes::<F>(c, num_parties, rng);

                for i in 0..socks.len() {
                    if i != beaver_server {
                        // NOTE: only works if beaver_server is the LAST NODE!
                        socks[i].write_all(&a_shards[i])?;
                        socks[i].write_all(&b_shards[i])?;
                        socks[i].write(&c_shards[i])?;
                    }
                }
                Ok(())
            } else {
                assert_eq!(in_files.len(), 3);

                let field_elts = in_files
                    .iter()
                    .map(|mut f| {
                        let mut buf = Vec::new();
                        if f.read_to_end(&mut buf).is_err() {
                            panic!("Error reading file");
                        }
                        F::deserialize_uncompressed(buf.as_slice()).unwrap()
                    })
                    .collect::<Vec<F>>();

                let sk_shard = field_elts[0];
                let pwd_shard = field_elts[1];
                let pwd_guess_shard = field_elts[2];

                // TODO: write them all together
                let beaver_a = read_beaver_elt(&mut socks, beaver_server).unwrap();
                let beaver_b = read_beaver_elt(&mut socks, beaver_server).unwrap();
                let beaver_c = read_beaver_elt(&mut socks, beaver_server).unwrap();

                let rng = &mut ChaCha20Rng::from_entropy();

                let hiding: F = F::rand(rng);

                // ROUND 1: multiplication
                // Send R - a1, and (PW - PWG) - b1
                let elts_to_write = (hiding - beaver_a, (pwd_shard - pwd_guess_shard) - beaver_b);
                let mut v1 = Vec::new();
                assert!(elts_to_write.serialize_uncompressed(&mut v1).is_ok());

                //broadcast to all other nodes
                for i in 0..socks.len() {
                    if i != (rank as usize) && i != beaver_server {
                        socks[i as usize].write_all(&v1)?;
                    }
                }

                let mut buf1 = [0u8; F_SIZE * 2];
                let mut x_sub_a = elts_to_write.0;
                let mut y_sub_b = elts_to_write.1;

                for i in 0..socks.len() {
                    if i != (rank as usize) && i != beaver_server {
                        socks[i as usize].read(&mut buf1)?;
                        let resp = <(F, F)>::deserialize_uncompressed(buf1.as_slice()).unwrap();
                        x_sub_a += resp.0;
                        y_sub_b += resp.1;
                    }
                }

                // here, 0 is the special node who adds a little extra term, but it doesnt have to be like that.
                let z;
                if rank == 0 {
                    z = beaver_c + x_sub_a * beaver_b + y_sub_b * beaver_a + x_sub_a * y_sub_b;
                } else if rank > (socks.len() as u8) - 1 {
                    panic!("oops");
                } else {
                    z = beaver_c + x_sub_a * beaver_b + y_sub_b * beaver_a
                }

                // ROUND 2: exchange z's
                // TODO make rounds more generic? it's basically just sending something serializable and deserializing it.

                let mut v2 = Vec::new();
                let mut combined_z = z;
                assert!(z.serialize_uncompressed(&mut v2).is_ok());
                for i in 0..socks.len() {
                    if i != (rank as usize) && i != beaver_server {
                        socks[i as usize].write_all(&v2)?;
                    }
                }

                let mut buf2 = [0u8; F_SIZE];
                for i in 0..socks.len() {
                    if i != (rank as usize) && i != beaver_server {
                        socks[i as usize].read(&mut buf2)?;
                        let z_share = F::deserialize_uncompressed(buf2.as_slice()).unwrap();
                        combined_z += z_share;
                    }
                }

                let field_to_write: F = sk_shard * (combined_z + F::one());

                let mut result = Vec::new();
                assert!(field_to_write.serialize_uncompressed(&mut result).is_ok());

                assert_eq!(out_files.len(), 1);
                let mut out_file = &out_files[0];
                out_file.write_all(&result)?;

                Ok(())
            }
        }
        _ => panic!(),
    }
}
