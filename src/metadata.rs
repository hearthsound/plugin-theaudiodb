//! The metadata operations: artist profiles.
//!
//! TheAudioDB is an artist and artwork database, not a recording catalog. It
//! has no recording-level identity resolution and no release materialization,
//! so `resolve-recording` answers no matches and `get-release` answers
//! not-found; both let the server move on to the next metadata instance in the
//! layer order.
//!
//! Artist information is real: a MusicBrainz id the server already holds is
//! looked up directly, and when that lookup finds nothing the artist name is
//! searched instead. A search hit is accepted only when it spells the requested
//! name (or the row's alternate name) the same way, so a namesake or a tribute
//! act is never mistaken for the artist the server asked about.

use hearthsound_plugin::metadata_provider::exports::hearthsound::plugin::metadata::{
    ArtistInfo, ArtistInfoRequest, RecordingHint, RecordingMatch,
};
use hearthsound_plugin::types::{Album, IdPair, ProviderError};

use crate::api::TheAudioDbApi;
use crate::audiodb;
use crate::map;

/// Returns the profile of the requested artist.
///
/// # Parameters
///
/// - `api`: the artist operations of the configured instance.
/// - `request`: the artist the server wants information on, carrying an
///   `mb-artist` id, a name, or both.
///
/// # Returns
///
/// The artist's profile, or `None` when neither the MusicBrainz id nor the name
/// found an artist.
///
/// # Errors
///
/// Returns the provider error of a request that failed.
pub(crate) fn artist_info(
    api: &dyn TheAudioDbApi,
    request: &ArtistInfoRequest,
) -> Result<Option<ArtistInfo>, ProviderError> {
    if let Some(mbid) = id_value(&request.ids, map::NS_ARTIST) {
        let artists = api.artist_by_mbid(mbid)?;

        if let Some(artist) = artists.first() {
            return Ok(Some(map::artist_info(artist)));
        }
    }

    let name = request.name.trim();

    if name.is_empty() {
        return Ok(None);
    }

    let artists = api.search_artists(name)?;

    Ok(select_artist(&artists, name).map(map::artist_info))
}

/// Picks the searched artist that names the requested one.
///
/// # Parameters
///
/// - `artists`: the rows of an artist search, most relevant first.
/// - `name`: the artist name that was searched for.
///
/// # Returns
///
/// The first row whose primary or alternate name equals `name` ignoring case
/// and whitespace, or `None` when no row qualifies.
///
/// # Example
///
/// ```text
/// select_artist(&[Coldplay, Coldplay Tribute Band], "Coldplay") -> Some(Coldplay)
/// select_artist(&[Coldplay Tribute Band], "Coldplay")           -> None
/// ```
pub(crate) fn select_artist<'a>(
    artists: &'a [audiodb::Artist],
    name: &str,
) -> Option<&'a audiodb::Artist> {
    artists.iter().find(|artist| {
        artist
            .name()
            .is_some_and(|value| map::same_text(value, name))
            || artist
                .alternate_name()
                .is_some_and(|value| map::same_text(value, name))
    })
}

/// Returns the value of an identifier in one namespace.
///
/// # Parameters
///
/// - `ids`: the identifiers the request carries.
/// - `namespace`: the namespace to look for.
///
/// # Returns
///
/// The trimmed value of the first identifier in that namespace, or `None` when
/// there is none or it is blank.
fn id_value<'a>(ids: &'a [IdPair], namespace: &str) -> Option<&'a str> {
    ids.iter()
        .find(|pair| pair.namespace == namespace)
        .map(|pair| pair.value.trim())
        .filter(|value| !value.is_empty())
}

/// Resolves a recording hint to recordings.
///
/// # Parameters
///
/// - `_hint`: the hint the server wants resolved.
///
/// # Returns
///
/// An empty list: TheAudioDB does not index recordings, so it has no identity
/// to resolve and the server asks the next metadata instance.
///
/// # Errors
///
/// This implementation never fails.
pub(crate) fn resolve_recording(
    _hint: &RecordingHint,
) -> Result<Vec<RecordingMatch>, ProviderError> {
    Ok(Vec::new())
}

/// Materializes a release from canonical identifiers.
///
/// # Parameters
///
/// - `_ids`: the identifiers of the release to materialize.
///
/// # Returns
///
/// This implementation always answers [`ProviderError::NotFound`], because
/// TheAudioDB does not publish release contents or track lists.
///
/// # Errors
///
/// Always returns [`ProviderError::NotFound`].
pub(crate) fn get_release(_ids: &[IdPair]) -> Result<Album, ProviderError> {
    Err(ProviderError::NotFound)
}

/// Probes the API with a small artist search.
///
/// # Parameters
///
/// - `api`: the artist operations of the configured instance.
///
/// # Returns
///
/// `Ok(())` when the service answered with the JSON this plugin expects.
///
/// # Errors
///
/// Returns the provider error of the request.
pub(crate) fn health(api: &dyn TheAudioDbApi) -> Result<(), ProviderError> {
    api.search_artists("the").map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct FakeApi {
        mbid: Vec<audiodb::Artist>,
        search: Vec<audiodb::Artist>,
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

            Ok(self.mbid.clone())
        }

        fn search_artists(&self, name: &str) -> Result<Vec<audiodb::Artist>, ProviderError> {
            self.record(format!("search:{name}"));

            Ok(self.search.clone())
        }
    }

    fn fixture(name: &str) -> Vec<audiodb::Artist> {
        let json = match name {
            "artist_mb" => include_str!("fixtures/artist_mb.json"),
            "artist_search" => include_str!("fixtures/artist_search.json"),
            _ => include_str!("fixtures/artist_none.json"),
        };

        serde_json::from_str::<audiodb::ArtistsResponse>(json)
            .unwrap()
            .into_artists()
    }

    fn request(name: &str, ids: Vec<IdPair>) -> ArtistInfoRequest {
        ArtistInfoRequest {
            name: name.to_string(),
            ids,
        }
    }

    #[test]
    fn an_mbid_names_the_artist_without_a_search() {
        let api = FakeApi {
            mbid: fixture("artist_mb"),
            ..FakeApi::default()
        };
        let info = artist_info(
            &api,
            &request(
                "Ignored name",
                vec![map::id_pair(
                    map::NS_ARTIST,
                    "cc197bad-dc9c-440d-a5b5-d52ba2e14234",
                )],
            ),
        )
        .unwrap()
        .unwrap();

        assert_eq!(info.ids[0].namespace, map::NS_ARTIST);
        assert_eq!(info.ids[0].value, "cc197bad-dc9c-440d-a5b5-d52ba2e14234");
        assert!(info.biography.unwrap().starts_with("Coldplay are"));
        assert_eq!(
            *api.calls.borrow(),
            vec!["mbid:cc197bad-dc9c-440d-a5b5-d52ba2e14234".to_string()]
        );
    }

    #[test]
    fn a_missing_mbid_falls_back_to_the_name_search() {
        let api = FakeApi {
            search: fixture("artist_search"),
            ..FakeApi::default()
        };
        let info = artist_info(
            &api,
            &request(
                "Coldplay",
                vec![map::id_pair(
                    map::NS_ARTIST,
                    "00000000-0000-0000-0000-000000000000",
                )],
            ),
        )
        .unwrap()
        .unwrap();

        assert_eq!(info.ids[1].namespace, map::NS_AUDIODB);
        assert_eq!(info.ids[1].value, "111239");
        assert_eq!(
            *api.calls.borrow(),
            vec![
                "mbid:00000000-0000-0000-0000-000000000000".to_string(),
                "search:Coldplay".to_string(),
            ]
        );
    }

    #[test]
    fn a_namesake_search_hit_is_not_the_artist() {
        let api = FakeApi {
            search: fixture("artist_search")[1..].to_vec(),
            ..FakeApi::default()
        };

        assert!(
            artist_info(&api, &request("Coldplay", Vec::new()))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn no_name_and_no_id_answers_none_without_a_request() {
        let api = FakeApi::default();

        assert!(
            artist_info(&api, &request("   ", Vec::new()))
                .unwrap()
                .is_none()
        );
        assert!(api.calls.borrow().is_empty());
    }

    #[test]
    fn an_alternate_name_names_the_artist() {
        let api = FakeApi {
            search: vec![audiodb::Artist {
                str_artist: Some("The Beatles".to_string()),
                str_artist_alternate: Some("Beatles".to_string()),
                ..audiodb::Artist::default()
            }],
            ..FakeApi::default()
        };

        assert!(
            artist_info(&api, &request("beatles", Vec::new()))
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn resolve_recording_answers_no_matches() {
        let hint = RecordingHint {
            title: "Yellow".to_string(),
            artists: vec!["Coldplay".to_string()],
            album: None,
            duration_ms: Some(266_000),
            isrc: None,
        };

        assert!(resolve_recording(&hint).unwrap().is_empty());
    }

    #[test]
    fn get_release_is_not_found() {
        assert!(matches!(
            get_release(&[map::id_pair(
                "mb-release",
                "72087170-41c6-4371-a87c-47061feeda10"
            )]),
            Err(ProviderError::NotFound)
        ));
    }
}
