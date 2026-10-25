# Hearthsound TheAudioDB plugin

A [Hearthsound](https://github.com/hearthsound) metadata and artwork plugin
backed by [TheAudioDB](https://www.theaudiodb.com/)'s public v1 JSON API. It
compiles to a WebAssembly component that implements the `metadata` and
`artwork` provider interfaces, so a Hearthsound server can show a biography and
reference links for an artist, and use its portrait and album covers.

The plugin needs no account and no paid key. The public test key `123` is enough
to evaluate it; a premium key raises the request limit and the size of a search
result page.

## Requirements

- Rust 1.99 or newer, with the `wasm32-wasip2` target. The included
  `rust-toolchain.toml` pins both; `rustup toolchain install` fetches them.

## Build

```console
cargo build --release --locked --target wasm32-wasip2
```

The component lands at
`target/wasm32-wasip2/release/hearthsound_plugin_theaudiodb.wasm`.

## Check and pack

Install the plugin CLI from the SDK repository:

```console
cargo install --locked --git https://github.com/hearthsound/plugin-sdk --tag v0.4.0 hearthsound-plugin-cli
```

Then validate the manifest and the component:

```console
hsplugin check
```

It prints `ok: app.hearthsound.theaudiodb 0.2.0 (provides metadata, artwork)`
on success.

Finally, build the archive you install on a server:

```console
hsplugin pack --out-dir dist
```

`dist/app.hearthsound.theaudiodb-0.2.0.hsplugin` contains the manifest and the
component.

## Tests

The response mapping and the lookup policy run on the host target, so their
unit tests need no WebAssembly toolchain and no network. They run against JSON
fixtures captured from the public API:

```console
cargo test
```

## Configuration

| Instance setting | Default | Meaning |
|---|---|---|
| `apiKey` | `123` | TheAudioDB API key, sent as a path segment, and `writeOnly`: the server seals it at rest and shows `[sealed]` on every read. The public test key works for evaluation; a premium key raises the service limit and returns more search results. |
| `baseUrl` | `https://www.theaudiodb.com/api/v1/json/` | Base URL of the v1 JSON API. Point it at a mirror or a test double to keep traffic off theaudiodb.com. |

The instance requests a `host-http` rate limit of one request per second with a
burst of one: at most 60 requests a minute. TheAudioDB's free tier allows 30 a
minute, so a free key can answer `429` during a burst; a premium key raises the
service limit.

## Notes

- The API key travels in the URL path, not in a header. The plugin encodes it
  and every query value before building a request.
- `get-artist-info` reads the artist the server's `mb-artist` id names; when
  that lookup finds nothing, or the server holds no id, it searches the artist
  name instead. A search hit counts only when its primary or alternate name
  spells the requested name the same way, ignoring case and whitespace, so a
  namesake or a tribute act is never shown as the artist.
- The biography is the English column. The current API serves it in
  `strBiography`; the plugin also reads the older `strBiographyEN` spelling
  first, so it works against both. The links come from the row's profile
  columns: `strWebsite` as `homepage` and `strLastFMChart` as `lastfm`.
  Facebook and Twitter have no matching link kind and are not reported, and the
  artist response carries no Wikipedia, Wikidata, Discogs or AllMusic column.
- TheAudioDB is not a recording catalog: `resolve-recording` answers no matches
  and `get-release` answers not-found, so the server moves on to the next
  metadata instance in the layer order.
- `find-artwork` answers an artist's portrait from `strArtistThumb` and an
  album's front cover from `strAlbumThumb` (or `strAlbumThumbHQ` when the
  standard column is empty). A subject is looked up by the instance's own id,
  then a MusicBrainz artist or release-group id, then an artist-and-title
  search. A search hit counts only when its title, and the artist it credits,
  are the ones the subject names, so a remix or a namesake is never used.
- A subject the service does not know, and a row without the picture column,
  answer an empty image list rather than an error: the server then asks the
  next artwork instance.
- An unknown artist is an HTTP 200 response whose collection is `null`, not an
  error. The plugin answers "no artist" for it, and translates a 404 (an
  unrecognized key or an endpoint the shared service does not serve), a 429
  (rate limited, with the `Retry-After` delay when present) and a 503 (shared
  service unavailable) into the provider errors that describe them.

## Layout

| Path | Contents |
|---|---|
| `Cargo.toml` | Crate metadata and the SDK dependency. |
| `src/lib.rs` | The plugin: configuration, lifecycle and the metadata interface. |
| `src/api.rs` | The HTTP transport, request paths and TheAudioDB's error statuses. |
| `src/audiodb.rs` | Serde types for TheAudioDB's v1 JSON responses. |
| `src/artwork.rs` | The artwork operations: artist portraits and album covers. |
| `src/map.rs` | TheAudioDB rows to provider records: biography, identifiers, links and images. |
| `src/metadata.rs` | The metadata operations: artist lookup with the name-search fallback. |
| `src/url.rs` | Base URL validation, endpoint composition and query encoding. |
| `manifest.json` | The plugin manifest the server reads. |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
