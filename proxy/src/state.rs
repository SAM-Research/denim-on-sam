use std::sync::Arc;

#[derive(Clone)]
pub struct DenimState {
    sam_url: String,
    channel_buffer: usize,
    ws_proxy_tls_config: Option<Arc<rustls::ClientConfig>>,
}

impl DenimState {
    pub fn new(
        sam_addr: String,
        channel_buffer: usize,
        ws_proxy_tls_config: Option<Arc<rustls::ClientConfig>>,
    ) -> Self {
        Self {
            sam_url: sam_addr,
            channel_buffer,
            ws_proxy_tls_config,
        }
    }

    pub fn sam_url(&self) -> &String {
        &self.sam_url
    }

    pub fn channel_buffer(&self) -> usize {
        self.channel_buffer
    }

    pub fn ws_proxy_tls_config(&self) -> Option<Arc<rustls::ClientConfig>> {
        self.ws_proxy_tls_config.clone()
    }
}
