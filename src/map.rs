//! Conversion of TheAudioDB rows into provider records.
//!
//! TheAudioDB keeps a handful of identifier and profile columns per artist and
//! album. This module turns those columns into the `metadata` interface's
//! [`ArtistInfo`] and the `artwork` interface's images, leaving out every value
//! that is blank or cannot be turned into an absolute URL.
//!
//! The artist response carries a website and a Last.fm chart page, which the
//! shared WIT vocabulary names `homepage` and `lastfm`. It also carries
//! Facebook and Twitter columns, but the vocabulary has no kind for them, so
//! they are not reported.

use hearthsound_plugin::metadata_artwork_provider::exports::hearthsound::plugin::metadata::{
    ArtistInfo, ArtistLink,
};
use hearthsound_plugin::types::{IdPair, Image};

use crate::audiodb;

/// Id namespace of a MusicBrainz artist.
pub const NS_ARTIST: &str = "mb-artist";

/// Id namespace of a TheAudioDB artist or album.
pub const NS_AUDIODB: &str = "theaudiodb";

/// Id namespace of a MusicBrainz release group.
pub const NS_RELEASE_GROUP: &str = "mb-release-group";

/// Returns an id pair in one namespace.
///
/// # Parameters
///
/// - `namespace`: the namespace of the identifier.
/// - `value`: the identifier itself.
///
/// # Returns
///
/// The pair, ready to append to a record's `ids`.
///
/// # Example
///
/// ```text
/// id_pair(NS_ARTIST, "cc197bad-dc9c-440d-a5b5-d52ba2e14234")
///   -> IdPair { namespace: "mb-artist", value: "cc197bad-dc9c-440d-a5b5-d52ba2e14234" }
/// ```
pub fn id_pair(namespace: &str, value: &str) -> IdPair {
    IdPair {
        namespace: namespace.to_string(),
        value: value.to_string(),
    }
}

/// Converts an artist row into the profile the metadata interface returns.
///
/// # Parameters
///
/// - `artist`: the artist row.
///
/// # Returns
///
/// The artist's identifiers (`mb-artist` when the row carries one, then the
/// TheAudioDB id), the English biography when it is present and non-blank, and
/// the web links [`artist_links`] builds from the row's profile columns.
///
/// # Example
///
/// ```text
/// artist_info(&coldplay_row)
///   -> ArtistInfo { ids: [("mb-artist", "cc197bad-..."), ("theaudiodb", "111239")], biography: Some("Coldplay are ..."), links: [...] }
/// ```
pub fn artist_info(artist: &audiodb::Artist) -> ArtistInfo {
    let mut ids = Vec::with_capacity(2);

    if let Some(mbid) = artist.music_brainz_id() {
        ids.push(id_pair(NS_ARTIST, mbid));
    }
    if let Some(id) = artist.provider_id() {
        ids.push(id_pair(NS_AUDIODB, id));
    }

    ArtistInfo {
        ids,
        biography: artist.biography().map(str::to_string),
        links: artist_links(artist),
    }
}

/// Builds the web links of an artist row.
///
/// # Parameters
///
/// - `artist`: the artist row.
///
/// # Returns
///
/// One link per non-blank profile column that maps to a link kind of the WIT
/// vocabulary: `homepage` from `strWebsite` and `lastfm` from `strLastFMChart`.
/// A free-text URL without a scheme is given `https://`. Facebook and Twitter
/// columns are not reported because the vocabulary has no kind for them, and
/// the artist response carries no Wikipedia, Wikidata, Discogs or AllMusic
/// column.
///
/// # Example
///
/// ```text
/// artist_links(&row_with_website_and_lastfm)
///   -> [ArtistLink { kind: "homepage", url: "https://www.coldplay.com" },
///       ArtistLink { kind: "lastfm", url: "http://www.last.fm/music/Coldplay/+charts?rangetype=6month" }]
/// ```
pub fn artist_links(artist: &audiodb::Artist) -> Vec<ArtistLink> {
    let mut links = Vec::new();

    push_link(
        &mut links,
        "homepage",
        absolute_url(artist.str_website.as_deref()),
    );
    push_link(
        &mut links,
        "lastfm",
        absolute_url(artist.str_last_fm_chart.as_deref()),
    );

    links
}

/// Returns the value of an identifier in one namespace.
///
/// # Parameters
///
/// - `ids`: the identifiers to read.
/// - `namespace`: the namespace to look for.
///
/// # Returns
///
/// The trimmed value of the first identifier in that namespace, or `None` when
/// there is none or it is blank.
///
/// # Example
///
/// ```text
/// id_value(&[("mb-artist", "cc197bad-..."), ("theaudiodb", "111239")], "theaudiodb") -> Some("111239")
/// ```
pub(crate) fn id_value<'a>(ids: &'a [IdPair], namespace: &str) -> Option<&'a str> {
    ids.iter()
        .find(|pair| pair.namespace == namespace)
        .map(|pair| pair.value.trim())
        .filter(|value| !value.is_empty())
}

/// Builds the portrait renditions of an artist row.
///
/// # Parameters
///
/// - `artist`: the artist row.
///
/// # Returns
///
/// One image for the row's portrait, or an empty list when the row has no
/// portrait. TheAudioDB reports one portrait size per row, so the image carries
/// no width or height.
pub fn artist_images(artist: &audiodb::Artist) -> Vec<Image> {
    image(artist.thumb())
}

/// Builds the cover renditions of an album row.
///
/// # Parameters
///
/// - `album`: the album row.
///
/// # Returns
///
/// One image for the row's front cover, or an empty list when the row has no
/// cover. TheAudioDB reports one cover size per row, so the image carries no
/// width or height.
pub fn album_images(album: &audiodb::Album) -> Vec<Image> {
    image(album.thumb())
}

/// Builds a one-image list from an optional URL.
///
/// # Parameters
///
/// - `url`: the image URL, or `None` when the row has no picture.
///
/// # Returns
///
/// One image without dimensions, or an empty list.
fn image(url: Option<&str>) -> Vec<Image> {
    url.map(|url| {
        vec![Image {
            url: url.to_string(),
            width: None,
            height: None,
        }]
    })
    .unwrap_or_default()
}

/// Returns whether two names spell the same text.
///
/// Comparison ignores case and any difference in the amount or width of
/// whitespace, so `"The  Weeknd"` and `"the weeknd"` are the same name.
///
/// # Parameters
///
/// - `left`: the first name.
/// - `right`: the second name.
///
/// # Returns
///
/// `true` when the folded names are equal.
///
/// # Example
///
/// ```text
/// same_text("Coldplay", "coldplay") -> true
/// same_text("Coldplay", "Coldplay Tribute Band") -> false
/// ```
pub fn same_text(left: &str, right: &str) -> bool {
    normalize(left) == normalize(right)
}

/// Folds a name for comparison: lower case, whitespace runs collapsed.
///
/// # Parameters
///
/// - `value`: the name to fold.
///
/// # Returns
///
/// The folded name, without leading or trailing whitespace.
fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_lowercase()
}

/// Appends a link when the value exists.
///
/// # Parameters
///
/// - `links`: the list to append to.
/// - `kind`: the link kind.
/// - `value`: the URL, or `None` when the row carried no usable value.
fn push_link(links: &mut Vec<ArtistLink>, kind: &str, value: Option<String>) {
    if let Some(url) = value {
        links.push(ArtistLink {
            kind: kind.to_string(),
            url,
        });
    }
}

/// Turns a free-text profile URL into an absolute URL.
///
/// # Parameters
///
/// - `value`: the column value.
///
/// # Returns
///
/// The URL unchanged when it carries an `http` or `https` scheme, the same
/// URL with `https://` in front when it is a bare host, and `None` when the
/// value is blank or is not host-like (TheAudioDB stores a bare flag such as
/// `1` in some of these columns).
fn absolute_url(value: Option<&str>) -> Option<String> {
    let value = audiodb::non_blank(value)?;

    if value.starts_with("http://") || value.starts_with("https://") {
        return Some(value.to_string());
    }

    if !value.contains('.') {
        return None;
    }

    Some(format!("https://{value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coldplay() -> audiodb::Artist {
        let response: audiodb::ArtistsResponse =
            serde_json::from_str(include_str!("fixtures/artist_mb.json")).unwrap();

        response.into_artists().remove(0)
    }

    #[test]
    fn maps_biography_and_identifiers() {
        let info = artist_info(&coldplay());

        let pairs: Vec<(&str, &str)> = info
            .ids
            .iter()
            .map(|pair| (pair.namespace.as_str(), pair.value.as_str()))
            .collect();

        assert_eq!(
            pairs,
            vec![
                (NS_ARTIST, "cc197bad-dc9c-440d-a5b5-d52ba2e14234"),
                (NS_AUDIODB, "111239"),
            ]
        );
        assert!(info.biography.unwrap().starts_with("Coldplay are"));
    }

    #[test]
    fn maps_the_profile_links() {
        let links = artist_links(&coldplay());
        let pairs: Vec<(&str, &str)> = links
            .iter()
            .map(|link| (link.kind.as_str(), link.url.as_str()))
            .collect();

        assert_eq!(
            pairs,
            vec![
                ("homepage", "https://www.coldplay.com"),
                (
                    "lastfm",
                    "http://www.last.fm/music/Coldplay/+charts?rangetype=6month"
                ),
            ]
        );
    }

    #[test]
    fn omits_blank_and_flag_like_values() {
        let artist = audiodb::Artist {
            str_website: Some("1".to_string()),
            str_facebook: Some("www.facebook.com/somebody".to_string()),
            str_twitter: Some("1".to_string()),
            ..audiodb::Artist::default()
        };

        assert!(artist_links(&artist).is_empty());
    }

    #[test]
    fn compares_names_ignoring_case_and_whitespace() {
        assert!(same_text("Coldplay", "coldplay"));
        assert!(same_text("The  Weeknd", " the weeknd "));
        assert!(!same_text("Coldplay", "Coldplay Tribute Band"));
    }

    #[test]
    fn an_artist_without_identifiers_maps_to_no_ids() {
        let artist = audiodb::Artist {
            str_artist: Some("Anon".to_string()),
            ..audiodb::Artist::default()
        };
        let info = artist_info(&artist);

        assert!(info.ids.is_empty());
        assert_eq!(info.biography, None);
    }
}
