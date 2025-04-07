use denim_sam_common::{PreKeyBundle, Seed};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sam_client::storage::key_generation::generate_ec_pre_key;
use sam_common::{
    api::{EcPreKey, SignedEcPreKey},
    AccountId,
};
use sam_server::managers::traits::{
    account_manager::AccountManager, device_manager::DeviceManager,
};

use crate::{
    error::ServerError,
    managers::{DenimEcPreKeyManager, DenimSignedPreKeyManager, DEFAULT_DEVICE},
    state::{DenimState, StateType},
};

pub async fn get_keys_for<T: StateType>(
    state: &mut DenimState<T>,
    account_id: AccountId,
) -> Result<PreKeyBundle, ServerError> {
    let pk_res = state.keys.pre_keys.get_ec_pre_key(account_id).await;

    let pre_key = if let Ok(pk) = pk_res {
        pk
    } else {
        let mut csprng = state.keys.pre_keys.get_csprng_for(account_id).await?;
        for _ in 0..=100 {
            let pk: EcPreKey = generate_ec_pre_key(44.into(), &mut csprng).await.into();
            state
                .keys
                .pre_keys
                .add_ec_pre_key(account_id, pk.clone())
                .await?;
            state
                .keys
                .pre_keys
                .store_csprng_for(account_id, &csprng)
                .await?;
        }

        state.keys.pre_keys.get_ec_pre_key(account_id).await?
    };

    let signed_pre_key = state
        .keys
        .signed_pre_keys
        .get_signed_pre_key(account_id)
        .await?;

    let device = state
        .devices
        .get_device(account_id, DEFAULT_DEVICE.into())
        .await?;

    Ok(PreKeyBundle::new(
        device.id(),
        device.registration_id(),
        pre_key,
        signed_pre_key,
    ))
}

pub async fn update_signed_pre_key<T: StateType>(
    state: &mut DenimState<T>,
    account_id: AccountId,
    signed_pre_key: SignedEcPreKey,
) -> Result<(), ServerError> {
    state
        .keys
        .signed_pre_keys
        .set_signed_pre_key(
            account_id,
            state.accounts.get_account(account_id).await?.identity(),
            signed_pre_key,
        )
        .await?;
    Ok(())
}

pub async fn update_seed<T: StateType>(
    state: &mut DenimState<T>,
    account_id: AccountId,
    seed: Seed,
) -> Result<(), ServerError> {
    let csprng = ChaCha20Rng::from_seed(*seed);
    state
        .keys
        .pre_keys
        .store_csprng_for(account_id, &csprng)
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use libsignal_protocol::IdentityKeyPair;
    use rand::{rngs::OsRng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use sam_client::storage::key_generation::generate_signed_pre_key;
    use sam_common::{api::Key as _, AccountId};
    use sam_server::{
        auth::password::Password,
        managers::{
            entities::{Account, Device},
            traits::{account_manager::AccountManager as _, device_manager::DeviceManager as _},
        },
    };

    use crate::{
        logic::keys::get_keys_for,
        managers::{DenimEcPreKeyManager, DenimSignedPreKeyManager, DEFAULT_DEVICE},
        state::{DenimState, InMemory},
    };

    #[tokio::test]
    async fn test_get_keybundle() {
        let mut state = DenimState::<InMemory>::in_memory_test("127.0.0.1:8000".to_owned());
        let mut rng = OsRng;
        let pair = IdentityKeyPair::generate(&mut rng);

        let account = Account::builder()
            .id(AccountId::generate())
            .identity(*pair.identity_key())
            .username("Alice".to_string())
            .build();

        state
            .accounts
            .add_account(&account)
            .await
            .expect("Can add account");

        let device = Device::builder()
            .id(DEFAULT_DEVICE.into())
            .name("Alice Secret Phone".to_string())
            .password(Password::generate("dave<3".to_string()).expect("Alice can create password"))
            .creation(0)
            .registration_id(1.into())
            .build();

        let account_id = account.id();
        state
            .devices
            .add_device(account_id, &device)
            .await
            .expect("Alice can add device");

        let signed_pre_key = generate_signed_pre_key(22.into(), pair.private_key(), &mut rng)
            .await
            .expect("Can generate Signed Pre Key");

        state
            .keys
            .signed_pre_keys
            .set_signed_pre_key(account_id, pair.identity_key(), signed_pre_key.into())
            .await
            .expect("Can set signed pre key");

        let alice_rng = ChaCha20Rng::from_rng(rng).expect("Can create RNG");
        state
            .keys
            .pre_keys
            .store_csprng_for(account_id, &alice_rng)
            .await
            .expect("Can store csprng");

        // Error if no pre_key
        assert!(state
            .keys
            .pre_keys
            .get_ec_pre_key(account_id)
            .await
            .is_err());

        // testing if we get keys
        let bundle = get_keys_for(&mut state, account_id)
            .await
            .expect("User have uploaded bundles");

        // Now, pre keys should have been generated.
        assert!(state.keys.pre_keys.get_ec_pre_key(account_id).await.is_ok());

        assert!(bundle.device_id == DEFAULT_DEVICE);
        assert!(bundle.registration_id == 1);
        assert!(bundle.signed_pre_key.id() == 22);
    }
}
