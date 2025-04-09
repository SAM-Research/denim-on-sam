use bon::bon;
use denim_sam_common::buffers::{InMemoryReceivingBuffer, InMemorySendingBuffer};
use denim_sam_common::denim_message::deniable_message::MessageKind;
use denim_sam_common::denim_message::{BlockRequest, KeyRequest};
use libsignal_protocol::{IdentityKeyPair, IdentityKeyStore};
use rand::rngs::OsRng;
use rand::{CryptoRng, Rng};
use sam_client::encryption::DecryptedEnvelope;

use sam_common::{address::AccountId, address::RegistrationId, api::LinkDeviceToken, DeviceId};

use sam_client::logic::{handle_message_response, prepare_message, provision_device};

use sam_client::net::HttpClient;
use sam_client::storage::{
    InMemoryStoreType, MessageStore, SqliteStoreType, Store, StoreConfig, StoreType,
};
use sam_client::{
    logic::{publish_prekeys, register_account},
    net::{api_trait::ApiClientConfig, ApiClient},
    storage::AccountStore,
};
use tokio::sync::broadcast::Receiver;

use crate::deniable_store::inmem::InMemoryDeniableStoreType;
use crate::deniable_store::{DeniableStore, DeniableStoreConfig, DeniableStoreType};
use crate::error::DenimClientError;
use crate::protocol::{
    denim_client::{DenimProtocolClient, DenimSamClient},
    DenimProtocolConfig,
};
use crate::receiver::SamDenimMessage;
use tokio::sync::mpsc::Receiver as MpscReceiver;

pub trait DenimClientType {
    type Store: StoreType;
    type DeniableStore: DeniableStoreType;
    type ApiClient: ApiClient;
    type ProtocolClient: DenimSamClient;
    type Rng: Rng + CryptoRng + Default;
}

pub struct DefaultDenimClientType<
    T: StoreType,
    U: ApiClient,
    V: DenimSamClient,
    D: DeniableStoreType,
> {
    _store: std::marker::PhantomData<T>,
    _api: std::marker::PhantomData<U>,
    _protocol: std::marker::PhantomData<V>,
    _deniable_store: std::marker::PhantomData<D>,
}

impl<T: StoreType, U: ApiClient, V: DenimSamClient, D: DeniableStoreType> DenimClientType
    for DefaultDenimClientType<T, U, V, D>
{
    type Store = T;

    type DeniableStore = D;

    type ApiClient = U;

    type ProtocolClient = V;

    type Rng = OsRng;
}

pub type InMemoryDenimClientType = DefaultDenimClientType<
    InMemoryStoreType,
    HttpClient,
    DenimProtocolClient<InMemorySendingBuffer, InMemoryReceivingBuffer>,
    InMemoryDeniableStoreType,
>;
pub type SqliteDenimClientType = DefaultDenimClientType<
    SqliteStoreType,
    HttpClient,
    DenimProtocolClient<InMemorySendingBuffer, InMemoryReceivingBuffer>,
    InMemoryDeniableStoreType,
>;

pub struct DenimClient<T: DenimClientType> {
    store: Store<T::Store>,
    _deniable_store: DeniableStore<T::DeniableStore>,
    api_client: T::ApiClient,
    protocol_client: T::ProtocolClient,
    envelope_queue: MpscReceiver<SamDenimMessage>,
    rng: T::Rng,
}

#[bon]
impl<T: DenimClientType> DenimClient<T> {
    #[builder]
    pub async fn from_provisioning(
        store_config: impl StoreConfig<StoreType = T::Store>,
        deniable_store_config: impl DeniableStoreConfig<DeniableStoreType = T::DeniableStore>,
        api_client_config: impl ApiClientConfig<ApiClient = T::ApiClient>,
        protocol_config: impl DenimProtocolConfig<ProtocolClient = T::ProtocolClient>,
        device_name: &str,
        id_key_pair: IdentityKeyPair,
        token: LinkDeviceToken,
        #[builder(default = 100)] upload_prekey_count: usize,
        #[builder(default = 16)] password_length: usize,
        #[builder(default = <T::Rng as Default>::default())] mut rng: T::Rng,
    ) -> Result<Self, DenimClientError> {
        let api_client = api_client_config.create().await?;
        let registration_id = RegistrationId::generate(&mut rng);

        let mut store = store_config
            .create_store(id_key_pair, registration_id)
            .await?;

        provision_device(
            &api_client,
            &mut store,
            device_name,
            token,
            upload_prekey_count,
            password_length,
            &mut rng,
        )
        .await?;

        let deniable_store = deniable_store_config.create_store().await?;

        let mut protocol_client = protocol_config.create(
            store.account_store.get_account_id().await?,
            store.account_store.get_device_id().await?,
            store.account_store.get_password().await?,
        )?;

        let queue = protocol_client.connect().await?;

        Ok(Self {
            store,
            _deniable_store: deniable_store,
            api_client,
            protocol_client,
            envelope_queue: queue,
            rng,
        })
    }

    /// Register a new account.
    #[builder]
    pub async fn from_registration(
        store_config: impl StoreConfig<StoreType = T::Store>,
        deniable_store_config: impl DeniableStoreConfig<DeniableStoreType = T::DeniableStore>,
        api_client_config: impl ApiClientConfig<ApiClient = T::ApiClient>,
        protocol_config: impl DenimProtocolConfig<ProtocolClient = T::ProtocolClient>,
        username: &str,
        device_name: &str,
        #[builder(default = 100)] upload_prekey_count: usize,
        #[builder(default = 16)] password_length: usize,
        #[builder(default = <T::Rng as Default>::default())] mut rng: T::Rng,
    ) -> Result<Self, DenimClientError> {
        let registration_id = RegistrationId::generate(&mut rng);
        let id_key_pair = IdentityKeyPair::generate(&mut rng);
        let mut store = store_config
            .create_store(id_key_pair, registration_id)
            .await?;
        let api_client = api_client_config.create().await?;

        register_account(
            &api_client,
            &mut store,
            username,
            device_name,
            password_length,
            upload_prekey_count,
            &mut rng,
        )
        .await?;

        let deniable_store = deniable_store_config.create_store().await?;

        let mut protocol_client = protocol_config.create(
            store.account_store.get_account_id().await?,
            store.account_store.get_device_id().await?,
            store.account_store.get_password().await?,
        )?;

        let queue = protocol_client.connect().await?;

        Ok(Self {
            store,
            _deniable_store: deniable_store,
            api_client,
            rng,
            protocol_client,
            envelope_queue: queue,
        })
    }

    /// Instantiate a client from a valid store.
    #[builder]
    pub async fn from_stores(
        store: Store<T::Store>,
        deniable_store: DeniableStore<T::DeniableStore>,
        api_client_config: impl ApiClientConfig<ApiClient = T::ApiClient>,
        protocol_config: impl DenimProtocolConfig<ProtocolClient = T::ProtocolClient>,
        #[builder(default = <T::Rng as Default>::default())] rng: T::Rng,
    ) -> Result<Self, DenimClientError> {
        let mut protocol_client = protocol_config.create(
            store.account_store.get_account_id().await?,
            store.account_store.get_device_id().await?,
            store.account_store.get_password().await?,
        )?;

        let queue = protocol_client.connect().await?;

        Ok(Self {
            store,
            _deniable_store: deniable_store,
            api_client: api_client_config.create().await?,
            protocol_client,
            envelope_queue: queue,
            rng,
        })
    }

    pub async fn account_id(&self) -> Result<AccountId, DenimClientError> {
        Ok(self.store.account_store.get_account_id().await?)
    }

    pub async fn device_id(&self) -> Result<DeviceId, DenimClientError> {
        Ok(self.store.account_store.get_device_id().await?)
    }

    pub async fn identity_key_pair(&self) -> Result<IdentityKeyPair, DenimClientError> {
        Ok(self
            .store
            .identity_key_store
            .get_identity_key_pair()
            .await?)
    }

    /// Delete Account and consumes the client.
    /// If account deletion fails, the client is returned along with the error.
    pub async fn delete_account(self) -> Result<(), (Self, DenimClientError)> {
        let account_id = self.account_id().await;
        let device_id = self.device_id().await;
        let password = self
            .store
            .account_store
            .get_password()
            .await
            .map_err(DenimClientError::Client);

        let Ok(account_id) = account_id else {
            return Err((self, account_id.unwrap_err()));
        };

        let Ok(device_id) = device_id else {
            return Err((self, device_id.unwrap_err()));
        };

        let Ok(password) = password else {
            return Err((self, password.unwrap_err()));
        };

        let delete_result = self
            .api_client
            .delete_account(account_id, device_id, &password)
            .await;

        let Ok(()) = delete_result else {
            return Err((self, DenimClientError::Api(delete_result.unwrap_err())));
        };

        Ok(())
    }

    /// Delete this device and consumes the client.
    /// This cannot be done for the primary device.
    ///
    /// See `unlink_device` if you want to delete another device.
    pub async fn delete_device(self) -> Result<(), (Self, DenimClientError)> {
        let account_id = self.account_id().await;
        let device_id = self.device_id().await;
        let password = self
            .store
            .account_store
            .get_password()
            .await
            .map_err(DenimClientError::Client);

        let Ok(account_id) = account_id else {
            return Err((self, account_id.unwrap_err()));
        };

        let Ok(device_id) = device_id else {
            return Err((self, device_id.unwrap_err()));
        };

        let Ok(password) = password else {
            return Err((self, password.unwrap_err()));
        };

        let delete_result = self
            .api_client
            .delete_device(account_id, device_id, &password, device_id)
            .await;

        let Ok(()) = delete_result else {
            return Err((self, DenimClientError::Api(delete_result.unwrap_err())));
        };

        Ok(())
    }

    /// Unlink another device from the client's account.
    /// This can only be done from the primary device.
    pub async fn unlink_device(self, device_id: DeviceId) -> Result<(), DenimClientError> {
        self.api_client
            .delete_device(
                self.account_id().await?,
                self.device_id().await?,
                &self.store.account_store.get_password().await?,
                device_id,
            )
            .await?;
        Ok(())
    }

    /// Get the [AccountId] of a user by username.
    pub async fn get_account_id_for(&self, username: &str) -> Result<AccountId, DenimClientError> {
        let account_id = self
            .api_client
            .get_user_account_id(
                self.account_id().await?,
                self.device_id().await?,
                self.store.account_store.get_password().await?.as_str(),
                username,
            )
            .await?;

        Ok(account_id)
    }

    /// Disconnect from the server.
    pub async fn disconnect(&mut self) -> Result<(), DenimClientError> {
        Ok(self.protocol_client.disconnect().await?)
    }

    /// Connect to the server to recieve messages.
    pub async fn connect(&mut self) -> Result<(), DenimClientError> {
        self.envelope_queue = self.protocol_client.connect().await?;
        Ok(())
    }

    /// Returns whether or not the client is connected to the server.
    pub async fn is_connected(&self) -> bool {
        self.protocol_client.is_connected().await
    }

    /// Send any message to recipient. Also sends syncs the message with your other devices.
    pub async fn send_message(
        &mut self,
        recipient: AccountId,
        msg: impl Into<Vec<u8>>,
    ) -> Result<(), DenimClientError> {
        let client_envelope = prepare_message(
            &mut self.store,
            &self.api_client,
            recipient,
            msg,
            &mut self.rng,
        )
        .await?;
        let status = self.protocol_client.send_message(client_envelope).await?;
        handle_message_response(&mut self.store, &self.api_client, &mut self.rng, status).await?;
        Ok(())
    }

    /// Returns a broadcast receiver for incoming messages that have been decrypted.
    pub fn subscribe(&self) -> Receiver<DecryptedEnvelope> {
        self.store.message_store.subscribe()
    }

    /// Recieve and decrypt messages. Block until at least one message is received.
    pub async fn process_messages_blocking(&mut self) -> Result<(), DenimClientError> {
        // TODO: implement
        Ok(())
    }

    /// Recieve and decrypt messages.
    pub async fn process_messages(&mut self) -> Result<(), DenimClientError> {
        // TODO: implement
        Ok(())
    }

    /// Publish new prekeys.
    #[builder]
    pub async fn publish_prekeys(
        &mut self,
        #[builder(default)] onetime_prekeys: usize,
        #[builder(default = false)] new_signed_prekey: bool,
        #[builder(default = false)] new_last_resort: bool,
    ) -> Result<(), DenimClientError> {
        publish_prekeys(
            &mut self.store,
            &self.api_client,
            onetime_prekeys,
            new_signed_prekey,
            new_last_resort,
            &mut self.rng,
        )
        .await
        .map_err(DenimClientError::Client)
    }

    /// Create a provisioning token for linking a new device to your account.
    pub async fn create_provision(&mut self) -> Result<LinkDeviceToken, DenimClientError> {
        Ok(self
            .api_client
            .provision_device(
                self.account_id().await?,
                self.device_id().await?,
                &self.store.account_store.get_password().await?,
            )
            .await?)
    }

    pub async fn block_user(&mut self, user: AccountId) {
        let block_user_message = BlockRequest::builder().account_id(user.to_string()).build();

        self.protocol_client
            .enqueue_deniable(MessageKind::BlockRequest(block_user_message))
            .await;
    }

    pub async fn fetch_keys(&mut self, user: AccountId) {
        let key_request = KeyRequest::builder()
            .account_id(Vec::from(user.as_bytes()))
            .specific_device_ids(vec![1])
            .build();

        self.protocol_client
            .enqueue_deniable(MessageKind::KeyRequest(key_request))
            .await;
    }
}
