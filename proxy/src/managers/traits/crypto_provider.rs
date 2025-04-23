use async_trait::async_trait;
use denim_sam_common::Seed;
use rand::{CryptoRng, Rng};

#[async_trait]
pub trait CryptoProvider: Clone + Send + Sync {
    type Rng: CryptoRng + Rng + Send;
    fn get_seeded(seed: Seed) -> Self::Rng;
    fn get_seeded_with_offset(seed: Seed, offset: u128) -> Self::Rng;
    fn extract_seed_offset(csprng: &Self::Rng) -> (Seed, u128);
}
