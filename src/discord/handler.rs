mod forms;
mod profile;

use crate::state::{ConfigKey, DbKey};
use config::AppConfig;
use forms::{
    build_role_modal, collect_modal_inputs, confirm_action_row, role_label, setup_action_row,
    setup_embed,
};
use profile::{compute_binding, upsert_member_binding};
use sea_orm::DatabaseConnection;
use serenity::all::{
    ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, EditMember, GuildId, Interaction, Message, ModalInteraction, Permissions,
    RoleId, UserId,
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::{Context, EventHandler};

pub struct Handler;

impl Handler {
    async fn update_member_profile(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        nickname: String,
        roles: Vec<u64>,
    ) -> String {
        let mut msg = String::new();

        match guild_id
            .edit_member(&ctx.http, user_id, EditMember::new().nickname(&nickname))
            .await
        {
            Ok(_) => msg.push_str(&format!("\n[成功] 暱稱已更新為: {}", nickname)),
            Err(err) => {
                eprintln!("[Error] Nickname failed: {:?}", err);
                msg.push_str("\n[錯誤] 暱稱修改失敗");
            }
        }

        for role_id in roles {
            let rid = RoleId::new(role_id);
            if let Err(e) = ctx.http.add_member_role(guild_id, user_id, rid, None).await {
                eprintln!("[Error] Role {} failed: {:?}", role_id, e);
                msg.push_str(&format!("\n[錯誤] 身分組 {} 分配失敗", role_id));
            }
        }
        msg.push_str("\n[成功] 身分組分配完成");
        msg
    }

    async fn handle_setup_command(&self, ctx: &Context, msg: &Message) {
        if msg.content != "!setup class_info" {
            return;
        }

        if !check_with_admin_role(ctx, msg).await {
            return;
        }

        let _ = msg.delete(&ctx.http).await;
        let _ = msg
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new()
                    .embed(setup_embed())
                    .components(vec![setup_action_row()]),
            )
            .await;
    }

    async fn send_confirm_step(
        &self,
        ctx: &Context,
        component: &ComponentInteraction,
        role_type: &str,
    ) {
        let label = role_label(role_type);

        let content = format!(
            "您目前選擇的是 **{}**，請確認。本操作完成後將無法反悔。",
            label
        );

        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .components(vec![confirm_action_row(role_type)])
                        .ephemeral(true),
                ),
            )
            .await;
    }

    async fn handle_component_interaction(
        &self,
        ctx: &Context,
        component: ComponentInteraction,
        config: &AppConfig,
    ) {
        let custom_id = &component.data.custom_id;

        if custom_id.starts_with("setup:") {
            let role_type = custom_id.split(':').nth(1).unwrap_or("");
            if role_type == "cancel" {
                let _ = component
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .content("操作已取消")
                                .components(vec![]),
                        ),
                    )
                    .await;
                return;
            }

            self.send_confirm_step(ctx, &component, role_type).await;
            return;
        }

        if !custom_id.starts_with("confirm:") {
            return;
        }

        let role_type = custom_id.split(':').nth(1).unwrap_or("");
        match role_type {
            "guest" => {
                if let Some(guild_id) = component.guild_id {
                    let res = self
                        .update_member_profile(
                            ctx,
                            guild_id,
                            component.user.id,
                            component.user.name.clone(),
                            vec![config.roles.verified_role, config.roles.guest_role],
                        )
                        .await;

                    let _ = component
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::UpdateMessage(
                                CreateInteractionResponseMessage::new()
                                    .content(format!("已完成路人驗證{}", res))
                                    .components(vec![]),
                            ),
                        )
                        .await;
                }
            }
            _ => {
                if let Some(modal) = build_role_modal(role_type) {
                    let _ = component
                        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                        .await;
                }
            }
        }
    }

    async fn handle_modal_interaction(
        &self,
        ctx: &Context,
        modal: ModalInteraction,
        config: &AppConfig,
        db: &DatabaseConnection,
    ) {
        let role_type = modal.data.custom_id.split(':').nth(1).unwrap_or("");
        let inputs = collect_modal_inputs(&modal);
        let display_name = modal.user.display_name();

        let Some(guild_id) = modal.guild_id else {
            return;
        };

        let binding = compute_binding(role_type, &inputs, display_name, config);
        if !binding.valid {
            let _ = modal
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("學號/姓名驗證失敗，非本班學生或請在輸入一遍")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        upsert_member_binding(
            db,
            modal.user.id.get() as i64,
            role_type,
            binding.name.clone(),
            binding.sid.clone(),
        )
        .await;

        let res = self
            .update_member_profile(
                ctx,
                guild_id,
                modal.user.id,
                binding.nickname,
                binding.roles,
            )
            .await;

        let _ = modal
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("資料綁定完成{}", res))
                        .ephemeral(true),
                ),
            )
            .await;
    }
}

async fn check_with_admin_role(ctx: &Context, msg: &Message) -> bool {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return false,
    };
    let member = match guild_id.member(&ctx.http, &msg.author.id).await {
        Ok(m) => m,
        Err(_) => return false,
    };
    if let Some(guild) = ctx.cache.guild(guild_id) {
        if let Some(channel) = guild.channels.get(&msg.channel_id) {
            let perms = guild.user_permissions_in(channel, &member);
            if perms.contains(Permissions::ADMINISTRATOR)
                || perms.contains(Permissions::MANAGE_GUILD)
            {
                return true;
            }
        }
    }
    let config = {
        let data = ctx.data.read().await;
        let config_arc = data.get::<ConfigKey>().expect("Config missing").clone();
        config_arc.read().await.clone()
    };
    for &role_id in &config.roles.admin_roles {
        if member.roles.contains(&RoleId::new(role_id)) {
            return true;
        }
    }
    false
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("[Info] {} is online", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        self.handle_setup_command(&ctx, &msg).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let (config, db) = {
            let data = ctx.data.read().await;
            let config_arc = data.get::<ConfigKey>().expect("Config missing").clone();
            let db = data.get::<DbKey>().expect("DB missing").clone();
            (config_arc.read().await.clone(), db)
        };

        match interaction {
            Interaction::Component(component) => {
                self.handle_component_interaction(&ctx, component, &config)
                    .await;
            }
            Interaction::Modal(modal) => {
                self.handle_modal_interaction(&ctx, modal, &config, &db)
                    .await
            }
            _ => (),
        }
    }
}
