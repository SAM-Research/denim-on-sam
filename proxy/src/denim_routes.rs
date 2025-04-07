use denim_sam_common::buffers::{ReceivingBufferConfig, SendingBufferConfig};
use log::info;

use crate::{
    managers::{default::ClientRequest, traits::MessageIdProvider},
    state::DenimState,
};

pub async fn denim_router<
    T: ReceivingBufferConfig,
    U: SendingBufferConfig,
    V: MessageIdProvider,
>(
    _state: DenimState<T, U, V>,
    _request: ClientRequest,
) {
    info!("TODO: Denim Routing");
}
