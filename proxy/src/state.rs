#[derive(Clone)]
pub struct DenimState {
    sam_url: String,
    channel_buffer: usize,
}

impl DenimState {
    pub fn new(sam_addr: String, channel_buffer: usize) -> Self {
        Self {
            sam_url: format!("ws://{sam_addr}"),
            channel_buffer,
        }
    }

    pub fn sam_url(&self) -> &String {
        &self.sam_url
    }

    pub fn channel_buffer(&self) -> usize {
        self.channel_buffer
    }
}
