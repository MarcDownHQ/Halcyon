//! The component boundary: `wit/provider.wit` exports and imports, adapted to the plain Rust of the crate.

#![allow(clippy::too_many_arguments, clippy::same_length_and_capacity, reason = "the code wit-bindgen generates")]

use crate::model;

wit_bindgen::generate!({ path: "wit", world: "provider" });

struct Halcyon;

fn not_yet() -> Error {
    Error::BadResponse("not implemented yet".into())
}

impl Guest for Halcyon {
    fn describe() -> ProviderInfo {
        let info: model::ProviderInfo = crate::provider::describe();
        ProviderInfo {
            id: info.id,
            name: info.name,
            fields: Vec::new(),
            resolves_streams: info.resolves_streams,
            presets: info
                .presets
                .into_iter()
                .map(|p| nightglass::provider::types::Preset { label: p.label, name: p.name, config: p.config })
                .collect(),
        }
    }

    fn load_catalog(_config: Config) -> Result<Catalog, Error> {
        Err(not_yet())
    }

    fn stream_url(_config: Config, _channel: Channel) -> Result<Playback, Error> {
        Err(not_yet())
    }

    fn catchup_url(_config: Config, _channel: Channel, _start: i64, _end: i64) -> Result<Playback, Error> {
        Err(not_yet())
    }

    fn episodes(_config: Config, _series: Series) -> Result<Vec<Episode>, Error> {
        Err(not_yet())
    }

    fn movie_url(_config: Config, _movie: Movie) -> Result<Playback, Error> {
        Err(not_yet())
    }

    fn episode_url(_config: Config, _series: Series, _episode: Episode) -> Result<Playback, Error> {
        Err(not_yet())
    }

    fn filter(_config: Config, _url: String, media: Media) -> Result<Media, Error> {
        Ok(media)
    }
}

export!(Halcyon);
