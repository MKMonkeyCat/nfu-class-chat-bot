use reqwest::StatusCode;

pub(crate) const MISSING_TARGET_ERROR: &str =
    "missing discord target: set discord_channel_id or discord_webhook_url";

pub(crate) fn is_missing_target_error(error: &str) -> bool {
    error.contains(MISSING_TARGET_ERROR)
}

pub(crate) fn bad_gateway_from_error(_error: &str) -> StatusCode {
    StatusCode::BAD_GATEWAY
}
