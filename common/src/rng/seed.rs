use std::ops::Deref;

use derive_more::From;
use rand::Rng;

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
