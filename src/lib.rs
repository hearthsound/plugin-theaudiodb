//! TheAudioDB plugin: a Hearthsound metadata provider over TheAudioDB's public
//! v1 JSON API.
//!
//! The plugin needs no account, and the public test key `123` is enough to
//! evaluate it; a premium key only raises the request limit and the size of a
//! result page. It implements `lifecycle` (configuration validation and a
//! health probe) and `metadata` (artist information).
//!
//! `get-artist-info` looks the artist up by the MusicBrainz id the server holds
//! and falls back to an artist-name search when that id finds nothing. It
//! reports the English biography and the reference links the row carries
//! (homepage, Wikipedia, Wikidata, Last.fm, Discogs, AllMusic). TheAudioDB has
//! no recording index, so `resolve-recording` answers no matches and
//! `get-release` answers not-found, which lets the server ask the next
//! metadata instance.
//!
//! Two instance settings point the plugin at its service: `baseUrl`, which
//! defaults to the public v1 JSON API, and `apiKey`, which defaults to the
//! public test key. Either can be changed per instance.
//!
//! The pure logic sits in public modules ([`url`], [`audiodb`], [`map`]); the
//! binding-coupled glue that drives it - the HTTP transport and the metadata
//! operations - stays private.
//!
//! # Tests
//!
//! The pure logic and the lookup policy are unit-tested on the host target,
//! against JSON fixtures captured from the public API:
//!
//! ```text
//! cargo test
//! ```
//!
//! Rustdoc examples in those modules are illustrative `text` blocks rather than
//! doctests: the crate is a `cdylib` for `wasm32-wasip2`, so there is no host
//! library target for a doctest to link against. Every example appears as an
//! assertion in the unit tests of the module that carries it.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use hearthsound_plugin::metadata_provider::exports::hearthsound::plugin::lifecycle::Guest as LifecycleGuest;
use hearthsound_plugin::metadata_provider::exports::hearthsound::plugin::metadata::{
    ArtistInfo, ArtistInfoRequest, Guest as MetadataGuest, RecordingHint, RecordingMatch,
};
use hearthsound_plugin::types::{Album, IdPair, ProviderError};

mod api;
pub mod audiodb;
pub mod map;
mod metadata;
pub mod url;

/// The plugin implementation.
struct TheAudioDb;

/// Server-scoped configuration of the plugin.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    /// API key sent in the request path.
    #[serde(default = "default_api_key")]
    api_key: String,
    /// Base URL of the v1 JSON API.
    #[serde(default = "default_base_url")]
    base_url: String,
}

/// Returns the API key used when the instance configuration omits one.
fn default_api_key() -> String {
    url::DEFAULT_API_KEY.to_string()
}

/// Returns the base URL used when the instance configuration omits one.
fn default_base_url() -> String {
    url::DEFAULT_BASE_URL.to_string()
}

/// Checks a configured base URL and returns it normalized.
///
/// # Parameters
///
/// - `value`: the configured URL.
///
/// # Returns
///
/// The URL, trimmed and with exactly one trailing slash.
///
/// # Errors
///
/// Returns a message when the URL is not an absolute `http` or `https` URL.
fn check_base_url(value: &str) -> Result<String, String> {
    let normalized = url::normalize_base_url(value);

    if url::is_http_url(&normalized) {
        Ok(normalized)
    } else {
        Err(format!(
            "baseUrl must be an http or https URL, got `{value}`"
        ))
    }
}

/// Checks a configured API key.
///
/// # Parameters
///
/// - `value`: the configured key.
///
/// # Returns
///
/// The trimmed key.
///
/// # Errors
///
/// Returns a message when the key is blank, because TheAudioDB would answer
/// every request with a not-found page.
fn check_api_key(value: &str) -> Result<String, String> {
    let key = value.trim();

    if key.is_empty() {
        Err("apiKey must not be empty".to_string())
    } else {
        Ok(key.to_string())
    }
}

/// Reads this instance's configuration and builds its API client.
///
/// # Returns
///
/// The client over the configured base URL and API key.
///
/// # Errors
///
/// Returns [`ProviderError::InvalidConfig`] when the configuration cannot be
/// read or deserialized, when the base URL is not an absolute `http` or
/// `https` URL, and when the API key is blank.
fn api() -> Result<api::HttpApi, ProviderError> {
    let config: Config = hearthsound_plugin::config::server_config()?;
    let base_url = check_base_url(&config.base_url).map_err(ProviderError::InvalidConfig)?;
    let api_key = check_api_key(&config.api_key).map_err(ProviderError::InvalidConfig)?;

    Ok(api::HttpApi::new(base_url, api_key))
}

/// Validates the plugin's server-scoped configuration.
///
/// # Parameters
///
/// - `server_config`: the configuration JSON as the administrator stored it.
///
/// # Returns
///
/// `Ok(())` when the configuration is usable.
///
/// # Errors
///
/// Returns one message per problem: JSON that does not deserialize into the
/// configuration, a blank API key, and a base URL that is not an absolute
/// `http` or `https` URL.
fn validate_config(server_config: &str) -> Result<(), Vec<String>> {
    let config: Config = match serde_json::from_str(server_config) {
        Ok(config) => config,
        Err(error) => return Err(vec![format!("server config: {error}")]),
    };

    let mut errors = Vec::new();

    if let Err(error) = check_api_key(&config.api_key) {
        errors.push(error);
    }
    if let Err(error) = check_base_url(&config.base_url) {
        errors.push(error);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Implements the lifecycle interface.
impl LifecycleGuest for TheAudioDb {
    fn validate_config(server_config: String) -> Result<(), Vec<String>> {
        validate_config(&server_config)
    }

    fn health() -> Result<String, ProviderError> {
        metadata::health(&api()?)?;

        Ok("TheAudioDB API reachable".to_string())
    }
}

/// Implements the metadata interface.
impl MetadataGuest for TheAudioDb {
    /// Answers that TheAudioDB resolves no recordings.
    ///
    /// # Parameters
    ///
    /// - `hint`: the recording hint to resolve.
    ///
    /// # Returns
    ///
    /// An empty list, so the server asks the next metadata instance.
    ///
    /// # Errors
    ///
    /// This implementation never fails.
    fn resolve_recording(hint: RecordingHint) -> Result<Vec<RecordingMatch>, ProviderError> {
        hearthsound_plugin::debug!(
            "resolve-recording {:?} with {} artists",
            hint.title,
            hint.artists.len()
        );

        metadata::resolve_recording(&hint)
    }

    /// Answers that TheAudioDB materializes no releases.
    ///
    /// # Parameters
    ///
    /// - `ids`: the identifiers of the release to materialize.
    ///
    /// # Returns
    ///
    /// This implementation always answers not-found.
    ///
    /// # Errors
    ///
    /// Always returns [`ProviderError::NotFound`].
    fn get_release(ids: Vec<IdPair>) -> Result<Album, ProviderError> {
        hearthsound_plugin::debug!("get-release for {} ids", ids.len());

        metadata::get_release(&ids)
    }

    /// Looks the requested artist up in TheAudioDB.
    ///
    /// # Parameters
    ///
    /// - `request`: the artist the server wants information on, named by an
    ///   `mb-artist` id, a name, or both.
    ///
    /// # Returns
    ///
    /// The artist's identifiers, English biography and reference links, or
    /// `None` when neither the id nor the name found an artist.
    ///
    /// # Errors
    ///
    /// Returns the provider error of the request that failed.
    fn get_artist_info(request: ArtistInfoRequest) -> Result<Option<ArtistInfo>, ProviderError> {
        hearthsound_plugin::debug!(
            "get-artist-info for {:?} with {} ids",
            request.name,
            request.ids.len()
        );

        metadata::artist_info(&api()?, &request)
    }
}

hearthsound_plugin::export_metadata_provider!(TheAudioDb);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_valid_configuration() {
        assert_eq!(validate_config("{}"), Ok(()));
        assert_eq!(
            validate_config(r#"{"baseUrl":"https://www.theaudiodb.com/api/v1/json/"}"#),
            Ok(())
        );
        assert_eq!(
            validate_config(r#"{"apiKey":"premium","baseUrl":"http://audiodb.internal:8080/"}"#),
            Ok(())
        );
    }

    #[test]
    fn rejects_a_base_url_that_is_not_http() {
        let errors =
            validate_config(r#"{"baseUrl":"www.theaudiodb.com/api/v1/json/"}"#).unwrap_err();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("baseUrl"), "{}", errors[0]);
    }

    #[test]
    fn rejects_a_blank_api_key() {
        let errors = validate_config(r#"{"apiKey":"  "}"#).unwrap_err();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("apiKey"), "{}", errors[0]);
    }

    #[test]
    fn reports_every_unusable_setting_at_once() {
        let errors =
            validate_config(r#"{"apiKey":"","baseUrl":"www.theaudiodb.com"}"#).unwrap_err();

        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("apiKey"), "{}", errors[0]);
        assert!(errors[1].contains("baseUrl"), "{}", errors[1]);
    }

    #[test]
    fn rejects_configuration_of_an_unexpected_shape() {
        let errors = validate_config(r#"{"baseUrl":7}"#).unwrap_err();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("server config"), "{}", errors[0]);
    }

    #[test]
    fn the_defaults_are_the_public_api_and_test_key() {
        assert_eq!(default_api_key(), "123");
        assert_eq!(
            default_base_url(),
            "https://www.theaudiodb.com/api/v1/json/"
        );
        assert_eq!(url::DEFAULT_API_KEY, default_api_key());
        assert_eq!(url::DEFAULT_BASE_URL, default_base_url());
    }
}
