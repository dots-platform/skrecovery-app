use blake2::{Blake2s256, Blake2b512, Digest};
use p256::{NonZeroScalar, Scalar, U256};
use elliptic_curve::{generic_array::{GenericArray, typenum::U32}, bigint::Encoding, subtle::ConstantTimeEq};
use block_padding::{Pkcs7, Padding};

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

pub fn sk_pad_to_nzs(str: &str) -> NonZeroScalar {
    let sk_bytes = str.as_bytes();
    println!("{:?}", sk_bytes);
    // express string as blocks
    // pad each block to 32 bytes
    let pos = sk_bytes.len();
    println!("{}", pos);
    let mut block: GenericArray::<u8, U32> = [0u8; 32].into();
    block[..pos].copy_from_slice(sk_bytes);
    Pkcs7::pad(&mut block, pos);

    // turn bytes into U256
    let sk_uint = U256::from_be_bytes(block.into());
    // get field element from U256 (Uint for the P256 curve)
    let sk_nzs = NonZeroScalar::from_uint(sk_uint).unwrap();
    sk_nzs
}

pub fn scalar_unpad_to_string(scalar: Scalar) -> String {
    let bytes: GenericArray::<u8, U32> = scalar.to_bytes();
    let res = Pkcs7::unpad(&bytes).unwrap();
    let sk_string = String::from_utf8(res.to_vec()).unwrap();
    sk_string
}

//TODO: write test?
pub fn verify_sk_hash(salts: Vec<Vec<u8>>, hashes: Vec<Vec<u8>>, sk: Scalar) -> bool {
    let mut hasher = Blake2b512::new();
    hasher.update(&salts[0]);
    hasher.update(&sk.to_bytes());
    let hash_result = hasher.finalize();
    if hashes[0] != hash_result.to_vec() {
        false
    } else {
        true
    }
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
fn test_sk_to_nzs() {
    let sk1 = String::from("my_key");
    let nz1 = sk_pad_to_nzs(&sk1);
    let sk_recovered = scalar_unpad_to_string(*nz1.as_ref());
    assert!(sk1 == sk_recovered);
}