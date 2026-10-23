//! URL validation and query encoding for TheAudioDB API.
//!
//! Everything here is pure: it never touches the network and never depends on
//! the plugin bindings, so it is unit-tested on the host target.

/// Base URL of TheAudioDB's v1 JSON API.
pub const DEFAULT_BASE_URL: &str = "https://www.theaudiodb.com/api/v1/json/";

/// API key TheAudioDB publishes for evaluation.
pub const DEFAULT_API_KEY: &str = "123";

/// Returns whether `value` is an absolute `http` or `https` URL.
///
/// The check is deliberately structural rather than a full URL parse: it
/// accepts exactly the shapes a TheAudioDB base URL can have (the public API, a
/// mirror, or a test double on a private host), and rejects everything a
/// request could not be built from.
///
/// # Parameters
///
/// - `value`: the candidate URL.
///
/// # Returns
///
/// `true` when `value` starts with `http://` or `https://`, carries a non-empty
/// host, and contains no whitespace.
///
/// # Example
///
/// ```text
/// is_http_url("https://www.theaudiodb.com/api/v1/json/") -> true
/// is_http_url("http://localhost:8080/api")               -> true
/// is_http_url("www.theaudiodb.com/api/v1/json/")         -> false
/// is_http_url("ftp://www.theaudiodb.com")                -> false
/// ```
pub fn is_http_url(value: &str) -> bool {
    let after_scheme = match value.strip_prefix("https://") {
        Some(rest) => rest,
        None => match value.strip_prefix("http://") {
            Some(rest) => rest,
            None => return false,
        },
    };
    let host = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();

    !host.is_empty() && !host.contains(char::is_whitespace)
}

/// Normalizes a configured base URL.
///
/// Surrounding whitespace and trailing slashes are removed, then exactly one
/// trailing slash is added, so the API key and endpoint the plugin appends land
/// directly under the configured base.
///
/// # Parameters
///
/// - `value`: the configured base URL.
///
/// # Returns
///
/// The trimmed URL with exactly one trailing slash.
///
/// # Example
///
/// ```text
/// normalize_base_url(" https://www.theaudiodb.com/api/v1/json/ ") -> "https://www.theaudiodb.com/api/v1/json/"
/// normalize_base_url("https://mirror.example/api")               -> "https://mirror.example/api/"
/// ```
pub fn normalize_base_url(value: &str) -> String {
    format!("{}/", value.trim().trim_end_matches('/'))
}

/// Joins a base URL, the API key and an API path into an endpoint URL.
///
/// TheAudioDB authenticates with the key as a URL path segment, so every
/// endpoint is `<base>/<key>/<endpoint>`. The key is percent-encoded, and a
/// leading slash on the path is dropped so the parts do not double up.
///
/// # Parameters
///
/// - `base_url`: the configured base URL, with or without a trailing slash.
/// - `api_key`: the API key sent in the path.
/// - `path`: the endpoint, with or without a leading slash, already
///   query-encoded.
///
/// # Returns
///
/// The absolute URL of the endpoint.
///
/// # Example
///
/// ```text
/// endpoint("https://www.theaudiodb.com/api/v1/json/", "123", "search.php?s=Coldplay")
///   -> "https://www.theaudiodb.com/api/v1/json/123/search.php?s=Coldplay"
/// endpoint("https://mirror.example/api", "abc", "/artist-mb.php?i=x")
///   -> "https://mirror.example/api/abc/artist-mb.php?i=x"
/// ```
pub fn endpoint(base_url: &str, api_key: &str, path: &str) -> String {
    format!(
        "{}{}/{}",
        normalize_base_url(base_url),
        encode_component(api_key),
        path.trim_start_matches('/')
    )
}

/// Percent-encodes one query or path component.
///
/// Everything outside the RFC 3986 unreserved set is escaped, including `/`,
/// `&`, `=`, `?` and `#`, so a search text or an identifier cannot add
/// parameters or change the endpoint it is pasted into. A space becomes `%20`
/// rather than `+`.
///
/// # Parameters
///
/// - `value`: the component to encode.
///
/// # Returns
///
/// The encoded component.
///
/// # Example
///
/// ```text
/// encode_component("Daft Punk")        -> "Daft%20Punk"
/// encode_component("AC/DC & friends")  -> "AC%2FDC%20%26%20friends"
/// ```
pub fn encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(*byte));
            }
            _ => {
                encoded.push('%');
                encoded.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
                encoded.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
            }
        }
    }

    encoded
}

/// Uppercase hexadecimal digits used by [`encode_component`].
const HEX_DIGITS: [u8; 16] = *b"0123456789ABCDEF";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_http_and_https_urls() {
        assert!(is_http_url("https://www.theaudiodb.com/api/v1/json/"));
        assert!(is_http_url("http://localhost:8080"));
        assert!(is_http_url("https://mirror.example.org/theaudiodb/"));
    }

    #[test]
    fn rejects_urls_without_a_host_or_scheme() {
        assert!(!is_http_url("www.theaudiodb.com/api/v1/json/"));
        assert!(!is_http_url("ftp://www.theaudiodb.com"));
        assert!(!is_http_url("https://"));
        assert!(!is_http_url("https:///api"));
        assert!(!is_http_url("http://the audio db"));
    }

    #[test]
    fn normalization_removes_surrounding_space_and_adds_one_trailing_slash() {
        assert_eq!(
            normalize_base_url("  https://x.example/api/ "),
            "https://x.example/api/"
        );
        assert_eq!(
            normalize_base_url("https://x.example/api"),
            "https://x.example/api/"
        );
        assert_eq!(
            normalize_base_url("https://x.example///"),
            "https://x.example/"
        );
    }

    #[test]
    fn endpoint_places_the_api_key_in_the_path() {
        assert_eq!(
            endpoint(
                "https://www.theaudiodb.com/api/v1/json",
                "123",
                "search.php?s=Coldplay"
            ),
            "https://www.theaudiodb.com/api/v1/json/123/search.php?s=Coldplay"
        );
        assert_eq!(
            endpoint("https://mirror.example/api/", "abc", "/artist-mb.php?i=x"),
            "https://mirror.example/api/abc/artist-mb.php?i=x"
        );
    }

    #[test]
    fn encoding_escapes_reserved_characters() {
        assert_eq!(encode_component("Daft Punk"), "Daft%20Punk");
        assert_eq!(
            encode_component("AC/DC & friends"),
            "AC%2FDC%20%26%20friends"
        );
        assert_eq!(encode_component("a.b_c-d~e"), "a.b_c-d~e");
    }

    #[test]
    fn the_default_base_url_is_the_v1_json_api() {
        assert_eq!(
            normalize_base_url(DEFAULT_BASE_URL),
            "https://www.theaudiodb.com/api/v1/json/"
        );
        assert_eq!(DEFAULT_API_KEY, "123");
    }
}
