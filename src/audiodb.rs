//! Serde types for TheAudioDB's v1 JSON responses.
//!
//! TheAudioDB is a legacy database whose rows carry many fields and whose
//! scalar values are sometimes strings and sometimes numbers. Only the fields
//! the plugin reads are modelled, every one of them optional, and the
//! identifiers are normalized to strings by [`text`] so a row deserializes
//! whether the API sends `"111239"` or `111239`.
//!
//! A response wraps its rows in a property named after the entity, and an
//! empty result is that property set to `null` rather than an empty array:
//!
//! ```text
//! {"artists": [{"idArtist": "111239", ...}]}
//! {"artists": null}
//! ```
//!
//! Unknown properties are ignored, which keeps the plugin working as the
//! legacy table gains columns.

use serde::{Deserialize, Deserializer};

/// One artist row.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Artist {
    /// TheAudioDB's own artist id.
    #[serde(default, rename = "idArtist", deserialize_with = "text")]
    pub id_artist: Option<String>,
    /// The artist's primary name.
    #[serde(default, rename = "strArtist", deserialize_with = "text")]
    pub str_artist: Option<String>,
    /// Another name the artist is known by.
    #[serde(default, rename = "strArtistAlternate", deserialize_with = "text")]
    pub str_artist_alternate: Option<String>,
    /// The MusicBrainz artist id.
    #[serde(default, rename = "strMusicBrainzID", deserialize_with = "text")]
    pub str_music_brainz_id: Option<String>,
    /// The English biography in the legacy spelling of the field.
    #[serde(default, rename = "strBiographyEN", deserialize_with = "text")]
    pub str_biography_en: Option<String>,
    /// The English biography in the field the current API serves it in.
    #[serde(default, rename = "strBiography", deserialize_with = "text")]
    pub str_biography: Option<String>,
    /// The artist's own website.
    #[serde(default, rename = "strWebsite", deserialize_with = "text")]
    pub str_website: Option<String>,
    /// The artist's Facebook page.
    #[serde(default, rename = "strFacebook", deserialize_with = "text")]
    pub str_facebook: Option<String>,
    /// The artist's Twitter account.
    #[serde(default, rename = "strTwitter", deserialize_with = "text")]
    pub str_twitter: Option<String>,
    /// A Last.fm chart page for the artist.
    #[serde(default, rename = "strLastFMChart", deserialize_with = "text")]
    pub str_last_fm_chart: Option<String>,
    /// The artist's portrait.
    #[serde(default, rename = "strArtistThumb", deserialize_with = "text")]
    pub str_artist_thumb: Option<String>,
}

impl Artist {
    /// Returns the TheAudioDB artist id, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The id, or `None` when it is absent or blank.
    pub fn provider_id(&self) -> Option<&str> {
        non_blank(self.id_artist.as_deref())
    }

    /// Returns the artist's primary name, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The name, or `None` when it is absent or blank.
    pub fn name(&self) -> Option<&str> {
        non_blank(self.str_artist.as_deref())
    }

    /// Returns the alternate artist name, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The alternate name, or `None` when it is absent or blank.
    pub fn alternate_name(&self) -> Option<&str> {
        non_blank(self.str_artist_alternate.as_deref())
    }

    /// Returns the MusicBrainz artist id, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The id, or `None` when it is absent or blank.
    pub fn music_brainz_id(&self) -> Option<&str> {
        non_blank(self.str_music_brainz_id.as_deref())
    }

    /// Returns the English biography, trimmed, when the row carries one.
    ///
    /// The current API serves the English text in `strBiography`; a legacy
    /// response used `strBiographyEN`. The legacy field is read first, so a
    /// service that still sends it wins, and the current field is the fallback.
    ///
    /// # Returns
    ///
    /// The biography, or `None` when both fields are absent or blank.
    pub fn biography(&self) -> Option<&str> {
        non_blank(self.str_biography_en.as_deref())
            .or_else(|| non_blank(self.str_biography.as_deref()))
    }

    /// Returns the artist portrait URL, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The URL, or `None` when it is absent or blank.
    pub fn thumb(&self) -> Option<&str> {
        non_blank(self.str_artist_thumb.as_deref())
    }
}

/// The response of every artist endpoint.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ArtistsResponse {
    /// The matching artists, or `null` when the service knows none.
    #[serde(default)]
    pub artists: Option<Vec<Artist>>,
}

impl ArtistsResponse {
    /// Returns the rows the response carries.
    ///
    /// # Returns
    ///
    /// The rows, or an empty list when the response carried `null`.
    pub fn into_artists(self) -> Vec<Artist> {
        self.artists.unwrap_or_default()
    }
}

/// One album row.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Album {
    /// TheAudioDB's own album id.
    #[serde(default, rename = "idAlbum", deserialize_with = "text")]
    pub id_album: Option<String>,
    /// The album title.
    #[serde(default, rename = "strAlbum", deserialize_with = "text")]
    pub str_album: Option<String>,
    /// The album artist.
    #[serde(default, rename = "strArtist", deserialize_with = "text")]
    pub str_artist: Option<String>,
    /// The MusicBrainz release group id.
    #[serde(default, rename = "strMusicBrainzID", deserialize_with = "text")]
    pub str_music_brainz_id: Option<String>,
    /// The front cover thumbnail.
    #[serde(default, rename = "strAlbumThumb", deserialize_with = "text")]
    pub str_album_thumb: Option<String>,
    /// The high-quality front cover thumbnail.
    #[serde(default, rename = "strAlbumThumbHQ", deserialize_with = "text")]
    pub str_album_thumb_hq: Option<String>,
}

impl Album {
    /// Returns the TheAudioDB album id, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The id, or `None` when it is absent or blank.
    pub fn provider_id(&self) -> Option<&str> {
        non_blank(self.id_album.as_deref())
    }

    /// Returns the album title, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The title, or `None` when it is absent or blank.
    pub fn title(&self) -> Option<&str> {
        non_blank(self.str_album.as_deref())
    }

    /// Returns the album artist, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The artist name, or `None` when it is absent or blank.
    pub fn artist(&self) -> Option<&str> {
        non_blank(self.str_artist.as_deref())
    }

    /// Returns the MusicBrainz release group id, trimmed, when the row carries
    /// one.
    ///
    /// # Returns
    ///
    /// The id, or `None` when it is absent or blank.
    pub fn release_group_id(&self) -> Option<&str> {
        non_blank(self.str_music_brainz_id.as_deref())
    }

    /// Returns the album cover URL, trimmed, when the row carries one.
    ///
    /// # Returns
    ///
    /// The standard thumbnail, or the high-quality one when the standard is
    /// absent, or `None` when the row has neither.
    pub fn thumb(&self) -> Option<&str> {
        non_blank(self.str_album_thumb.as_deref())
            .or_else(|| non_blank(self.str_album_thumb_hq.as_deref()))
    }
}

/// The response of every album endpoint.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AlbumsResponse {
    /// The matching albums, or `null` when the service knows none.
    #[serde(default)]
    pub album: Option<Vec<Album>>,
}

impl AlbumsResponse {
    /// Returns the rows the response carries.
    ///
    /// # Returns
    ///
    /// The rows, or an empty list when the response carried `null`.
    pub fn into_albums(self) -> Vec<Album> {
        self.album.unwrap_or_default()
    }
}

/// Deserializes a legacy scalar that may be a string, a number, or null.
///
/// # Parameters
///
/// - `deserializer`: the value to read.
///
/// # Returns
///
/// The value as a string, whichever of the two scalar shapes it has, or `None`
/// for null.
///
/// # Errors
///
/// Returns the deserializer's error when the value is neither a scalar nor
/// null (an object or an array, for example).
fn text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;

    Ok(value.and_then(|value| match value {
        serde_json::Value::String(text) => Some(text),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }))
}

/// Returns a trimmed string, or `None` when it is absent or blank.
///
/// # Parameters
///
/// - `value`: the value as the response carried it.
///
/// # Returns
///
/// The trimmed value, or `None`.
pub(crate) fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_artist_lookup() {
        let response: ArtistsResponse =
            serde_json::from_str(include_str!("fixtures/artist_mb.json")).unwrap();
        let artists = response.into_artists();

        assert_eq!(artists.len(), 1);
        let artist = &artists[0];
        assert_eq!(artist.provider_id(), Some("111239"));
        assert_eq!(artist.name(), Some("Coldplay"));
        assert_eq!(
            artist.music_brainz_id(),
            Some("cc197bad-dc9c-440d-a5b5-d52ba2e14234")
        );
        assert!(artist.biography().unwrap().starts_with("Coldplay are"));
        assert_eq!(
            artist.thumb(),
            Some("https://r2.theaudiodb.com/images/media/artist/thumb/uxrqxy1347913147.jpg")
        );
    }

    #[test]
    fn reads_an_empty_artist_response() {
        let response: ArtistsResponse =
            serde_json::from_str(include_str!("fixtures/artist_none.json")).unwrap();

        assert!(response.into_artists().is_empty());
    }

    #[test]
    fn reads_numeric_identifiers_as_text() {
        let response: ArtistsResponse = serde_json::from_str(
            r#"{"artists":[{"idArtist":555,"strArtist":"Numeric","strTwitter":1}]}"#,
        )
        .unwrap();
        let artists = response.into_artists();

        assert_eq!(artists[0].provider_id(), Some("555"));
        assert_eq!(artists[0].str_twitter.as_deref(), Some("1"));
    }

    #[test]
    fn ignores_unknown_properties_and_missing_ones() {
        let response: ArtistsResponse = serde_json::from_str(
            r#"{"artists":[{"strArtist":"Minimal","strStyle":"Rock/Pop","intFormedYear":1996}]}"#,
        )
        .unwrap();
        let artists = response.into_artists();

        assert_eq!(artists[0].name(), Some("Minimal"));
        assert_eq!(artists[0].provider_id(), None);
        assert_eq!(artists[0].biography(), None);
    }

    #[test]
    fn blank_values_read_as_absent() {
        let response: ArtistsResponse =
            serde_json::from_str(r#"{"artists":[{"strArtist":"   ","strBiography":""}]}"#).unwrap();
        let artists = response.into_artists();

        assert_eq!(artists[0].name(), None);
        assert_eq!(artists[0].biography(), None);
    }

    #[test]
    fn reads_the_biography_from_the_field_the_api_serves_it_in() {
        let response: ArtistsResponse =
            serde_json::from_str(r#"{"artists":[{"strBiography":"Coldplay are a band."}]}"#)
                .unwrap();

        assert_eq!(
            response.into_artists()[0].biography(),
            Some("Coldplay are a band.")
        );
    }

    #[test]
    fn prefers_the_legacy_biography_field_when_it_is_present() {
        let response: ArtistsResponse = serde_json::from_str(
            r#"{"artists":[{"strBiography":"Current","strBiographyEN":"Legacy"}]}"#,
        )
        .unwrap();

        assert_eq!(response.into_artists()[0].biography(), Some("Legacy"));
    }

    #[test]
    fn reads_an_album_lookup() {
        let response: AlbumsResponse =
            serde_json::from_str(include_str!("fixtures/album_mb.json")).unwrap();
        let albums = response.into_albums();

        assert_eq!(albums.len(), 1);
        let album = &albums[0];
        assert_eq!(album.provider_id(), Some("2115888"));
        assert_eq!(album.title(), Some("Echoes of Silence"));
        assert_eq!(album.artist(), Some("The Weeknd"));
        assert_eq!(
            album.release_group_id(),
            Some("1dc4c347-a1db-32aa-b14f-bc9cc507b843")
        );
        assert_eq!(
            album.thumb(),
            Some("https://r2.theaudiodb.com/images/media/album/thumb/xwvwvp1342551819.jpg")
        );
    }

    #[test]
    fn reads_an_empty_album_response() {
        let response: AlbumsResponse =
            serde_json::from_str(include_str!("fixtures/album_none.json")).unwrap();

        assert!(response.into_albums().is_empty());
    }

    #[test]
    fn a_missing_thumbnail_falls_back_to_the_high_quality_one() {
        let response: AlbumsResponse = serde_json::from_str(
            r#"{"album":[{"strAlbum":"Only HQ","strAlbumThumb":null,"strAlbumThumbHQ":"https://x.example/hq.jpg"}]}"#,
        )
        .unwrap();

        assert_eq!(
            response.into_albums()[0].thumb(),
            Some("https://x.example/hq.jpg")
        );
    }
}
