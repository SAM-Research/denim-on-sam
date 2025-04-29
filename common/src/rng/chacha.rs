use derive_more::From;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use super::{
    seed::{KeyIdSeed, KeySeed},
    RngState,
};

impl From<KeyIdSeed> for ChaChaRngState {
    fn from(value: KeyIdSeed) -> Self {
        let bytes = *value;
        let seed = u32::from_be_bytes(bytes);
        ChaCha20Rng::seed_from_u64(seed as u64).into()
    }
}

impl From<KeySeed> for ChaChaRngState {
    fn from(value: KeySeed) -> Self {
        let bytes = *value;
        ChaChaRngState::from_seed_and_offset(bytes, 0)
    }
}

#[derive(Debug, Clone, From, Default)]
pub struct ChaChaRngState(<ChaCha20Rng as SeedableRng>::Seed, u128);

impl ChaChaRngState {
    pub fn new(seed: <ChaCha20Rng as SeedableRng>::Seed) -> Self {
        Self(seed, 0)
    }

    pub fn from_seed_and_offset(seed: <ChaCha20Rng as SeedableRng>::Seed, offset: u128) -> Self {
        Self(seed, offset)
    }

    pub fn random(rng: &mut impl Rng) -> Self {
        let mut bytes = <ChaCha20Rng as SeedableRng>::Seed::default();
        rng.fill_bytes(&mut bytes);
        Self(bytes, 0)
    }
}

impl RngState for ChaChaRngState {
    type Rng = ChaCha20Rng;

    fn into_rng(self) -> Self::Rng {
        ChaCha20Rng::from(self)
    }
}

impl From<ChaChaRngState> for ChaCha20Rng {
    fn from(value: ChaChaRngState) -> Self {
        let mut rng = ChaCha20Rng::from_seed(value.0);
        rng.set_word_pos(value.1);
        rng
    }
}

impl From<ChaCha20Rng> for ChaChaRngState {
    fn from(value: ChaCha20Rng) -> Self {
        Self::from_seed_and_offset(value.get_seed(), value.get_word_pos())
    }
}
