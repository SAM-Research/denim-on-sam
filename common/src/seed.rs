use std::ops::Deref;

use derive_more::From;
use rand::{CryptoRng, Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::error::ConversionError;

/// 32 byte key seed for generating keys.
const KEY_SEED_SIZE: usize = 32;

/// 32 bit id seed for generating key ids.
const KEY_ID_SEED_SIZE: usize = 4;

macro_rules! define_seed_type {
    ($name:ident, $size_const:ident) => {
        #[derive(Debug, Clone, From)]
        pub struct $name([u8; $size_const]);

        impl $name {
            pub fn new(seed: [u8; $size_const]) -> Self {
                Self(seed)
            }
        }

        impl $name {
            pub fn random(rng: &mut impl Rng) -> Self {
                let mut bytes = [0u8; $size_const];
                rng.fill_bytes(&mut bytes);
                Self(bytes)
            }
        }

        #[cfg(test)]
        impl Default for $name {
            fn default() -> $name {
                $name([0; $size_const])
            }
        }

        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl AsMut<[u8]> for $name {
            fn as_mut(&mut self) -> &mut [u8] {
                &mut self.0
            }
        }

        impl Deref for $name {
            type Target = [u8; $size_const];

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl TryFrom<Vec<u8>> for $name {
            type Error = ConversionError;

            fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
                let seed: [u8; $size_const] = value
                    .try_into()
                    .map_err(|_| ConversionError::SeedConversionError)?;

                Ok($name::new(seed))
            }
        }
    };
}

define_seed_type!(KeySeed, KEY_SEED_SIZE);
define_seed_type!(KeyIdSeed, KEY_ID_SEED_SIZE);

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

pub trait RngState: Into<Self::Rng> + From<Self::Rng> + Send + Sync + Clone + Default {
    type Rng: Rng + CryptoRng + SeedableRng + Send + Clone;

    fn into_rng(self) -> Self::Rng;
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
