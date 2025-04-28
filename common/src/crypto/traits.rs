use crate::KeySeed;
use async_trait::async_trait;
use rand::{CryptoRng, Rng, SeedableRng};

#[async_trait]
pub trait CryptoProvider: Clone + Send + Sync {
    type Rng: SeedableRng + CryptoRng + Rng + Send;
    fn get_seeded(seed: KeySeed) -> Self::Rng;
    fn get_seeded_with_offset(seed: KeySeed, offset: u128) -> Self::Rng;
    fn extract_seed_offset(csprng: &Self::Rng) -> (KeySeed, u128);
}
