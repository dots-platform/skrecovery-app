pub const THRESHOLD: usize = 2;
pub const NUM_SERVERS: usize = 5;

/* use rand::RngCore;

pub fn shard<F: Field>(n: F, num_shards: usize, rng: &mut impl RngCore) -> Vec<F> {
    // Initialize random number array, sum
    let random_vals = (0..num_shards).map(|_| F::rand(rng)).collect::<Vec<F>>();
    let sum = random_vals.iter().sum();
    // Find the inverse of sum
    let sum_inv = match F::inverse(&sum) {
        Some(s) => s,
        None => panic!("some random numbers summed to zero, go buy a lottery ticket"),
    };
    // Multiple all n random numbers by sk * sum^-1
    let shards = random_vals
        .iter()
        .map(|x| *x * sum_inv * n)
        .collect::<Vec<F>>();
    // Return shards
    shards
}

#[test]
fn test_shard() {
    use ark_bls12_381::Fr;
    use ark_ff::UniformRand;

    let rng = &mut ark_std::test_rng();
    // test with a variety of shard sizes
    for i in 1..20 {
        let f = Fr::rand(rng);
        let shards = shard(f, i, rng);
        let s: Fr = shards.iter().sum();

        assert_eq!(s, f);
    }
}

pub fn shard_to_bytes<F: Field>(n: F, num_shards: usize, rng: &mut impl RngCore) -> Vec<Vec<u8>> {
    let field_elts = shard::<F>(n, num_shards, rng);
    field_elts
        .iter()
        .map(|f| {
            let mut b = Vec::new();
            assert!(f.serialize_uncompressed(&mut b).is_ok());
            b
        })
        .collect::<Vec<_>>()
}

#[test]
fn test_shard_to_bytes() {
    use ark_bls12_381::Fr;
    use ark_ff::UniformRand;
    use ark_serialize::CanonicalDeserialize;

    let rng = &mut ark_std::test_rng();
    // test with a variety of shard sizes
    for i in 1..20 {
        let f = Fr::rand(rng);
        let shards = shard_to_bytes(f, i, rng);
        let s: Fr = shards
            .iter()
            .map(|x| Fr::deserialize_uncompressed(x.as_slice()).unwrap())
            .sum();

        assert_eq!(s, f);
    }
}
*/
