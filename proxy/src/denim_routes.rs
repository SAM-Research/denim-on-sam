use denim_sam_common::{
    buffers::{ReceivingBufferConfig, SendingBufferConfig},
    denim_message::DeniableMessage,
};

use crate::{
    error::DenimRouterError,
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
) -> Result<DeniableMessage, DenimRouterError> {
    todo!("Denim Proxy does not yet support denim routing");
}
