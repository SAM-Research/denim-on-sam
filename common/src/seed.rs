use std::ops::Deref;

use derive_more::From;
use rand::Rng;

const RNG_SEED_SIZE: usize = 32;

#[derive(Debug, Clone, From)]
pub struct Seed([u8; RNG_SEED_SIZE]);

impl Seed {
    pub fn new(seed: [u8; RNG_SEED_SIZE]) -> Self {
        Self(seed)
    }
}

impl Seed {
    pub fn random(rng: &mut impl Rng) -> Self {
        let mut bytes = [0u8; RNG_SEED_SIZE];
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }
}

#[cfg(test)]
impl Default for Seed {
    fn default() -> Seed {
        Seed([0; RNG_SEED_SIZE])
    }
}

impl AsRef<[u8]> for Seed {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Seed {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Deref for Seed {
    type Target = [u8; RNG_SEED_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
