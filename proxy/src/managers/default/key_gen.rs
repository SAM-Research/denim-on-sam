use denim_sam_common::rng::RngState;
use log::debug;
use sam_common::{api::EcPreKey, AccountId, DeviceId};
use sam_security::key_gen::generate_ec_pre_key;

use crate::managers::{error::DenimKeyManagerError, DenimEcPreKeyManager};

pub async fn generate_ec_pre_keys<R: RngState>(
    key_manager: &mut impl DenimEcPreKeyManager<R>,
    account_id: AccountId,
    device_id: DeviceId,
    amount: usize,
) -> Result<(), DenimKeyManagerError> {
    let mut id_rng = key_manager
        .get_key_id_seed_for(account_id, device_id)
        .await?
        .into_rng();

    let mut key_rng = key_manager
        .get_key_seed_for(account_id, device_id)
        .await?
        .into_rng();

    for _ in 0..amount {
        let key_id = key_manager
            .next_key_id(account_id, device_id, &mut id_rng)
            .await?;
        debug!("Generating EC Pre Key '{key_id}' for {account_id}.{device_id}");
        let pk: EcPreKey = generate_ec_pre_key(key_id.into(), &mut key_rng)
            .await
            .into();

        key_manager
            .add_ec_pre_key(account_id, device_id, pk.clone())
            .await?;

        key_manager
            .store_key_id_seed_for(account_id, device_id, id_rng.clone().into())
            .await?;

        key_manager
            .store_key_seed_for(account_id, device_id, key_rng.clone().into())
            .await?;
    }
    Ok(())
}
