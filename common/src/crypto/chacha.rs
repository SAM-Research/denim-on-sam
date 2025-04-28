use crate::KeySeed;
use async_trait::async_trait;
use rand::SeedableRng as _;
use rand_chacha::ChaCha20Rng;

use super::CryptoProvider;

#[derive(Clone)]
pub struct ChaChaCryptoProvider;

#[async_trait]
impl CryptoProvider for ChaChaCryptoProvider {
    type Rng = ChaCha20Rng;

    fn get_seeded(seed: KeySeed) -> ChaCha20Rng {
        ChaCha20Rng::from_seed(*seed)
    }

    fn get_seeded_with_offset(seed: KeySeed, offset: u128) -> ChaCha20Rng {
        let mut csprng = ChaCha20Rng::from_seed(*seed);
        csprng.set_word_pos(offset);
        csprng
    }

    fn extract_seed_offset(csprng: &ChaCha20Rng) -> (KeySeed, u128) {
        (csprng.get_seed().into(), csprng.get_word_pos())
    }
}
