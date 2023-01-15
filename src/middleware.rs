use async_trait::async_trait;
use cognitoauth::cognito_srp_auth::{auth, CognitoAuthInput};
use ethers::{
    core::types::{transaction::eip2718::TypedTransaction, BlockId, U256},
    providers::{FromErr, Middleware, PendingTransaction},
    types::{Bytes, NameOrAddress, H256},
    utils::__serde_json::{json, Value},
};
use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::Mutex;

// Same for every project, taken from here: https://docs.openzeppelin.com/defender/api-auth
const RELAYER_URL: &str = "https://api.defender.openzeppelin.com/txs";
const CLIENT_ID: &str = "1bpd19lcr33qvg5cr3oi79rdap";
const POOL_ID: &str = "us-west-2_iLmIggsiy";

use std::{collections::HashMap, str::FromStr};
lazy_static::lazy_static! {
    static ref TOKENS: Mutex<HashMap<String, Token>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Serialize)]
struct Transaction<'a> {
    to: Option<&'a NameOrAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<&'a U256>,
    #[serde(rename = "gasLimit", skip_serializing_if = "Option::is_none")]
    gas_limit: Option<&'a U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<&'a Bytes>,
}

#[derive(Debug, Clone)]
struct RelayerConfig {
    api_key: String,
    api_secret: String,
}

#[derive(Debug)]
struct Token {
    access_token: String,
    expiration_time: u64,
}

/// Refreshes or creates a new access token for Defender API and returns it.
async fn refresh_token(config: RelayerConfig) -> eyre::Result<String> {
    let now = chrono::Utc::now().timestamp() as u64;
    let mut tokens = TOKENS.lock().await;
    if let Some(token) = tokens.get(&config.api_key.clone()) {
        if now < token.expiration_time {
            // token still valid
            return Ok(token.access_token.to_string());
        }
    }

    let input = CognitoAuthInput {
        client_id: CLIENT_ID.to_string(),
        pool_id: POOL_ID.to_string(),
        username: config.api_key.clone(),
        password: config.api_secret.clone(),
        mfa: None,
        client_secret: None,
    };

    let res = auth(input)
        .await
        .map_err(|_| eyre::eyre!("Authentication failed"))?
        .ok_or(eyre::eyre!("Authentication failed"))?;

    let access_token = res
        .access_token()
        .ok_or(eyre::eyre!("Authentication failed"))?;

    tokens.insert(
        config.api_key.clone(),
        Token {
            access_token: access_token.to_string(),
            expiration_time: now + res.expires_in() as u64,
        },
    );
    Ok(access_token.to_string())
}

#[derive(Debug)]
pub struct OzRelayerMiddleware<M> {
    inner: M,
    config: RelayerConfig,
}

impl<M> OzRelayerMiddleware<M>
where
    M: Middleware,
{
    pub fn new(
        inner: M,
        api_key: String,
        api_secret: String,
    ) -> Result<Self, OzRelayerMiddlewareError<M>> {
        let config = RelayerConfig {
            api_key,
            api_secret,
        };
        Ok(Self { inner, config })
    }
}

#[async_trait]
impl<M> Middleware for OzRelayerMiddleware<M>
where
    M: Middleware,
{
    type Error = OzRelayerMiddlewareError<M>;
    type Provider = M::Provider;
    type Inner = M;

    fn inner(&self) -> &M {
        &self.inner
    }

    async fn send_transaction<T: Into<TypedTransaction> + Send + Sync>(
        &self,
        tx: T,
        _: Option<BlockId>,
    ) -> Result<PendingTransaction<'_, Self::Provider>, Self::Error> {
        let tx: TypedTransaction = tx.into();

        // refresh token if necessary
        let token = refresh_token(self.config.clone())
            .await
            .map_err(|_| OzRelayerMiddlewareError::<M>::AuthenticationError())?;

        let api_tx = Transaction {
            to: tx.to(),
            value: tx.value(),
            gas_limit: tx.gas(),
            data: tx.data(),
        };

        let client = reqwest::Client::new();
        let res = client
            .post(RELAYER_URL)
            .header("X-Api-Key", self.config.api_key.clone())
            .header("Authorization", format!("Bearer {}", token))
            .body(json!(api_tx).to_string())
            .send()
            .await
            .map_err(|_| OzRelayerMiddlewareError::<M>::AuthenticationError())?;

        match res.status() {
            StatusCode::OK => {
                let obj = res
                    .json::<Value>()
                    .await
                    .map_err(|_| OzRelayerMiddlewareError::<M>::UnknownResponse())?;
                let id = obj
                    .get("transactionId")
                    .ok_or(OzRelayerMiddlewareError::<M>::UnknownResponse())?
                    .as_str()
                    .unwrap();
                Ok(PendingTransaction::new(
                    H256::from_str(format!("{:0>64}", &id.replace('-', "")).as_str()).unwrap(),
                    self.provider(),
                ))
            }
            _ => {
                return Err(OzRelayerMiddlewareError::<M>::AuthenticationError());
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum OzRelayerMiddlewareError<M: Middleware> {
    /// Thrown when the internal middleware errors
    #[error("{0}")]
    MiddlewareError(M::Error),
    #[error("Authentication error")]
    AuthenticationError(),
    #[error("Unknown response")]
    UnknownResponse(),
}

impl<M: Middleware> FromErr<M::Error> for OzRelayerMiddlewareError<M> {
    fn from(src: M::Error) -> Self {
        OzRelayerMiddlewareError::MiddlewareError(src)
    }
}
