use ark_ff::Field;
use rand::RngCore;

pub fn shard<F: Field>(n: F, num_shards: usize, rng: &mut impl RngCore) -> Vec<F>{
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

pub fn shard_to_bytes<F: Field>(n: F, num_shards: usize, rng: &mut impl RngCore) -> Vec<Vec<u8>> {
    let field_elts = shard::<F>(n, num_shards, rng);
    field_elts.iter().map(|f| {
        let mut b = Vec::new();
        assert!(f.serialize_uncompressed(&mut b).is_ok());
        b
    }).collect::<Vec<_>>()
}

/// There's a more rust-y way to do implement these conversions - use the From trait
fn to_bytes<F: Field>(n: &F) -> Vec<u8> {
    let mut v = Vec::new();
    assert!(n.serialize_uncompressed(&mut v).is_ok());
    v
}