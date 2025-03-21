use crate::denim_message::DenimMessage;

struct ReceivingBuffer {
    q: f32,
    buffer: Vec<u8>,
}

trait ReceivingBufferHandler {
    fn process_chunks(&self) -> Vec<DenimMessage>;
}