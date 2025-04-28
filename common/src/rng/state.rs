use rand::{CryptoRng, Rng, SeedableRng};

pub trait RngState: Into<Self::Rng> + From<Self::Rng> + Send + Sync + Clone + Default {
    type Rng: Rng + CryptoRng + SeedableRng + Send + Clone;

    fn into_rng(self) -> Self::Rng;
}
