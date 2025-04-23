use async_trait::async_trait;
use denim_sam_common::Seed;
use rand::{CryptoRng, Rng};

#[async_trait]
pub trait CryptoProvider<R: CryptoRng + Rng>: Clone + Send + Sync {
    async fn get_seeded(seed: Seed) -> R;
    async fn get_seeded_with_offset(seed: Seed, offset: u128) -> R;
    async fn extract_seed_offset(csprng: R) -> (Seed, u128);
}
