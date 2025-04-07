use crate::managers::BufferManager;

#[derive(Debug, Default, Clone)]
pub struct InMemoryBufferManager {}

impl BufferManager for InMemoryBufferManager {}
