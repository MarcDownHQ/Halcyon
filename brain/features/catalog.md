# Catalog (Planned)

How the configured channels, playlists and videos become the catalog, from [SPEC](../specs/SPEC.md) sections 2 and
4. M1 builds the RSS slice, M2 the full listing.

## Fields

`channels`, `playlists`, `live`, `videos`, `tabs`, `max-episodes` and `quality` are free text (the contract has no
list or boolean kind), split on newlines, commas or spaces. At least one of the four list fields must be set, or
`load-catalog` answers `setting("channels")`. An entry that cannot be resolved is logged and skipped; the field
fails with `setting("<key>")` only when every entry fails.

## What becomes what

| YouTube | Record | `id` | `stream` |
| --- | --- | --- | --- |
| A configured channel | `series`, group "YouTube" | `ch:<UCid>` | - |
| A configured playlist | `series`, group "YouTube playlists" | `pl:<playlistId>` | - |
| An upload or playlist item | `episode`, no season or number | `<videoId>` | `v:<videoId>` |
| A single video | `movie` | `<videoId>` | `v:<videoId>` |
| A live on air, a pinned 24/7 live | `channel` | see [live.md](live.md) | |

## Resolving a channel (M1)

A `UC…` id in the input is used as is. Otherwise the channel's `/videos` HTML page gives the id
(`metadata.channelMetadataRenderer.externalId`, or the canonical link) and doubles as the first uploads page;
`/youtubei/v1/navigation/resolve_url` is the fallback. Handle to id is kept with no expiry. HTML fetches send
`hl=en`, `gl=US` and a consent cookie so text fields parse in English.

## Uploads

- **M1, RSS.** `https://www.youtube.com/feeds/videos.xml?channel_id=UC…`: exact `published` dates, titles,
  thumbnails, descriptions; no duration, no live or Shorts flag.
- **M2, the tabs.** The `/videos` and `/streams` HTML tabs (and `/shorts` if `tabs` asks for it), then `/browse`
  continuations up to `max-episodes`. Items come as `lockupViewModel` today, with older renderers as fallbacks.
  Dates in tab listings are relative ("3 days ago"); RSS dates replace them where the video is in the feed.
  Members-only items, ads, mixes and upcoming items more than a day away are left out.
- **Fallback**: the `UU…` uploads playlist, whose items carry `lengthSeconds`.
- Hard-coded tab `params` heal themselves: a tab that comes back not `selected` means taking the tab's own `params`
  from an unparametrised browse.

## Playlists (M2)

`/playlist?list=<id>` or `/browse` with `VL<id>`; items are `playlistVideoRenderer` with `lengthSeconds`.

## Open points

The channel avatar for the series poster is not documented by any source (TODO, Q11).

## Sources

None yet; lands with M1 (RSS) and M2 (tabs, playlists).
