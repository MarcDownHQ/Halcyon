//! The records of `wit/provider.wit` the plugin produces, as plain Rust.

pub type Config = Vec<(String, String)>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    pub label: String,
    pub name: String,
    pub config: Config,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub resolves_streams: bool,
    pub presets: Vec<Preset>,
}
