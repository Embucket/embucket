use crate::error::{DynamoDbCredentialsSnafu, Error, Result};
use aws_config::Region;
use aws_config::meta::credentials::CredentialsProviderChain;
use aws_config::{BehaviorVersion, defaults};
use aws_credential_types::Credentials;
use aws_credential_types::provider::{ProvideCredentials, SharedCredentialsProvider};
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::config::Builder as DynamoConfigBuilder;
use aws_sdk_dynamodb::config::retry::RetryConfig;
use snafu::ResultExt;
use std::env;

#[derive(Debug, Clone)]
pub struct DynamoDbConfig {
    pub table_name: String,
    pub endpoint: Option<String>,
    pub region: Option<String>,
}

impl DynamoDbConfig {
    pub fn from_env() -> Result<Self> {
        let table_name = required_env("STATESTORE_TABLE_NAME")?;
        let endpoint = env::var("STATESTORE_DYNAMODB_ENDPOINT").ok();
        let region = env::var("AWS_REGION").ok();

        Ok(Self {
            table_name,
            endpoint,
            region,
        })
    }

    pub async fn client(&self) -> Result<Client> {
        let mut loader = defaults(BehaviorVersion::latest());

        if let Some(region) = &self.region {
            loader = loader.region(Region::new(region.clone()));
        }

        if let Some(endpoint) = &self.endpoint {
            loader = loader.endpoint_url(endpoint);
        }

        let creds = if let (Ok(access_key), Ok(secret_key)) = (
            env::var("AWS_DDB_ACCESS_KEY_ID"),
            env::var("AWS_DDB_SECRET_ACCESS_KEY"),
        ) {
            let token = env::var("AWS_DDB_SESSION_TOKEN").ok();
            Credentials::from_keys(access_key, secret_key, token)
        } else {
            // Default AWS Credential Provider Chain
            // Resolution order:
            // 1. Environment variables
            // 2. Shared config (`~/.aws/config`, `~/.aws/credentials`)
            // 3. Web Identity Tokens
            // 4. ECS (IAM Roles for Tasks) & General HTTP credentials
            // 5. EC2 IMDSv2
            let provider = CredentialsProviderChain::default_provider().await;
            provider
                .provide_credentials()
                .await
                .context(DynamoDbCredentialsSnafu)?
        };

        loader = loader.credentials_provider(SharedCredentialsProvider::new(creds));
        let config = loader.load().await;
        let retry_config = RetryConfig::adaptive();
        let config_builder = DynamoConfigBuilder::from(&config).retry_config(retry_config);

        Ok(Client::from_conf(config_builder.build()))
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name).map_err(|_| Error::MissingEnvVar {
        reason: name.to_string(),
    })
}
