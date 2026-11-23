//! The plugin's exports, as plain Rust.

use crate::model::ProviderInfo;

pub const ID: &str = "halcyon";
pub const NAME: &str = "Halcyon";

pub fn describe() -> ProviderInfo {
    ProviderInfo { id: ID.into(), name: NAME.into(), resolves_streams: true, presets: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_a_provider_that_resolves_at_play_time() {
        let info = describe();
        assert_eq!((info.id.as_str(), info.name.as_str()), ("halcyon", "Halcyon"));
        assert!(info.resolves_streams);
        assert_eq!(info.presets, Vec::new());
    }
}
