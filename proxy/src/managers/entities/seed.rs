const RNG_SEED_SIZE: usize = 32;

#[derive(Debug, Clone)]
pub struct Seed([u8; RNG_SEED_SIZE]);

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
