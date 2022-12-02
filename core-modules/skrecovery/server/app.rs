use std::io;
use std::io::prelude::*;
use dtrust::utils::init_app;
use rand_chacha::ChaCha20Rng;
use rand::prelude::*;

// we use this field for all arithmetic 
// TODO: move this field choice into a config somewhere. Also, put the number of nodes in this config
// Also the file names we use for client/server communication. 
use ark_bls12_381::Fr as F;
use ark_ff::{One, Field};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use ark_ff::UniformRand;
use std::net::TcpStream;

const F_SIZE: usize = 32; // ceil(381 * 12 / 8)
const BEAVER_SERVER: usize = 2;

fn read_beaver_elt(socks: &mut Vec<TcpStream>) -> Result<F, SerializationError> {
    let mut buf = [0u8; F_SIZE];
    socks[2].read(&mut buf)?;
    F::deserialize_uncompressed(buf.as_slice())
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

fn shard_to_bytes<F: Field>(n: F, num_shards: usize, rng: &mut impl RngCore) -> Vec<Vec<u8>> {
    let field_elts = shard::<F>(n, num_shards, rng);
    field_elts.iter().map(|f| {
        let mut b = Vec::new();
        assert!(f.serialize_uncompressed(&mut b).is_ok());
        b
    }).collect::<Vec<_>>()
}

fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;

    println!("rank {} starting", rank);


    match &func_name[..] {
        "skrecovery" =>
        {
            if rank == BEAVER_SERVER as u8 {
                let rng = &mut ChaCha20Rng::from_entropy();
                let a = F::rand(rng);
                let b = F::rand(rng);
                let c = a * b;

                let a_shards = shard_to_bytes::<F>(a, 2, rng);
                let b_shards = shard_to_bytes::<F>(b, 2, rng);
                let c_shards = shard_to_bytes::<F>(c, 2, rng);

                for i in 0..socks.len() {
                    if i != BEAVER_SERVER {
                        // NOTE: only works if BEAVER_SERVER is the LAST NODE!
                        socks[i].write(&a_shards[i])?;
                        socks[i].write(&b_shards[i])?;
                        socks[i].write(&c_shards[i])?;
                    }
                }

            }
            assert_eq!(in_files.len(), 3);

            let field_elts = in_files.iter().map(|mut f| {
                let mut buf = vec![];
                if f.read_to_end(&mut buf).is_err() {
                    panic!("Error reading file");
                }
                F::deserialize_uncompressed(buf.as_slice()).unwrap()

            }).collect::<Vec<F>>();


            let sk_shard = field_elts[0];
            let pwd_shard = field_elts[1];
            let pwd_guess_shard = field_elts[2];

            // TODO: write them all together
            let beaver_a = read_beaver_elt(&mut socks).unwrap();
            let beaver_b = read_beaver_elt(&mut socks).unwrap();
            let beaver_c = read_beaver_elt(&mut socks).unwrap();

            let rng = &mut ChaCha20Rng::from_entropy();

            let hiding: F = F::rand(rng);

            // ROUND 1: multiplication
            // Send R - a1, and (PW - PWG) - b1
            let elts_to_write = (hiding - beaver_a, (pwd_shard - pwd_guess_shard) - beaver_b);
            let mut v1 = Vec::new();
            assert!(elts_to_write.serialize_uncompressed(&mut v1).is_ok());
            socks[1 - rank as usize].write_all(&v1)?;

            let mut buf1 = [0u8; F_SIZE * 2];
            socks[1 - rank as usize].read(&mut buf1)?;
            let resp1 = <(F, F)>::deserialize_uncompressed(buf1.as_slice()).unwrap();

            let x_sub_a = resp1.0 + elts_to_write.0;
            let y_sub_b = resp1.1 + elts_to_write.1;

            // here, 0 is the special node who adds a little extra term, but it doesnt have to be like that. 
            let z = match rank {
                0 => beaver_c + x_sub_a * beaver_b + y_sub_b * beaver_a,
                1 => beaver_c + x_sub_a * beaver_b + y_sub_b * beaver_a + x_sub_a * y_sub_b,
                _ => panic!("oops")
            };

            // ROUND 2: exchange z's
            // TODO make rounds more generic? it's basically just sending something serializable and deserializing it. 

            let mut v2 = Vec::new();
            assert!(z.serialize_uncompressed(&mut v2).is_ok());
            socks[1 - rank as usize].write(&v2)?;

            let mut buf2 = [0u8; F_SIZE];
            socks[1 - rank as usize].read(&mut buf2)?;
            let other_z = F::deserialize_uncompressed(buf2.as_slice()).unwrap();

            let field_to_write: F = sk_shard * (z + other_z + F::one());

            let mut result = Vec::new();
            assert!(field_to_write.serialize_uncompressed(&mut result).is_ok());

            assert_eq!(out_files.len(), 1);
            let mut out_file = &out_files[0];
            out_file.write_all(&result)?;

            Ok(())
        }
        _ => panic!()
    }
}