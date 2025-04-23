use async_trait::async_trait;
use denim_sam_common::Seed;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::managers::traits::CryptoProvider;

#[derive(Clone)]
pub struct ChaChaCryptoProvider;

#[async_trait]
impl CryptoProvider<ChaCha20Rng> for ChaChaCryptoProvider {
    fn get_seeded(seed: Seed) -> ChaCha20Rng {
        ChaCha20Rng::from_seed(*seed)
    }
    fn get_seeded_with_offset(seed: Seed, offset: u128) -> ChaCha20Rng {
        let mut csprng = ChaCha20Rng::from_seed(*seed);
        csprng.set_word_pos(offset);
        csprng
    }

    fn extract_seed_offset(csprng: &ChaCha20Rng) -> (Seed, u128) {
        (csprng.get_seed().into(), csprng.get_word_pos())
    }
}
