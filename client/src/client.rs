use bon::bon;
use libsignal_protocol::{IdentityKeyPair, IdentityKeyStore};
use rand::rngs::OsRng;
use rand::{CryptoRng, Rng};
use sam_common::{address::AccountId, address::RegistrationId, api::LinkDeviceToken, DeviceId};

use sam_client::logic::provision_device;

use sam_client::net::HttpClient;
use sam_client::storage::inmem::InMemorySignalStoreType;
use sam_client::storage::sqlite::{SqliteSamStoreType, SqliteSignalStoreType};
use sam_client::storage::{InMemorySamStoreType, SamStoreConfig, SignalStore, SignalStoreType};
use sam_client::{
    logic::{publish_prekeys, register_account},
    net::{api_trait::ApiClientConfig, ApiClient},
    storage::{AccountStore, SamStore, SamStoreType, SignalStoreConfig},
    ClientError,
};

use crate::error::DenimClientError;

pub trait DenimClientType {
    type RegularStore: SignalStoreType;
    type DenimStore: SignalStoreType;
    type SamStore: SamStoreType;
    type ApiClient: ApiClient;
    type Rng: Rng + CryptoRng + Default;
}

pub struct DefaultDenimClientType<T: SamStoreType, V: SignalStoreType, U: ApiClient> {
    _reg_store: std::marker::PhantomData<T>,
    _den_store: std::marker::PhantomData<V>,
    _api: std::marker::PhantomData<U>,
    //_protocol: std::marker::PhantomData<V>,
}

impl<T: SamStoreType, V: SignalStoreType, U: ApiClient> DenimClientType
    for DefaultDenimClientType<T, V, U>
{
    type RegularStore = V;

    type DenimStore = V;

    type SamStore = T;

    type ApiClient = U;

    type Rng = OsRng;
}

pub type InMemoryDenimClientType =
    DefaultDenimClientType<InMemorySamStoreType, InMemorySignalStoreType, HttpClient>;
pub type SqliteDenimClientType =
    DefaultDenimClientType<SqliteSamStoreType, SqliteSignalStoreType, HttpClient>;

pub struct DenimClient<T: DenimClientType> {
    regular_store: SignalStore<T::RegularStore>,
    _denim_store: SignalStore<T::DenimStore>,
    sam_store: SamStore<T::SamStore>,
    api_client: T::ApiClient,
    rng: T::Rng,
}

#[bon]
impl<T: DenimClientType> DenimClient<T> {
    #[builder]
    pub async fn from_provisioning(
        sam_store_config: impl SamStoreConfig<StoreType = T::SamStore>,
        regular_store_config: impl SignalStoreConfig<StoreType = T::RegularStore>,
        denim_store_config: impl SignalStoreConfig<StoreType = T::DenimStore>,
        api_client_config: impl ApiClientConfig<ApiClient = T::ApiClient>,
        device_name: &str,
        id_key_pair: IdentityKeyPair,
        token: LinkDeviceToken,
        #[builder(default = 100)] upload_prekey_count: usize,
        #[builder(default = 16)] password_length: usize,
        #[builder(default = <T::Rng as Default>::default())] mut rng: T::Rng,
    ) -> Result<Self, DenimClientError> {
        let api_client = api_client_config.create().await?;
        let registration_id = RegistrationId::generate(&mut rng);

        let mut sam_store = sam_store_config.create_store().await?;

        let mut regular_store = regular_store_config
            .create_store(id_key_pair, registration_id)
            .await?;

        provision_device()
            .api_client(&api_client)
            .signal_store(&mut regular_store)
            .sam_store(&mut sam_store)
            .device_name(device_name)
            .token(token)
            .upload_prekey_count(upload_prekey_count)
            .password_length(password_length)
            .rng(&mut rng)
            .call()
            .await?;

        let denim_store = denim_store_config
            .create_store(id_key_pair, registration_id)
            .await?;

        // TODO: DenimProtocolClient connect

        Ok(Self {
            sam_store,
            regular_store,
            _denim_store: denim_store,
            api_client,
            rng,
        })
    }

    /// Register a new account.
    #[builder]
    pub async fn from_registration(
        sam_store_config: impl SamStoreConfig<StoreType = T::SamStore>,
        regular_store_config: impl SignalStoreConfig<StoreType = T::RegularStore>,
        denim_store_config: impl SignalStoreConfig<StoreType = T::DenimStore>,
        api_client_config: impl ApiClientConfig<ApiClient = T::ApiClient>,
        username: &str,
        device_name: &str,

        #[builder(default = 100)] upload_prekey_count: usize,
        #[builder(default = 16)] password_length: usize,
        #[builder(default = <T::Rng as Default>::default())] mut rng: T::Rng,
    ) -> Result<Self, DenimClientError> {
        let registration_id = RegistrationId::generate(&mut rng);
        let id_key_pair = IdentityKeyPair::generate(&mut rng);
        let mut sam_store = sam_store_config.create_store().await?;
        let mut regular_store = regular_store_config
            .create_store(id_key_pair, registration_id)
            .await?;
        let api_client = api_client_config.create().await?;

        register_account()
            .api_client(&api_client)
            .sam_store(&mut sam_store)
            .signal_store(&mut regular_store)
            .username(username)
            .device_name(device_name)
            .password_length(password_length)
            .upload_prekey_count(upload_prekey_count)
            .rng(&mut rng)
            .call()
            .await?;

        let denim_store = denim_store_config
            .create_store(id_key_pair, registration_id)
            .await?;

        // TODO: DenimProtocolClient

        Ok(Self {
            regular_store,
            _denim_store: denim_store,
            sam_store,
            api_client,
            rng,
        })
    }

    /// Instantiate a client from a valid store.
    #[builder]
    pub async fn from_store(
        sam_store: SamStore<T::SamStore>,
        regular_store: SignalStore<T::RegularStore>,
        denim_store: SignalStore<T::DenimStore>,
        api_client_config: impl ApiClientConfig<ApiClient = T::ApiClient>,
        #[builder(default = <T::Rng as Default>::default())] rng: T::Rng,
    ) -> Result<Self, ClientError> {
        //TODO: Add DenimProtocolClient

        Ok(Self {
            sam_store,
            regular_store,
            _denim_store: denim_store,
            api_client: api_client_config.create().await?,
            rng,
        })
    }

    pub async fn account_id(&self) -> Result<AccountId, DenimClientError> {
        Ok(self.sam_store.account_store.get_account_id().await?)
    }

    pub async fn device_id(&self) -> Result<DeviceId, DenimClientError> {
        Ok(self.sam_store.account_store.get_device_id().await?)
    }

    pub async fn identity_key_pair(&self) -> Result<IdentityKeyPair, DenimClientError> {
        Ok(self
            .regular_store
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
            .sam_store
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
            .sam_store
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
                &self.sam_store.account_store.get_password().await?,
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
                self.sam_store.account_store.get_password().await?.as_str(),
                username,
            )
            .await?;

        Ok(account_id)
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
            &mut self.regular_store,
            &mut self.sam_store,
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
                &self.sam_store.account_store.get_password().await?,
            )
            .await?)
    }
}
