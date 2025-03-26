use crate::managers::KeyDistributionCenter;

#[derive(Debug, Default, Clone)]
pub struct InMemoryKeyDistributionCenter {}

impl KeyDistributionCenter for InMemoryKeyDistributionCenter {}
