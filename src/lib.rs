//! Bunkr downloader core, shared by the CLI (`bunkr`) and the GUI (`bunkr-gui`).

pub mod i18n;
pub mod album;
pub mod config;
pub mod dryrun;
pub mod engine;
pub mod fsutil;
pub mod net;
pub mod parse;
pub mod ratelimit;
pub mod state;
pub mod ui;
pub mod urlutil;
