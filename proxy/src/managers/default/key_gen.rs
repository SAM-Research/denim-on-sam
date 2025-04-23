use rand::{CryptoRng, Rng};

use sam_common::{api::EcPreKey, AccountId, DeviceId};
use sam_security::key_gen::generate_ec_pre_key;

use crate::managers::{traits::CryptoProvider, DenimEcPreKeyManager, DenimKeyManagerError};

pub async fn generate_ec_pre_keys<C: CryptoProvider<R>, R: CryptoRng + Rng>(
    key_manager: &mut impl DenimEcPreKeyManager,
    account_id: AccountId,
    device_id: DeviceId,
    amount: usize,
) -> Result<(), DenimKeyManagerError> {
    let (seed, offset) = key_manager.get_csprng_for(account_id, device_id).await?;
    let mut csprng = C::get_seeded_with_offset(seed, offset);

    for _ in 0..amount {
        let pk: EcPreKey = generate_ec_pre_key(
            key_manager.next_key_id(account_id, device_id).await?.into(),
            &mut csprng,
        )
        .await
        .into();
        key_manager
            .add_ec_pre_key(account_id, device_id, pk.clone())
            .await?;

        let (seed, offset) = C::extract_seed_offset(&csprng);
        key_manager
            .store_csprng_for(account_id, device_id, seed, offset)
            .await?;
    }
    Ok(())
}
