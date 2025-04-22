use async_trait::async_trait;
use denim_sam_common::Seed;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::managers::traits::CryptoProvider;

#[derive(Clone, Default)]
pub struct ChaChaProvider;

#[async_trait]
impl CryptoProvider<ChaCha20Rng> for ChaChaProvider {
    async fn get_seeded(&self, seed: Seed) -> ChaCha20Rng {
        ChaCha20Rng::from_seed(*seed)
    }
    async fn get_seeded_with_offset(&self, seed: Seed, offset: u128) -> ChaCha20Rng {
        let mut csprng = ChaCha20Rng::from_seed(*seed);
        csprng.set_word_pos(offset);
        csprng
    }

    async fn extract_seed_offset(csprng: ChaCha20Rng) -> (Seed, u128) {
        (csprng.get_seed().into(), csprng.get_word_pos())
    }
}
