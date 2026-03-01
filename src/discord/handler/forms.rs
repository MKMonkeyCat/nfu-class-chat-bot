use serenity::all::{
    ActionRowComponent, ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateInputText,
    CreateModal, InputTextStyle, ModalInteraction,
};
use std::collections::HashMap;

pub(super) fn role_label(role_type: &str) -> &'static str {
    match role_type {
        "local" => "本班學生",
        "senior" => "學長姐",
        "teacher" => "老師",
        "guest" => "路人",
        _ => "未知",
    }
}

pub(super) fn setup_embed() -> CreateEmbed {
    CreateEmbed::new()
        .title("班級系統設定")
        .description("請選擇您的身分並完成資料綁定")
        .color(0x3498db)
}

pub(super) fn setup_action_row() -> CreateActionRow {
    CreateActionRow::Buttons(vec![
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
    ])
}

pub(super) fn confirm_action_row(role_type: &str) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(format!("confirm:{}", role_type))
            .label("確認")
            .style(ButtonStyle::Success),
        CreateButton::new("setup:cancel")
            .label("取消")
            .style(ButtonStyle::Secondary),
    ])
}

pub(super) fn build_role_modal(role_type: &str) -> Option<CreateModal> {
    match role_type {
        "local" => Some(
            CreateModal::new("modal:local", "本班學生驗證").components(vec![
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "請輸入學號", "sid_input")
                        .required(true),
                ),
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "真實姓名", "name_input")
                        .required(true),
                ),
            ]),
        ),
        "senior" => Some(
            CreateModal::new("modal:senior", "學長姐資料綁定").components(vec![
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "姓名", "name_input")
                        .required(true),
                ),
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "學號", "sid_input").required(true),
                ),
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "科系與年級", "dept_input")
                        .required(true),
                ),
            ]),
        ),
        "teacher" => Some(
            CreateModal::new("modal:teacher", "老師資料綁定").components(vec![
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "姓名", "name_input")
                        .required(true),
                ),
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "職稱", "info_input")
                        .required(true),
                ),
                CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "所屬科系", "dept_input")
                        .required(true),
                ),
            ]),
        ),
        _ => None,
    }
}

pub(super) fn collect_modal_inputs(modal: &ModalInteraction) -> HashMap<String, String> {
    let mut inputs = HashMap::new();
    for row in &modal.data.components {
        for comp in &row.components {
            if let ActionRowComponent::InputText(input) = comp {
                inputs.insert(
                    input.custom_id.clone(),
                    input.value.clone().unwrap_or_default(),
                );
            }
        }
    }
    inputs
}
