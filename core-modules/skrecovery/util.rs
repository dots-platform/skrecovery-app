use blake2::{Blake2s256, Digest};
use p256::{NonZeroScalar, U256};
use elliptic_curve::{generic_array::GenericArray, bigint::Encoding, subtle::ConstantTimeEq};

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
    return str_nzs;
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