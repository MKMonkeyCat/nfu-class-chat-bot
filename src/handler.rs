use crate::db::{ActiveModel, Column, Entity};
use crate::state::{ConfigKey, DbKey};
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, Set};
use serenity::all::{
    ActionRowComponent, ButtonStyle, ComponentInteraction, CreateActionRow, CreateButton,
    CreateEmbed, CreateInputText, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, CreateModal, EditMember, GuildId, InputTextStyle, Interaction, Message,
    Permissions, RoleId, UserId,
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::{Context, EventHandler};
use std::collections::HashMap;

pub struct Handler;

// static RE: OnceLock<Regex> = OnceLock::new();
// let sid_regex = RE.get_or_init(|| Regex::new(r"^[345]\d{2}\d{2}[12]\d{2}$").unwrap());
// sid_regex.is_match(sid)
fn check_student_id(sid: &str) -> bool {
    let chars: Vec<char> = sid.chars().collect();
    if chars.len() != 8 {
        return false;
    }
    if !matches!(chars[0], '3' | '4' | '5') {
        return false;
    }
    if !chars.iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    matches!(chars[5], '1' | '2')
}

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

    async fn send_confirm_step(
        &self,
        ctx: &Context,
        component: &ComponentInteraction,
        role_type: &str,
    ) {
        let label = match role_type {
            "local" => "本班學生",
            "senior" => "學長姐",
            "teacher" => "老師",
            "guest" => "路人",
            _ => "未知",
        };

        let content = format!(
            "您目前選擇的是 **{}**，請確認。本操作完成後將無法反悔。",
            label
        );
        let row = CreateActionRow::Buttons(vec![
            CreateButton::new(format!("confirm:{}", role_type))
                .label("確認")
                .style(ButtonStyle::Success),
            CreateButton::new("setup:cancel")
                .label("取消")
                .style(ButtonStyle::Secondary),
        ]);

        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .components(vec![row])
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
        if msg.content == "!setup class_info" {
            if check_with_admin_role(&ctx, &msg).await {
                let _ = msg.delete(&ctx.http).await;
                let embed = CreateEmbed::new()
                    .title("班級系統設定")
                    .description("請選擇您的身分並完成資料綁定")
                    .color(0x3498db);

                let row = CreateActionRow::Buttons(vec![
                    CreateButton::new("setup:local")
                        .label("本班學生")
                        .style(ButtonStyle::Primary),
                    CreateButton::new("setup:senior")
                        .label("學長姐")
                        .style(ButtonStyle::Primary),
                    CreateButton::new("setup:teacher")
                        .label("老師")
                        .style(ButtonStyle::Primary),
                    CreateButton::new("setup:guest")
                        .label("路人")
                        .style(ButtonStyle::Primary),
                ]);

                let _ = msg
                    .channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(embed).components(vec![row]),
                    )
                    .await;
            }
        }
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
                    self.send_confirm_step(&ctx, &component, role_type).await;
                }

                if custom_id.starts_with("confirm:") {
                    let role_type = custom_id.split(':').nth(1).unwrap_or("");
                    match role_type {
                        "local" => {
                            let modal =
                                CreateModal::new("modal:local", "本班學生驗證").components(vec![
                                    CreateActionRow::InputText(
                                        CreateInputText::new(
                                            InputTextStyle::Short,
                                            "請輸入學號",
                                            "sid_input",
                                        )
                                        .required(true),
                                    ),
                                    CreateActionRow::InputText(
                                        CreateInputText::new(
                                            InputTextStyle::Short,
                                            "真實姓名",
                                            "name_input",
                                        )
                                        .required(true),
                                    ),
                                ]);
                            let _ = component
                                .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                                .await;
                        }
                        "senior" => {
                            let modal = CreateModal::new("modal:senior", "學長姐資料綁定")
                                .components(vec![
                                    CreateActionRow::InputText(
                                        CreateInputText::new(
                                            InputTextStyle::Short,
                                            "姓名",
                                            "name_input",
                                        )
                                        .required(true),
                                    ),
                                    CreateActionRow::InputText(
                                        CreateInputText::new(
                                            InputTextStyle::Short,
                                            "學號",
                                            "sid_input",
                                        )
                                        .required(true),
                                    ),
                                    CreateActionRow::InputText(
                                        CreateInputText::new(
                                            InputTextStyle::Short,
                                            "科系與年級",
                                            "dept_input",
                                        )
                                        .required(true),
                                    ),
                                ]);
                            let _ = component
                                .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                                .await;
                        }
                        "teacher" => {
                            let modal = CreateModal::new("modal:teacher", "老師資料綁定")
                                .components(vec![
                                    CreateActionRow::InputText(
                                        CreateInputText::new(
                                            InputTextStyle::Short,
                                            "姓名",
                                            "name_input",
                                        )
                                        .required(true),
                                    ),
                                    CreateActionRow::InputText(
                                        CreateInputText::new(
                                            InputTextStyle::Short,
                                            "職稱",
                                            "info_input",
                                        )
                                        .required(true),
                                    ),
                                    CreateActionRow::InputText(
                                        CreateInputText::new(
                                            InputTextStyle::Short,
                                            "所屬科系",
                                            "dept_input",
                                        )
                                        .required(true),
                                    ),
                                ]);
                            let _ = component
                                .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                                .await;
                        }
                        "guest" => {
                            if let Some(gid) = component.guild_id {
                                let res = self
                                    .update_member_profile(
                                        &ctx,
                                        gid,
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
                        _ => (),
                    }
                }
            }

            Interaction::Modal(modal) => {
                let custom_id = &modal.data.custom_id;
                let role_type = custom_id.split(':').nth(1).unwrap_or("");
                let mut inputs = HashMap::new();
                for row in &modal.data.components {
                    for comp in &row.components {
                        if let ActionRowComponent::InputText(t) = comp {
                            inputs
                                .insert(t.custom_id.as_str(), t.value.clone().unwrap_or_default());
                        }
                    }
                }

                let display_name = modal.user.display_name();
                if let Some(guild_id) = modal.guild_id {
                    let mut final_roles = vec![config.roles.verified_role];
                    let (final_nick, final_sid, final_name, is_valid) = match role_type {
                        "local" => {
                            let sid = inputs
                                .get("sid_input")
                                .cloned()
                                .unwrap_or_default()
                                .trim()
                                .to_string();
                            let name = inputs
                                .get("name_input")
                                .cloned()
                                .unwrap_or_default()
                                .trim()
                                .to_string();

                            if !check_student_id(&sid) {
                                (String::new(), String::new(), String::new(), false)
                            } else if !config
                                .class_students
                                .get(&sid)
                                .map(|expected_name| expected_name == &name)
                                .unwrap_or(false)
                            {
                                (String::new(), String::new(), String::new(), false)
                            } else {
                                final_roles.push(config.roles.local_role);
                                (
                                    format!(
                                        "{} ({})",
                                        display_name,
                                        &sid[sid.len().saturating_sub(2)..]
                                    ),
                                    sid,
                                    name,
                                    true,
                                )
                            }
                        }
                        "senior" => {
                            let name = inputs.get("name_input").cloned().unwrap_or_default();
                            let sid = inputs.get("sid_input").cloned().unwrap_or_default();
                            let dept = inputs.get("dept_input").cloned().unwrap_or_default();

                            if !check_student_id(&sid) {
                                (String::new(), String::new(), String::new(), false)
                            } else {
                                final_roles.push(config.roles.senior_role);
                                (format!("[{}] {}", dept, display_name), sid, name, true)
                            }
                        }
                        "teacher" => {
                            let name = inputs.get("name_input").cloned().unwrap_or_default();
                            let title = inputs.get("info_input").cloned().unwrap_or_default();
                            let dept = inputs.get("dept_input").cloned().unwrap_or_default();
                            final_roles.push(config.roles.teacher_role);
                            (
                                format!("{} ({})", display_name, title),
                                format!("T-{}", dept),
                                name,
                                true,
                            )
                        }
                        _ => (display_name.to_string(), String::new(), String::new(), true),
                    };

                    if !is_valid {
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

                    let _ = Entity::insert(ActiveModel {
                        user_id: Set(modal.user.id.get() as i64),
                        name: Set(final_name),
                        employee_id: Set(final_sid),
                        identity: Set(role_type.to_string()),
                    })
                    .on_conflict(
                        OnConflict::column(Column::UserId)
                            .update_columns([Column::Name, Column::EmployeeId, Column::Identity])
                            .to_owned(),
                    )
                    .exec(&db)
                    .await;

                    let res = self
                        .update_member_profile(
                            &ctx,
                            guild_id,
                            modal.user.id,
                            final_nick,
                            final_roles,
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
            _ => (),
        }
    }
}
