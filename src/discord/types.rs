pub(crate) struct DiscordDeliveryTarget {
    pub(crate) channel_id: u64,
    pub(crate) webhook_url: String,
    pub(crate) webhook_avatar_url: String,
}

#[derive(Clone)]
pub(crate) struct DiscordEmbedField {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) inline: bool,
}

pub(crate) struct DiscordEmbedPayload {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) url: String,
    pub(crate) color: u32,
    pub(crate) fields: Vec<DiscordEmbedField>,
    pub(crate) footer: String,
}
