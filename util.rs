use blake2::{Blake2s256, Blake2b512, Digest};
use p256::{NonZeroScalar, Scalar, U256};
use elliptic_curve::{generic_array::{GenericArray, typenum::U32}, bigint::Encoding, subtle::ConstantTimeEq};
use block_padding::{Pkcs7, Padding};
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;

pub const THRESHOLD: usize = 2;
pub const NUM_SERVERS: usize = 5;
// This constant is for the number of A's each individual server belongs to.
pub const NUM_A: usize = 6; // NUM_SERVERS - 1 choose (NUM_SERVERS - THRESHOLD - 1)

pub fn string_hash_to_nzs(str: &str) -> NonZeroScalar {
    // make string take up 256 bits by hashing
    let mut hasher = Blake2s256::new();
    hasher.update(str.as_bytes());
    let mut buf = [0u8; 32]; 
    hasher.finalize_into_reset(GenericArray::from_mut_slice(&mut buf));
    
    // turn bytes into U256
    let str_uint = U256::from_be_bytes(buf);
    // get field element from U256 (Uint for the P256 curve)
    let str_nzs = NonZeroScalar::from_uint(str_uint).unwrap();
    str_nzs
}

pub fn sk_to_field_elts(str: &str) -> Vec<NonZeroScalar> {
    let sk_bytes = str.as_bytes();
    let mut sk_nzs_vec = Vec::new();
    // split string into chunks
    for chunk in sk_bytes.chunks(31) {
        let pos = chunk.len();
        let mut block: GenericArray::<u8, U32> = [0u8; 32].into();
        block[..pos].copy_from_slice(chunk);
        // pad each chunk to 32 bytes
        Pkcs7::pad(&mut block, pos);
        // turn bytes into U256
        let chunk_uint = U256::from_be_bytes(block.into());
        // get field element from U256 (Uint for the P256 curve)
        let chunk_nzs = NonZeroScalar::from_uint(chunk_uint).unwrap();
        sk_nzs_vec.push(chunk_nzs);
    }
    sk_nzs_vec
}

pub fn field_elts_to_string(scalars: &[Scalar]) -> String {
    let mut sk_combined = String::new();
    for scalar in scalars {
        let bytes: GenericArray::<u8, U32> = scalar.to_bytes();
        let res = Pkcs7::unpad(&bytes).unwrap();
        let sk_string = String::from_utf8(res.to_vec()).unwrap();
        sk_combined = sk_combined + sk_string.as_str();
    }
    sk_combined
}

//TODO: write test?
pub fn verify_sk_hash(salts: &[&[u8]], hashes: &[&[u8]], sk_vec: &[Scalar]) -> bool {
    let mut hasher = Blake2b512::new();
    for i in 0..hashes.len() {
        hasher.update(&salts[i]);
        for scalar in sk_vec {
            hasher.update(scalar.to_bytes());
        }
        let hash_result = hasher.finalize_reset();
        if hashes[i] != hash_result.to_vec() {
            return false;
        }
    }
    true
}

 #[test]
fn test_string_hash_to_nzs() {
    let str1 = String::from("str1");
    let str2 = String::from("str2");
    let nz1 = string_hash_to_nzs(&str1);
    let nz2 = string_hash_to_nzs(&str1);
    let nz3 = string_hash_to_nzs(&str2);

    assert_eq!(nz1.ct_eq(&nz2).unwrap_u8(), 1);
    assert_eq!(nz1.ct_eq(&nz3).unwrap_u8(), 0);
}

#[test]
fn test_sk_to_field_elt() {
    let sk1 = String::from("my_key");
    let nz1 = sk_to_field_elts(&sk1);
    let mut scalar_vec = Vec::new();
    for nz in nz1 {
        scalar_vec.push(*nz.as_ref());
    }
    let sk_recovered = field_elts_to_string(scalar_vec.as_slice());
    assert!(sk1 == sk_recovered);

    let sk2 = String::from("AD302A6F48F74DD6F9D257F7149E4D06CD8936FE200AF67E08EF88D1CBA4525D");
    let nz2 = sk_to_field_elts(&sk2);
    let mut scalar_vec2 = Vec::new();
    for nz in nz2 {
        scalar_vec2.push(*nz.as_ref());
    }
    let sk_recovered2 = field_elts_to_string(scalar_vec2.as_slice());
    assert!(sk2 == sk_recovered2);

    let sk3 = String::from("");
    let nz3 = sk_to_field_elts(&sk3);
    let mut scalar_vec3 = Vec::new();
    for nz in nz3 {
        scalar_vec3.push(*nz.as_ref());
    }
    let sk_recovered3 = field_elts_to_string(scalar_vec3.as_slice());
    assert!(sk3 == sk_recovered3);
}

//#[test]
// fn test_verify_sk_hash() {
//     let rng = &mut ChaCha20Rng::from_entropy();
//     let salt = rng.gen::<[u8; 32]>();
//     let scalars = &mut [Scalar::ONE; 3];
//     let mut hasher = Blake2b512::new();
//     hasher.update(salt);
//     for i in 0..3 {
//         hasher.update(scalars[i].to_bytes());
//     }
//     let mut salts = Vec::new();
//     let mut salts_bad = Vec::new();
//     let mut hashes = Vec::new();
//     let mut hashes_bad = Vec::new();
//     let res = hasher.finalize();
//     for _ in 0..3 {
//         salts.push(salt.to_vec());
//         hashes.push(res.to_vec());
//         salts_bad.push(Vec::new());
//         hashes_bad.push(Vec::new());
//     }
//     assert!(verify_sk_hash(salts, hashes, scalars));
//     scalars[0] = Scalar::ZERO;
//     assert!(!verify_sk_hash(salts_bad, hashes_bad, scalars));
// }
