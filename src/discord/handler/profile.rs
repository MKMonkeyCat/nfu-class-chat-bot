use config::AppConfig;
use entity::model::guild_member::{self, MemberIdentity};
use entity::prelude::GuildMember;
use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use std::collections::HashMap;

pub(super) struct BindingResult {
    pub(super) nickname: String,
    pub(super) sid: String,
    pub(super) name: String,
    pub(super) roles: Vec<u64>,
    pub(super) valid: bool,
}

pub(super) fn compute_binding(
    role_type: &str,
    inputs: &HashMap<String, String>,
    display_name: &str,
    config: &AppConfig,
) -> BindingResult {
    let mut roles = vec![config.roles.verified_role];

    match role_type {
        "local" => {
            let sid = input_value(inputs, "sid_input");
            let name = input_value(inputs, "name_input");

            if !check_student_id(&sid)
                || !config
                    .class_students
                    .get(&sid)
                    .map(|expected_name| expected_name == &name)
                    .unwrap_or(false)
            {
                return invalid_binding(roles);
            }

            roles.push(config.roles.local_role);
            BindingResult {
                nickname: format!("{} ({})", display_name, &sid[sid.len().saturating_sub(2)..]),
                sid,
                name,
                roles,
                valid: true,
            }
        }
        "senior" => {
            let name = input_value(inputs, "name_input");
            let sid = input_value(inputs, "sid_input");
            let dept = input_value(inputs, "dept_input");

            if !check_student_id(&sid) {
                return invalid_binding(roles);
            }

            roles.push(config.roles.senior_role);
            BindingResult {
                nickname: format!("[{}] {}", dept, display_name),
                sid,
                name,
                roles,
                valid: true,
            }
        }
        "teacher" => {
            let name = input_value(inputs, "name_input");
            let title = input_value(inputs, "info_input");
            let dept = input_value(inputs, "dept_input");

            roles.push(config.roles.teacher_role);
            BindingResult {
                nickname: format!("{} ({})", display_name, title),
                sid: format!("T-{}", dept),
                name,
                roles,
                valid: true,
            }
        }
        _ => BindingResult {
            nickname: display_name.to_string(),
            sid: String::new(),
            name: String::new(),
            roles,
            valid: true,
        },
    }
}

pub(super) async fn upsert_member_binding(
    db: &DatabaseConnection,
    user_id: i64,
    role_type: &str,
    name: String,
    sid: String,
) {
    let _ = GuildMember::insert(guild_member::ActiveModel {
        user_id: Set(user_id),
        name: Set(name),
        employee_id: Set(sid),
        identity: Set(MemberIdentity::from_str(role_type)),
    })
    .on_conflict(
        OnConflict::column(guild_member::Column::UserId)
            .update_columns([
                guild_member::Column::Name,
                guild_member::Column::EmployeeId,
                guild_member::Column::Identity,
            ])
            .to_owned(),
    )
    .exec(db)
    .await;
}

fn invalid_binding(roles: Vec<u64>) -> BindingResult {
    BindingResult {
        nickname: String::new(),
        sid: String::new(),
        name: String::new(),
        roles,
        valid: false,
    }
}

fn input_value(inputs: &HashMap<String, String>, key: &str) -> String {
    inputs
        .get(key)
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

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
