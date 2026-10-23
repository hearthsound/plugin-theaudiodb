//! HTTP access to TheAudioDB through the `host-http` host import.
//!
//! This is the only module that touches the network. It builds request paths,
//! sends them, and turns a response into either decoded rows or a provider
//! error. The operations that decide what to do with those rows live in
//! [`crate::metadata`], and the conversion into provider records lives in
//! [`crate::map`].
//!
//! The API key is a path segment rather than a header, so every URL is
//! `<base>/<key>/<endpoint>`; [`crate::url::endpoint`] builds that shape and
//! percent-encodes the key.
//!
//! The service reports ordinary failed lookups as HTTP 200 with a `null`
//! collection, not as an HTTP error. A 404 therefore means the key or the
//! endpoint is not served, a 503 means the shared service is down, and a 429
//! means the request rate is above what the key allows; each is mapped to the
//! provider error that lets the server react correctly.

use serde::de::DeserializeOwned;

use hearthsound_plugin::host::http::Response;
use hearthsound_plugin::http;
use hearthsound_plugin::types::ProviderError;

use crate::audiodb::{Album, AlbumsResponse, Artist, ArtistsResponse};
use crate::url;

/// Wall-clock budget for one TheAudioDB request, in milliseconds.
///
/// The host clamps this to its own ceiling and to the remaining call deadline.
const REQUEST_TIMEOUT_MS: u32 = 10_000;

/// Delay reported with a rate-limited error when the response carries no
/// `Retry-After` header, in milliseconds.
///
/// The free key allows 30 requests per minute, so a delay of half a minute is
/// long enough for the window to move on without making a single rate limit
/// cost the caller a whole call deadline.
const DEFAULT_RETRY_AFTER_MS: u32 = 30_000;

/// Number of characters of a response body kept in an error message.
///
/// A response body is at most a few hundred kilobytes, but an error string only
/// needs enough of it for a human to recognize the failure.
const ERROR_BODY_CHARS: usize = 200;

/// The artist operations the plugin needs.
///
/// The trait exists so the lookup policy can be tested without a network or a
/// WebAssembly runtime: production code uses [`HttpApi`], and the unit tests
/// use a fake that answers the rows a fixture holds.
pub(crate) trait TheAudioDbApi {
    /// Returns the artist with a MusicBrainz id.
    ///
    /// # Parameters
    ///
    /// - `mbid`: the MusicBrainz artist id.
    ///
    /// # Returns
    ///
    /// The matching rows, or an empty list when the service knows no such
    /// artist.
    ///
    /// # Errors
    ///
    /// Returns the provider error of a request that failed.
    fn artist_by_mbid(&self, mbid: &str) -> Result<Vec<Artist>, ProviderError>;

    /// Searches artists by name.
    ///
    /// # Parameters
    ///
    /// - `name`: the artist name to search for.
    ///
    /// # Returns
    ///
    /// The rows the search answered with, most relevant first.
    ///
    /// # Errors
    ///
    /// Returns the provider error of a request that failed.
    fn search_artists(&self, name: &str) -> Result<Vec<Artist>, ProviderError>;

    /// Returns the artist with a TheAudioDB id.
    ///
    /// # Parameters
    ///
    /// - `id`: the TheAudioDB artist id.
    ///
    /// # Returns
    ///
    /// The matching rows, or an empty list when the service knows no such
    /// artist.
    ///
    /// # Errors
    ///
    /// Returns the provider error of a request that failed.
    fn artist_by_id(&self, id: &str) -> Result<Vec<Artist>, ProviderError>;

    /// Returns the album with a MusicBrainz release group id.
    ///
    /// # Parameters
    ///
    /// - `mbid`: the MusicBrainz release group id.
    ///
    /// # Returns
    ///
    /// The matching rows, or an empty list when the service knows no such
    /// album.
    ///
    /// # Errors
    ///
    /// Returns the provider error of a request that failed.
    fn album_by_release_group(&self, mbid: &str) -> Result<Vec<Album>, ProviderError>;

    /// Returns the album with a TheAudioDB id.
    ///
    /// # Parameters
    ///
    /// - `id`: the TheAudioDB album id.
    ///
    /// # Returns
    ///
    /// The matching rows, or an empty list when the service knows no such
    /// album.
    ///
    /// # Errors
    ///
    /// Returns the provider error of a request that failed.
    fn album_by_id(&self, id: &str) -> Result<Vec<Album>, ProviderError>;

    /// Searches albums by artist and title.
    ///
    /// # Parameters
    ///
    /// - `artist`: the album artist.
    /// - `album`: the album title.
    ///
    /// # Returns
    ///
    /// The rows the search answered with, most relevant first.
    ///
    /// # Errors
    ///
    /// Returns the provider error of a request that failed.
    fn search_albums(&self, artist: &str, album: &str) -> Result<Vec<Album>, ProviderError>;
}

/// The production implementation, over the instance's base URL and API key.
pub(crate) struct HttpApi {
    /// Base URL of the v1 JSON API, normalized with a trailing slash.
    base_url: String,
    /// API key sent as the first path segment.
    api_key: String,
}

impl HttpApi {
    /// Builds an API client for one instance.
    ///
    /// # Parameters
    ///
    /// - `base_url`: base URL of the v1 JSON API.
    /// - `api_key`: API key sent in the path.
    ///
    /// # Returns
    ///
    /// The client.
    pub(crate) fn new(base_url: String, api_key: String) -> Self {
        Self { base_url, api_key }
    }
}

impl TheAudioDbApi for HttpApi {
    fn artist_by_mbid(&self, mbid: &str) -> Result<Vec<Artist>, ProviderError> {
        let response: ArtistsResponse =
            get_json(&self.base_url, &self.api_key, &artist_mbid_path(mbid))?;

        Ok(response.into_artists())
    }

    fn search_artists(&self, name: &str) -> Result<Vec<Artist>, ProviderError> {
        let response: ArtistsResponse =
            get_json(&self.base_url, &self.api_key, &artist_search_path(name))?;

        Ok(response.into_artists())
    }

    fn artist_by_id(&self, id: &str) -> Result<Vec<Artist>, ProviderError> {
        let response: ArtistsResponse = get_json(&self.base_url, &self.api_key, &artist_path(id))?;

        Ok(response.into_artists())
    }

    fn album_by_release_group(&self, mbid: &str) -> Result<Vec<Album>, ProviderError> {
        let response: AlbumsResponse = get_json(
            &self.base_url,
            &self.api_key,
            &album_release_group_path(mbid),
        )?;

        Ok(response.into_albums())
    }

    fn album_by_id(&self, id: &str) -> Result<Vec<Album>, ProviderError> {
        let response: AlbumsResponse = get_json(&self.base_url, &self.api_key, &album_path(id))?;

        Ok(response.into_albums())
    }

    fn search_albums(&self, artist: &str, album: &str) -> Result<Vec<Album>, ProviderError> {
        let response: AlbumsResponse = get_json(
            &self.base_url,
            &self.api_key,
            &album_search_path(artist, album),
        )?;

        Ok(response.into_albums())
    }
}

/// Builds the path of an artist lookup by MusicBrainz id.
///
/// # Parameters
///
/// - `mbid`: the MusicBrainz artist id.
///
/// # Returns
///
/// `artist-mb.php?i=<encoded id>`.
///
/// # Example
///
/// ```text
/// artist_mbid_path("cc197bad-dc9c-440d-a5b5-d52ba2e14234")
///   -> "artist-mb.php?i=cc197bad-dc9c-440d-a5b5-d52ba2e14234"
/// ```
pub(crate) fn artist_mbid_path(mbid: &str) -> String {
    format!("artist-mb.php?i={}", url::encode_component(mbid.trim()))
}

/// Builds the path of an artist search.
///
/// # Parameters
///
/// - `name`: the artist name to search for.
///
/// # Returns
///
/// `search.php?s=<encoded name>`.
///
/// # Example
///
/// ```text
/// artist_search_path("Daft Punk") -> "search.php?s=Daft%20Punk"
/// ```
pub(crate) fn artist_search_path(name: &str) -> String {
    format!("search.php?s={}", url::encode_component(name.trim()))
}

/// Builds the path of an artist lookup by TheAudioDB id.
///
/// # Parameters
///
/// - `id`: the TheAudioDB artist id.
///
/// # Returns
///
/// `artist.php?i=<encoded id>`.
///
/// # Example
///
/// ```text
/// artist_path("111239") -> "artist.php?i=111239"
/// ```
pub(crate) fn artist_path(id: &str) -> String {
    format!("artist.php?i={}", url::encode_component(id.trim()))
}

/// Builds the path of an album lookup by MusicBrainz release group id.
///
/// # Parameters
///
/// - `mbid`: the MusicBrainz release group id.
///
/// # Returns
///
/// `album-mb.php?i=<encoded id>`.
///
/// # Example
///
/// ```text
/// album_release_group_path("1dc4c347-a1db-32aa-b14f-bc9cc507b843")
///   -> "album-mb.php?i=1dc4c347-a1db-32aa-b14f-bc9cc507b843"
/// ```
pub(crate) fn album_release_group_path(mbid: &str) -> String {
    format!("album-mb.php?i={}", url::encode_component(mbid.trim()))
}

/// Builds the path of an album lookup by TheAudioDB id.
///
/// # Parameters
///
/// - `id`: the TheAudioDB album id.
///
/// # Returns
///
/// `album.php?m=<encoded id>`.
///
/// # Example
///
/// ```text
/// album_path("2115888") -> "album.php?m=2115888"
/// ```
pub(crate) fn album_path(id: &str) -> String {
    format!("album.php?m={}", url::encode_component(id.trim()))
}

/// Builds the path of an album search.
///
/// # Parameters
///
/// - `artist`: the album artist.
/// - `album`: the album title.
///
/// # Returns
///
/// `searchalbum.php?s=<encoded artist>&a=<encoded title>`.
///
/// # Example
///
/// ```text
/// album_search_path("Daft Punk", "Homework")
///   -> "searchalbum.php?s=Daft%20Punk&a=Homework"
/// ```
pub(crate) fn album_search_path(artist: &str, album: &str) -> String {
    format!(
        "searchalbum.php?s={}&a={}",
        url::encode_component(artist.trim()),
        url::encode_component(album.trim())
    )
}

/// Sends a GET request to an endpoint and decodes the JSON body.
///
/// # Parameters
///
/// - `base_url`: base URL of the v1 JSON API.
/// - `api_key`: API key sent in the path.
/// - `path`: endpoint path, already query-encoded.
///
/// # Returns
///
/// The decoded response body.
///
/// # Errors
///
/// Returns the [`status_error`] of a non-`2xx` response,
/// [`ProviderError::Unavailable`] when the host cannot perform the request at
/// all, and [`ProviderError::Internal`] when the body is not JSON for `T`.
fn get_json<T: DeserializeOwned>(
    base_url: &str,
    api_key: &str,
    path: &str,
) -> Result<T, ProviderError> {
    let endpoint = url::endpoint(base_url, api_key, path);

    let mut request = http::get(&endpoint);
    request.timeout_ms = Some(REQUEST_TIMEOUT_MS);

    let response = http::send(&request)?;

    decode(&response, &endpoint)
}

/// Decodes one TheAudioDB response.
///
/// # Parameters
///
/// - `response`: the response to decode.
/// - `url`: the request URL, used in error messages.
///
/// # Returns
///
/// The decoded body.
///
/// # Errors
///
/// Returns the [`status_error`] of a non-`2xx` response, and
/// [`ProviderError::Internal`] when a successful response is not JSON for `T`.
fn decode<T: DeserializeOwned>(response: &Response, url: &str) -> Result<T, ProviderError> {
    if !http::is_success(response) {
        return Err(status_error(response, url));
    }

    serde_json::from_slice(&response.body)
        .map_err(|error| ProviderError::Internal(format!("{url}: {error}")))
}

/// Maps a non-success response to a provider error.
///
/// # Parameters
///
/// - `response`: a response whose status is not `2xx`.
/// - `url`: the request URL, used in the messages.
///
/// # Returns
///
/// [`ProviderError::NotFound`] for `404` (an unrecognized key or an endpoint
/// the shared service does not serve), [`ProviderError::RateLimited`] for `429`
/// (the `Retry-After` header in milliseconds, or
/// [`DEFAULT_RETRY_AFTER_MS`]), [`ProviderError::Unauthorized`] for `401` and
/// `403`, [`ProviderError::Unavailable`] for every `5xx` including TheAudioDB's
/// `503`, and [`ProviderError::Internal`] for anything else - a `400`, for
/// example, means the plugin built a query the service rejects.
fn status_error(response: &Response, url: &str) -> ProviderError {
    match response.status {
        404 => ProviderError::NotFound,
        429 => {
            ProviderError::RateLimited(retry_after_ms(response).unwrap_or(DEFAULT_RETRY_AFTER_MS))
        }
        401 | 403 => ProviderError::Unauthorized(describe(response, url)),
        status if (500..600).contains(&status) => {
            ProviderError::Unavailable(describe(response, url))
        }
        _ => ProviderError::Internal(describe(response, url)),
    }
}

/// Renders a status line and an excerpt of the body for an error message.
///
/// # Parameters
///
/// - `response`: the response to describe.
/// - `url`: the request URL.
///
/// # Returns
///
/// `<url> answered HTTP <status>`, followed by `: <excerpt>` when the body is
/// not empty.
fn describe(response: &Response, url: &str) -> String {
    let excerpt = body_excerpt(response);

    if excerpt.is_empty() {
        format!("{url} answered HTTP {}", response.status)
    } else {
        format!("{url} answered HTTP {}: {excerpt}", response.status)
    }
}

/// Returns the body of a response as a short, printable excerpt.
///
/// # Parameters
///
/// - `response`: the response to read.
///
/// # Returns
///
/// The body decoded as UTF-8 with invalid sequences replaced, whitespace
/// collapsed, and truncated to [`ERROR_BODY_CHARS`] characters.
fn body_excerpt(response: &Response) -> String {
    let text = String::from_utf8_lossy(&response.body);
    let mut excerpt: String = text
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(ERROR_BODY_CHARS)
        .collect();

    if text.chars().count() > ERROR_BODY_CHARS {
        excerpt.push_str("...");
    }

    excerpt
}

/// Reads the `Retry-After` header as a delay in milliseconds.
///
/// # Parameters
///
/// - `response`: the response to read the header from.
///
/// # Returns
///
/// The header value in seconds times 1000, saturated to `u32`, or `None` when
/// the header is absent or is not a number of seconds.
fn retry_after_ms(response: &Response) -> Option<u32> {
    let value = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("retry-after"))
        .map(|(_, value)| value.trim())?;

    value
        .parse::<u32>()
        .ok()
        .map(|seconds| seconds.saturating_mul(1_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a response with the given status, headers and body.
    fn response(status: u16, headers: &[(&str, &str)], body: &str) -> Response {
        Response {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            body: body.as_bytes().to_vec(),
        }
    }

    #[test]
    fn builds_the_artist_paths() {
        assert_eq!(
            artist_mbid_path("cc197bad-dc9c-440d-a5b5-d52ba2e14234"),
            "artist-mb.php?i=cc197bad-dc9c-440d-a5b5-d52ba2e14234"
        );
        assert_eq!(artist_search_path("Daft Punk"), "search.php?s=Daft%20Punk");
        assert_eq!(artist_path("111239"), "artist.php?i=111239");
        assert_eq!(
            album_release_group_path("1dc4c347-a1db-32aa-b14f-bc9cc507b843"),
            "album-mb.php?i=1dc4c347-a1db-32aa-b14f-bc9cc507b843"
        );
        assert_eq!(album_path("2115888"), "album.php?m=2115888");
        assert_eq!(
            album_search_path("Daft Punk", "Homework"),
            "searchalbum.php?s=Daft%20Punk&a=Homework"
        );
    }

    #[test]
    fn decodes_an_album_response() {
        let decoded: AlbumsResponse = decode(
            &response(200, &[], include_str!("fixtures/album_mb.json")),
            "https://x",
        )
        .unwrap();

        assert_eq!(decoded.into_albums().len(), 1);
    }

    #[test]
    fn decodes_a_successful_response() {
        let decoded: ArtistsResponse = decode(
            &response(200, &[], include_str!("fixtures/artist_mb.json")),
            "https://x",
        )
        .unwrap();

        assert_eq!(decoded.into_artists().len(), 1);
    }

    #[test]
    fn a_missing_document_is_not_found() {
        assert!(matches!(
            decode::<serde_json::Value>(
                &response(404, &[], r#"{"Message":"Not found"}"#),
                "https://x"
            )
            .unwrap_err(),
            ProviderError::NotFound
        ));
    }

    #[test]
    fn a_rate_limited_response_carries_the_retry_delay() {
        assert!(matches!(
            decode::<serde_json::Value>(&response(429, &[("retry-after", "5")], ""), "https://x")
                .unwrap_err(),
            ProviderError::RateLimited(5_000)
        ));
        assert!(matches!(
            decode::<serde_json::Value>(&response(429, &[], ""), "https://x").unwrap_err(),
            ProviderError::RateLimited(DEFAULT_RETRY_AFTER_MS)
        ));
    }

    #[test]
    fn an_unavailable_service_is_unavailable() {
        let error = decode::<serde_json::Value>(
            &response(503, &[], r#"{"Message":"Service temporarily unavailable"}"#),
            "https://x",
        )
        .unwrap_err();

        match error {
            ProviderError::Unavailable(message) => {
                assert!(message.contains("HTTP 503"), "{message}");
                assert!(message.contains("temporarily unavailable"), "{message}");
            }
            _ => panic!("expected an unavailable error"),
        }
    }

    #[test]
    fn a_rejected_request_is_an_internal_error() {
        assert!(matches!(
            decode::<serde_json::Value>(&response(400, &[], ""), "https://x").unwrap_err(),
            ProviderError::Internal(_)
        ));
    }

    #[test]
    fn a_denied_request_is_unauthorized() {
        assert!(matches!(
            decode::<serde_json::Value>(&response(403, &[], "denied"), "https://x").unwrap_err(),
            ProviderError::Unauthorized(_)
        ));
    }

    #[test]
    fn a_body_that_is_not_json_is_an_internal_error() {
        let error =
            decode::<serde_json::Value>(&response(200, &[], "<html>"), "https://x").unwrap_err();

        assert!(matches!(error, ProviderError::Internal(_)));
    }

    #[test]
    fn an_excerpt_is_short_and_printable() {
        let long = "x".repeat(500);
        let excerpt = body_excerpt(&response(500, &[], &long));

        assert_eq!(excerpt.chars().count(), ERROR_BODY_CHARS + 3);
        assert!(excerpt.ends_with("..."));

        let whitespace = body_excerpt(&response(500, &[], "a\n  b\tc"));
        assert_eq!(whitespace, "a b c");
    }
}
