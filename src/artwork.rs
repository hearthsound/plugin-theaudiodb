//! The artwork operations: artist portraits and album covers.
//!
//! The plugin answers the artwork interface with image URLs rather than image
//! bytes, because the server fetches an image through the instance that named
//! it. TheAudioDB keeps one portrait column per artist row and one front-cover
//! column per album row, so a subject resolves to at most one image.
//!
//! A subject is looked up by the strongest identifier it carries: the
//! instance's own id first (when the server already holds one from this
//! plugin), then a canonical MusicBrainz id, then a title search. A search hit
//! counts only when the title and the credited artist are the ones the subject
//! names, so a remix or a namesake is never shown as the requested cover.
//!
//! Having no picture is not a failure: an artist or album the service does not
//! know, a row without the picture column, and a subject of a kind this plugin
//! does not serve all answer an empty list. Only a request that could not be
//! made at all becomes a provider error.

use hearthsound_plugin::metadata_artwork_provider::exports::hearthsound::plugin::artwork::ArtworkSubject;
use hearthsound_plugin::types::{EntityKind, Image, ProviderError};

use crate::api::TheAudioDbApi;
use crate::audiodb;
use crate::map;
use crate::metadata::select_artist;

/// Finds the picture of an artwork subject.
///
/// # Parameters
///
/// - `api`: the artist and album operations of the configured instance.
/// - `subject`: the entity whose picture is wanted.
///
/// # Returns
///
/// The portrait of an artist subject or the front cover of an album subject,
/// and an empty list for any other kind, for a subject that carries nothing to
/// look up, and for one the service does not know. An empty list is not an
/// error, because a provider without a picture is a normal answer for the
/// server's artwork layer.
///
/// # Errors
///
/// Returns the provider error of a request that failed.
pub(crate) fn find_artwork(
    api: &dyn TheAudioDbApi,
    subject: &ArtworkSubject,
) -> Result<Vec<Image>, ProviderError> {
    match subject.kind {
        EntityKind::Artist => artist_artwork(api, subject),
        EntityKind::Album => album_artwork(api, subject),
        _ => Ok(Vec::new()),
    }
}

/// Finds an artist subject's portrait.
///
/// # Parameters
///
/// - `api`: the artist operations of the configured instance.
/// - `subject`: the artist whose portrait is wanted.
///
/// # Returns
///
/// The portrait, or an empty list when the row has none and when no lookup
/// finds the artist.
///
/// # Errors
///
/// Returns the provider error of a request that failed.
fn artist_artwork(
    api: &dyn TheAudioDbApi,
    subject: &ArtworkSubject,
) -> Result<Vec<Image>, ProviderError> {
    if let Some(id) = provider_id(subject) {
        let artists = api.artist_by_id(id)?;

        if let Some(artist) = artists.first() {
            return Ok(map::artist_images(artist));
        }
    }

    if let Some(mbid) = map::id_value(&subject.ids, map::NS_ARTIST) {
        let artists = api.artist_by_mbid(mbid)?;

        if let Some(artist) = artists.first() {
            return Ok(map::artist_images(artist));
        }
    }

    let name = subject.title.trim();

    if name.is_empty() {
        return Ok(Vec::new());
    }

    let artists = api.search_artists(name)?;

    Ok(select_artist(&artists, name)
        .map(map::artist_images)
        .unwrap_or_default())
}

/// Finds an album subject's front cover.
///
/// # Parameters
///
/// - `api`: the album operations of the configured instance.
/// - `subject`: the album whose cover is wanted.
///
/// # Returns
///
/// The cover, or an empty list when the row has none, when no lookup finds the
/// album, and when the subject carries no artist to search with.
///
/// # Errors
///
/// Returns the provider error of a request that failed.
fn album_artwork(
    api: &dyn TheAudioDbApi,
    subject: &ArtworkSubject,
) -> Result<Vec<Image>, ProviderError> {
    if let Some(id) = provider_id(subject) {
        let albums = api.album_by_id(id)?;

        if let Some(album) = albums.first() {
            return Ok(map::album_images(album));
        }
    }

    if let Some(mbid) = map::id_value(&subject.ids, map::NS_RELEASE_GROUP) {
        let albums = api.album_by_release_group(mbid)?;

        if let Some(album) = albums.first() {
            return Ok(map::album_images(album));
        }
    }

    let title = subject.title.trim();
    let artist = subject
        .artists
        .first()
        .map(|artist| artist.trim())
        .filter(|artist| !artist.is_empty());

    let (true, Some(artist)) = (!title.is_empty(), artist) else {
        return Ok(Vec::new());
    };

    let albums = api.search_albums(artist, title)?;

    Ok(select_album(&albums, title, Some(artist))
        .map(map::album_images)
        .unwrap_or_default())
}

/// Returns the subject's own provider id, when it carries one.
///
/// # Parameters
///
/// - `subject`: the subject to read.
///
/// # Returns
///
/// The trimmed provider id, or `None` when it is absent or blank.
fn provider_id(subject: &ArtworkSubject) -> Option<&str> {
    subject
        .provider_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
}

/// Picks the searched album that matches a subject.
///
/// # Parameters
///
/// - `albums`: the rows of an album search, most relevant first.
/// - `title`: the album title the subject names.
/// - `artist`: the artist the subject credits, or `None` when it credits none.
///
/// # Returns
///
/// The first row whose title, and whose artist when one was given, equals the
/// subject's ignoring case and whitespace; `None` when no row qualifies.
///
/// # Example
///
/// ```text
/// select_album(&[Homework, Homework (Remixes)], "Homework", Some("Daft Punk"))
///   -> Some(Homework)
/// select_album(&[Homework (Remixes)], "Homework", Some("Daft Punk"))
///   -> None
/// ```
fn select_album<'a>(
    albums: &'a [audiodb::Album],
    title: &str,
    artist: Option<&str>,
) -> Option<&'a audiodb::Album> {
    albums.iter().find(|album| {
        album
            .title()
            .is_some_and(|value| map::same_text(value, title))
            && artist.is_none_or(|artist| {
                album
                    .artist()
                    .is_some_and(|value| map::same_text(value, artist))
            })
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use hearthsound_plugin::types::IdPair;

    use super::*;

    #[derive(Default)]
    struct FakeApi {
        artist_by_id: Vec<audiodb::Artist>,
        artist_by_mbid: Vec<audiodb::Artist>,
        artists: Vec<audiodb::Artist>,
        album_by_id: Vec<audiodb::Album>,
        album_by_release_group: Vec<audiodb::Album>,
        albums: Vec<audiodb::Album>,
        calls: RefCell<Vec<String>>,
    }

    impl FakeApi {
        fn record(&self, call: String) {
            self.calls.borrow_mut().push(call);
        }
    }

    impl TheAudioDbApi for FakeApi {
        fn artist_by_mbid(&self, mbid: &str) -> Result<Vec<audiodb::Artist>, ProviderError> {
            self.record(format!("mbid:{mbid}"));

            Ok(self.artist_by_mbid.clone())
        }

        fn search_artists(&self, name: &str) -> Result<Vec<audiodb::Artist>, ProviderError> {
            self.record(format!("search:{name}"));

            Ok(self.artists.clone())
        }

        fn artist_by_id(&self, id: &str) -> Result<Vec<audiodb::Artist>, ProviderError> {
            self.record(format!("artist:{id}"));

            Ok(self.artist_by_id.clone())
        }

        fn album_by_release_group(&self, mbid: &str) -> Result<Vec<audiodb::Album>, ProviderError> {
            self.record(format!("album-mbid:{mbid}"));

            Ok(self.album_by_release_group.clone())
        }

        fn album_by_id(&self, id: &str) -> Result<Vec<audiodb::Album>, ProviderError> {
            self.record(format!("album:{id}"));

            Ok(self.album_by_id.clone())
        }

        fn search_albums(
            &self,
            artist: &str,
            album: &str,
        ) -> Result<Vec<audiodb::Album>, ProviderError> {
            self.record(format!("album-search:{artist}:{album}"));

            Ok(self.albums.clone())
        }
    }

    fn artists(name: &str) -> Vec<audiodb::Artist> {
        let json = match name {
            "artist_mb" => include_str!("fixtures/artist_mb.json"),
            "artist_search" => include_str!("fixtures/artist_search.json"),
            _ => include_str!("fixtures/artist_none.json"),
        };

        serde_json::from_str::<audiodb::ArtistsResponse>(json)
            .unwrap()
            .into_artists()
    }

    fn albums(name: &str) -> Vec<audiodb::Album> {
        let json = match name {
            "album_mb" => include_str!("fixtures/album_mb.json"),
            "album_search" => include_str!("fixtures/album_search.json"),
            _ => include_str!("fixtures/album_none.json"),
        };

        serde_json::from_str::<audiodb::AlbumsResponse>(json)
            .unwrap()
            .into_albums()
    }

    fn subject(
        kind: EntityKind,
        title: &str,
        artists: &[&str],
        ids: Vec<IdPair>,
    ) -> ArtworkSubject {
        ArtworkSubject {
            kind,
            provider_id: None,
            ids,
            title: title.to_string(),
            artists: artists.iter().map(|artist| artist.to_string()).collect(),
        }
    }

    fn urls(images: &[Image]) -> Vec<&str> {
        images.iter().map(|image| image.url.as_str()).collect()
    }

    #[test]
    fn an_artist_mbid_finds_the_portrait() {
        let api = FakeApi {
            artist_by_mbid: artists("artist_mb"),
            ..FakeApi::default()
        };
        let subject = subject(
            EntityKind::Artist,
            "Ignored",
            &[],
            vec![map::id_pair(
                map::NS_ARTIST,
                "cc197bad-dc9c-440d-a5b5-d52ba2e14234",
            )],
        );
        let images = find_artwork(&api, &subject).unwrap();

        assert_eq!(
            urls(&images),
            vec!["https://www.theaudiodb.com/images/media/artist/thumb/uxrqxy1347913147.jpg"]
        );
        assert_eq!(
            *api.calls.borrow(),
            vec!["mbid:cc197bad-dc9c-440d-a5b5-d52ba2e14234".to_string()]
        );
    }

    #[test]
    fn a_missing_artist_mbid_falls_back_to_the_name() {
        let api = FakeApi {
            artists: artists("artist_search"),
            ..FakeApi::default()
        };
        let subject = subject(
            EntityKind::Artist,
            "Coldplay",
            &[],
            vec![map::id_pair(
                map::NS_ARTIST,
                "00000000-0000-0000-0000-000000000000",
            )],
        );
        let images = find_artwork(&api, &subject).unwrap();

        assert_eq!(
            urls(&images),
            vec!["https://www.theaudiodb.com/images/media/artist/thumb/uxrqxy1347913147.jpg"]
        );
        assert_eq!(
            *api.calls.borrow(),
            vec![
                "mbid:00000000-0000-0000-0000-000000000000".to_string(),
                "search:Coldplay".to_string(),
            ]
        );
    }

    #[test]
    fn a_provider_id_is_used_before_an_mbid() {
        let api = FakeApi {
            artist_by_id: artists("artist_mb"),
            artist_by_mbid: artists("artist_search"),
            ..FakeApi::default()
        };
        let mut subject = subject(
            EntityKind::Artist,
            "Coldplay",
            &[],
            vec![map::id_pair(
                map::NS_ARTIST,
                "cc197bad-dc9c-440d-a5b5-d52ba2e14234",
            )],
        );
        subject.provider_id = Some("111239".to_string());
        let images = find_artwork(&api, &subject).unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(*api.calls.borrow(), vec!["artist:111239".to_string()]);
    }

    #[test]
    fn an_artist_without_a_portrait_answers_nothing() {
        let api = FakeApi {
            artists: vec![audiodb::Artist {
                str_artist: Some("Anon".to_string()),
                str_artist_thumb: None,
                ..audiodb::Artist::default()
            }],
            ..FakeApi::default()
        };
        let subject = subject(EntityKind::Artist, "Anon", &[], Vec::new());

        assert!(find_artwork(&api, &subject).unwrap().is_empty());
    }

    #[test]
    fn an_album_release_group_finds_the_cover() {
        let api = FakeApi {
            album_by_release_group: albums("album_mb"),
            ..FakeApi::default()
        };
        let subject = subject(
            EntityKind::Album,
            "Echoes of Silence",
            &["The Weeknd"],
            vec![map::id_pair(
                map::NS_RELEASE_GROUP,
                "1dc4c347-a1db-32aa-b14f-bc9cc507b843",
            )],
        );
        let images = find_artwork(&api, &subject).unwrap();

        assert_eq!(
            urls(&images),
            vec!["https://www.theaudiodb.com/images/media/album/thumb/xwvwvp1342551819.jpg"]
        );
        assert_eq!(
            *api.calls.borrow(),
            vec!["album-mbid:1dc4c347-a1db-32aa-b14f-bc9cc507b843".to_string()]
        );
    }

    #[test]
    fn an_album_without_an_id_is_searched_by_artist_and_title() {
        let api = FakeApi {
            albums: albums("album_search"),
            ..FakeApi::default()
        };
        let subject = subject(EntityKind::Album, "Homework", &["Daft Punk"], Vec::new());
        let images = find_artwork(&api, &subject).unwrap();

        assert_eq!(
            urls(&images),
            vec!["https://www.theaudiodb.com/images/media/album/thumb/homework.jpg"]
        );
        assert_eq!(
            *api.calls.borrow(),
            vec!["album-search:Daft Punk:Homework".to_string()]
        );
    }

    #[test]
    fn a_search_hit_that_is_a_remix_is_not_the_album() {
        let api = FakeApi {
            albums: albums("album_search")[1..].to_vec(),
            ..FakeApi::default()
        };
        let subject = subject(EntityKind::Album, "Homework", &["Daft Punk"], Vec::new());

        assert!(find_artwork(&api, &subject).unwrap().is_empty());
    }

    #[test]
    fn an_album_without_an_artist_is_not_searched() {
        let api = FakeApi::default();
        let subject = subject(EntityKind::Album, "Homework", &[], Vec::new());

        assert!(find_artwork(&api, &subject).unwrap().is_empty());
        assert!(api.calls.borrow().is_empty());
    }

    #[test]
    fn a_track_subject_has_no_picture() {
        let api = FakeApi::default();
        let subject = subject(EntityKind::Track, "Yellow", &["Coldplay"], Vec::new());

        assert!(find_artwork(&api, &subject).unwrap().is_empty());
        assert!(api.calls.borrow().is_empty());
    }
}
