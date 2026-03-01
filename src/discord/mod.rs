pub(crate) mod delivery;
pub(crate) mod handler;
pub(crate) mod types;
pub(crate) mod utils;

pub(crate) use delivery::{send_embed, send_file, send_text};
pub(crate) use handler::Handler;
pub(crate) use types::{DiscordDeliveryTarget, DiscordEmbedField, DiscordEmbedPayload};
pub(crate) use utils::{bad_gateway_from_error, is_missing_target_error};
