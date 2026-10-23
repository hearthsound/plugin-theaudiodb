# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- The biography is read from `strBiography`, the field the current API serves
  the English text in, with the older `strBiographyEN` spelling still accepted
  first. The link mapping no longer reads Wikipedia, Wikidata, Discogs or
  AllMusic columns the artist response does not carry.

### Added

- The repository scaffolding: the crate manifest, the toolchain pin, the dual
  MIT/Apache-2.0 license files and this changelog.
- Serde types for TheAudioDB's v1 responses and the translation of an artist
  row into the metadata interface's profile, with captured response fixtures
  the unit tests run on.
- The artist information lookup: a MusicBrainz id the server holds is read
  directly, with a fallback to an exact-name artist search, and the profile
  carries the English biography and the homepage, Wikipedia, Wikidata, Last.fm,
  Discogs and AllMusic links the row holds.
- The artwork interface: an artist's portrait from its TheAudioDB row and an
  album's front cover from its release-group id or an artist-and-title search,
  with a search hit accepted only when its title and credited artist match the
  subject.
- A CI workflow that formats, lints, tests, builds, checks and packs the
  plugin, and uploads the archive.

[Unreleased]: https://github.com/hearthsound/plugin-theaudiodb/compare/v0.1.0...HEAD
