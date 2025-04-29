use denim_sam_common::{
    denim_message::KeyBundle,
    rng::seed::{KeyIdSeed, KeySeed},
};
use log::error;

use sam_common::{
    api::{Encode, SignedEcPreKey},
    AccountId, DeviceId,
};
use sam_server::managers::traits::{
    account_manager::AccountManager, device_manager::DeviceManager,
    key_manager::SignedPreKeyManager as _,
};

use crate::{
    error::LogicError,
    managers::{error::DenimKeyManagerError, DenimEcPreKeyManager},
    state::{DenimState, StateType},
};

pub async fn get_keys_for<T: StateType>(
    state: &mut DenimState<T>,
    account_id: AccountId,
    device_id: DeviceId,
) -> Result<KeyBundle, LogicError> {
    let pre_key = state
        .keys
        .pre_keys
        .get_ec_pre_key(account_id, device_id)
        .await?
        .encode()
        .map_err(|err| {
            error!("{err}");
            LogicError::Encode
        })?;

    let signed_pre_key = state
        .keys
        .signed_pre_keys
        .get_signed_pre_key(account_id, device_id)
        .await
        .map_err(DenimKeyManagerError::from)?
        .encode()
        .map_err(|err| {
            error!("{err}");
            LogicError::Encode
        })?;

    let device = state.devices.get_device(account_id, device_id).await?;

    Ok(KeyBundle::builder()
        .device_id(*device_id)
        .registration_id(*device.registration_id())
        .pre_key(pre_key)
        .signed_pre_key(signed_pre_key)
        .build())
}

pub async fn update_signed_pre_key<T: StateType>(
    state: &mut DenimState<T>,
    account_id: AccountId,
    device_id: DeviceId,
    signed_pre_key: SignedEcPreKey,
) -> Result<(), LogicError> {
    state
        .keys
        .signed_pre_keys
        .set_signed_pre_key(
            account_id,
            device_id,
            state.accounts.get_account(account_id).await?.identity(),
            signed_pre_key,
        )
        .await
        .map_err(DenimKeyManagerError::from)?;
    Ok(())
}

pub async fn update_seed<T: StateType>(
    state: &mut DenimState<T>,
    account_id: AccountId,
    device_id: DeviceId,
    key_seed: KeySeed,
    key_id_seed: KeyIdSeed,
) -> Result<(), LogicError> {
    state
        .keys
        .pre_keys
        .store_key_id_seed_for(account_id, device_id, key_id_seed.into())
        .await?;
    state
        .keys
        .pre_keys
        .store_key_seed_for(account_id, device_id, key_seed.into())
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use denim_sam_common::rng::seed::KeySeed;
    use libsignal_protocol::IdentityKeyPair;
    use rand::{rngs::OsRng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use sam_common::{
        address::DEFAULT_DEVICE_ID,
        api::{Decode as _, EcPreKey, Key},
        AccountId,
    };
    use sam_security::key_gen::generate_signed_pre_key;
    use sam_server::{
        auth::password::Password,
        managers::{
            entities::{Account, Device},
            traits::{
                account_manager::AccountManager as _, device_manager::DeviceManager as _,
                key_manager::SignedPreKeyManager as _,
            },
        },
    };

    use crate::{
        error::LogicError,
        logic::keys::get_keys_for,
        managers::{error::DenimKeyManagerError, DenimEcPreKeyManager},
        state::{DenimState, InMemoryStateType},
    };

    #[tokio::test]
    async fn get_keybundle() {
        let mut state =
            DenimState::<InMemoryStateType>::in_memory_test("127.0.0.1:8000".to_owned());
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
            .id(DEFAULT_DEVICE_ID.into())
            .name("Alice Secret Phone".to_string())
            .password(
                Password::generate("dave<3".to_string(), &mut rng)
                    .expect("Alice can create password"),
            )
            .registration_id(1.into())
            .build();

        let account_id = account.id();
        let device_id = DEFAULT_DEVICE_ID.into();
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
            .set_signed_pre_key(
                account_id,
                device_id,
                pair.identity_key(),
                signed_pre_key.into(),
            )
            .await
            .expect("Can set signed pre key");

        let alice_key_rng = ChaCha20Rng::from_rng(rng).expect("Can create RNG");

        state
            .keys
            .pre_keys
            .store_key_seed_for(account_id, device_id, alice_key_rng.into())
            .await
            .expect("Can store key csprng");

        let alice_id_rng = ChaCha20Rng::from_rng(rng).expect("Can create RNG");

        state
            .keys
            .pre_keys
            .store_key_id_seed_for(account_id, device_id, alice_id_rng.into())
            .await
            .expect("Can store id csprng");

        // testing if we get keys
        let bundle = get_keys_for(&mut state, account_id, device_id)
            .await
            .expect("User have uploaded bundles");

        assert!(bundle.device_id == DEFAULT_DEVICE_ID);
        assert!(bundle.registration_id == 1);
    }

    /// Tests that a NoSeed error is returned if you have not updated your seed.
    #[tokio::test]
    async fn update_seed() {
        let mut state =
            DenimState::<InMemoryStateType>::in_memory_test("127.0.0.1:8000".to_owned());
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
            .id(DEFAULT_DEVICE_ID.into())
            .name("Alice Secret Phone".to_string())
            .password(
                Password::generate("dave<3".to_string(), &mut rng)
                    .expect("Alice can create password"),
            )
            .registration_id(1.into())
            .build();

        let account_id = account.id();
        let device_id = DEFAULT_DEVICE_ID.into();
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
            .set_signed_pre_key(
                account_id,
                device_id,
                pair.identity_key(),
                signed_pre_key.into(),
            )
            .await
            .expect("Can set signed pre key");

        // Can't build a bundle without a seed
        assert!(get_keys_for(&mut state, account_id, device_id)
            .await
            .inspect_err(|err| println!("{err}"))
            .is_err_and(|err| matches!(err, LogicError::KeyManager(DenimKeyManagerError::NoSeed))));

        let alice_key_rng = ChaCha20Rng::from_rng(rng).expect("Can create RNG");

        state
            .keys
            .pre_keys
            .store_key_seed_for(account_id, device_id, alice_key_rng.into())
            .await
            .expect("Can store key csprng");

        let alice_id_rng = ChaCha20Rng::from_rng(rng).expect("Can create RNG");

        state
            .keys
            .pre_keys
            .store_key_id_seed_for(account_id, device_id, alice_id_rng.into())
            .await
            .expect("Can store id csprng");

        // testing if we get keys
        let bundle = get_keys_for(&mut state, account_id, device_id)
            .await
            .expect("User have uploaded bundles");

        // Now, pre keys should have been generated.
        assert!(state
            .keys
            .pre_keys
            .get_ec_pre_key(account_id, device_id)
            .await
            .is_ok());

        assert!(bundle.device_id == DEFAULT_DEVICE_ID);
        assert!(bundle.registration_id == 1);
    }

    #[tokio::test]
    async fn key_generation_is_reproducible() {
        let mut state =
            DenimState::<InMemoryStateType>::in_memory_test("127.0.0.1:8000".to_owned());
        let mut rng = OsRng;
        let alice_pair = IdentityKeyPair::generate(&mut rng);
        let bob_pair = IdentityKeyPair::generate(&mut rng);

        let alice = Account::builder()
            .id(AccountId::generate())
            .identity(*alice_pair.identity_key())
            .username("Alice".to_string())
            .build();

        let bob = Account::builder()
            .id(AccountId::generate())
            .identity(*bob_pair.identity_key())
            .username("Bob".to_string())
            .build();

        state
            .accounts
            .add_account(&alice)
            .await
            .expect("Can add alice");

        state.accounts.add_account(&bob).await.expect("Can add bob");

        let device_id = DEFAULT_DEVICE_ID.into();

        let alice_device = Device::builder()
            .id(device_id)
            .name("Alice Secret Phone".to_string())
            .password(
                Password::generate("dave<3".to_string(), &mut rng)
                    .expect("Alice can create password"),
            )
            .registration_id(1.into())
            .build();

        let bob_device = Device::builder()
            .id(device_id)
            .name("Bob Secret Phone".to_string())
            .password(
                Password::generate("dave<3".to_string(), &mut rng)
                    .expect("Bob can create password"),
            )
            .registration_id(1.into())
            .build();

        state
            .devices
            .add_device(alice.id(), &alice_device)
            .await
            .expect("Alice can add device");

        state
            .devices
            .add_device(bob.id(), &bob_device)
            .await
            .expect("Bob can add device");

        let alice_signed_pre_key =
            generate_signed_pre_key(22.into(), alice_pair.private_key(), &mut rng)
                .await
                .expect("Can generate Alice's Signed Pre Key");

        let bob_signed_pre_key =
            generate_signed_pre_key(22.into(), bob_pair.private_key(), &mut rng)
                .await
                .expect("Can generate Bob's Signed Pre Key");

        state
            .keys
            .signed_pre_keys
            .set_signed_pre_key(
                alice.id(),
                device_id,
                alice_pair.identity_key(),
                alice_signed_pre_key.into(),
            )
            .await
            .expect("Can set signed pre key");

        state
            .keys
            .signed_pre_keys
            .set_signed_pre_key(
                bob.id(),
                device_id,
                bob_pair.identity_key(),
                bob_signed_pre_key.into(),
            )
            .await
            .expect("Can set signed pre key");

        let seed = KeySeed::random(&mut OsRng);

        let alice_key_rng = ChaCha20Rng::from_seed(*seed);
        state
            .keys
            .pre_keys
            .store_key_seed_for(alice.id(), device_id, alice_key_rng.into())
            .await
            .expect("Can store key csprng");

        let alice_id_rng = ChaCha20Rng::from_seed(*seed);

        state
            .keys
            .pre_keys
            .store_key_id_seed_for(alice.id(), device_id, alice_id_rng.into())
            .await
            .expect("Can store id csprng");

        let bob_key_rng = ChaCha20Rng::from_seed(*seed);

        state
            .keys
            .pre_keys
            .store_key_seed_for(alice.id(), device_id, bob_key_rng.into())
            .await
            .expect("Can store key csprng");

        let bob_id_rng = ChaCha20Rng::from_seed(*seed);

        state
            .keys
            .pre_keys
            .store_key_id_seed_for(alice.id(), device_id, bob_id_rng.into())
            .await
            .expect("Can store id csprng");

        for _ in 0..=20 {
            let alice_key = EcPreKey::decode(
                get_keys_for(&mut state, alice.id(), device_id)
                    .await
                    .expect("Can get Alice's keys")
                    .pre_key
                    .as_slice(),
            )
            .expect("Can decode Alice's pre key");

            let bob_key = EcPreKey::decode(
                get_keys_for(&mut state, alice.id(), device_id)
                    .await
                    .expect("Can get Bob's keys")
                    .pre_key
                    .as_slice(),
            )
            .expect("Can decode Bob's pre key");

            assert!(alice_key.public_key == bob_key.public_key);

            state
                .keys
                .pre_keys
                .remove_ec_pre_key(alice.id(), device_id, alice_key.id())
                .await
                .expect("Can remove Alice's ec pre key");

            state
                .keys
                .pre_keys
                .remove_ec_pre_key(bob.id(), device_id, bob_key.id())
                .await
                .expect("Can remove Bob's ec pre key");
        }
    }
}
