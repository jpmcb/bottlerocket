//! The aws module implements the `PlatformDataProvider` trait for gathering userdata on AWS.

use super::{PlatformDataProvider, SettingsJson};
use crate::compression::expand_slice_maybe;
use async_trait::async_trait;
use imdsclient::ImdsClient;
use serde_json::json;
use snafu::{OptionExt, ResultExt};
use std::fs;
use std::path::Path;

use crate::provider::local_file::{local_file_user_data, USER_DATA_FILE};

/// Unit struct for AWS so we can implement the PlatformDataProvider trait.
pub(crate) struct AwsDataProvider;

impl AwsDataProvider {
    const IDENTITY_DOCUMENT_FILE: &'static str = "/etc/early-boot-config/identity-document";

    /// Fetches user data, which is expected to be in TOML form and contain a `[settings]` section,
    /// returning a SettingsJson representing the inside of that section.
    async fn user_data(client: &mut ImdsClient) -> Result<Option<SettingsJson>> {
        let user_data_raw = match client
            .fetch_userdata()
            .await
            .context(error::ImdsRequestSnafu)?
        {
            Some(user_data_raw) => user_data_raw,
            None => return Ok(None),
        };
        let user_data_str = expand_slice_maybe(&user_data_raw)
            .context(error::DecompressionSnafu { what: "user data" })?;
        trace!("Received user data: {}", user_data_str);

        let json = SettingsJson::from_toml_str(&user_data_str, "user data").context(
            error::SettingsToJSONSnafu {
                from: "instance user data",
            },
        )?;
        Ok(Some(json))
    }

    /// Fetches the instance identity, returning a SettingsJson representing the values from the
    /// document which we'd like to send to the API - currently just region.
    async fn identity_document(client: &mut ImdsClient) -> Result<Option<SettingsJson>> {
        let desc = "instance identity document";
        let file = Self::IDENTITY_DOCUMENT_FILE;

        let region = if Path::new(file).exists() {
            info!("{} found at {}, using it", desc, file);
            let data =
                fs::read_to_string(file).context(error::InputFileReadSnafu { path: file })?;
            let iid: serde_json::Value =
                serde_json::from_str(&data).context(error::DeserializeJsonSnafu)?;
            iid.get("region")
                .context(error::IdentityDocMissingDataSnafu { missing: "region" })?
                .as_str()
                .context(error::WrongTypeSnafu {
                    field_name: "region",
                    expected_type: "string",
                })?
                .to_owned()
        } else {
            client
                .fetch_region()
                .await
                .context(error::ImdsRequestSnafu)?
                .context(error::ImdsMissingRegionSnafu)?
        };
        trace!(
            "Retrieved region from instance identity document: {}",
            region
        );

        let val = json!({ "aws": {"region": region} });

        let json = SettingsJson::from_val(&val, desc).context(error::SettingsToJSONSnafu {
            from: "instance identity document",
        })?;
        Ok(Some(json))
    }
}

#[async_trait]
impl PlatformDataProvider for AwsDataProvider {
    /// Return settings changes from the instance identity document and user data.
    async fn platform_data(
        &self,
    ) -> std::result::Result<Vec<SettingsJson>, Box<dyn std::error::Error>> {
        let mut output = Vec::new();

        let mut client = ImdsClient::new();

        // Attempt to read from local file first
        match local_file_user_data()? {
            Some(s) => output.push(s),
            None => warn!("No user data found via local file: {}", USER_DATA_FILE),
        }

        // Instance identity doc next, so the user has a chance to override
        match Self::identity_document(&mut client).await? {
            Some(s) => output.push(s),
            None => warn!("No instance identity document found."),
        }

        // Optional user-specified configuration / overrides
        match Self::user_data(&mut client).await? {
            Some(s) => output.push(s),
            None => warn!("No user data found."),
        }

        Ok(output)
    }
}

mod error {
    use snafu::Snafu;
    use std::io;
    use std::path::PathBuf;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(crate) enum Error {
        #[snafu(display("Failed to decompress {}: {}", what, source))]
        Decompression { what: String, source: io::Error },

        #[snafu(display("Error deserializing from JSON: {}", source))]
        DeserializeJson { source: serde_json::error::Error },

        #[snafu(display("Instance identity document missing {}", missing))]
        IdentityDocMissingData { missing: String },

        #[snafu(display("IMDS client failed: {}", source))]
        ImdsClient { source: imdsclient::Error },

        #[snafu(display("Unable to read input file '{}': {}", path.display(), source))]
        InputFileRead { path: PathBuf, source: io::Error },

        #[snafu(display("IMDS request failed: missing region"))]
        ImdsMissingRegion {},

        #[snafu(display("IMDS request failed: {}", source))]
        ImdsRequest { source: imdsclient::Error },

        #[snafu(display("Unable to serialize settings from {}: {}", from, source))]
        SettingsToJSON {
            from: String,
            source: crate::settings::Error,
        },

        #[snafu(display(
            "Wrong type while deserializing, expected '{}' to be type '{}'",
            field_name,
            expected_type
        ))]
        WrongType {
            field_name: &'static str,
            expected_type: &'static str,
        },
    }
}

type Result<T> = std::result::Result<T, error::Error>;
