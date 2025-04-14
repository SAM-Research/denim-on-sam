use denim_sam_common::denim_message::DeniableMessage;
use sam_common::AccountId;

use crate::{
    error::DenimRouterError,
    managers::default::ClientRequest,
    state::{DenimState, StateType},
};

pub async fn denim_router<T: StateType>(
    _state: DenimState<T>,
    _request: ClientRequest,
    _account_id: AccountId,
) -> Result<DeniableMessage, DenimRouterError> {
    todo!("Denim Proxy does not yet support denim routing");
}
