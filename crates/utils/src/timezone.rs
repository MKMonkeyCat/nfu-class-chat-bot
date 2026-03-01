use chrono::{DateTime, FixedOffset, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

#[derive(Clone, Copy)]
pub enum AppTimeZone {
    Named(Tz),
    Fixed(FixedOffset),
}

impl AppTimeZone {
    pub fn now_utc(self) -> DateTime<Utc> {
        Utc::now()
    }

    pub fn today_str(self) -> String {
        match self {
            Self::Named(tz) => Utc::now().with_timezone(&tz).format("%Y-%m-%d").to_string(),
            Self::Fixed(offset) => Utc::now()
                .with_timezone(&offset)
                .format("%Y-%m-%d")
                .to_string(),
        }
    }

    pub fn utc_to_fixed_local(self, utc: DateTime<Utc>) -> DateTime<FixedOffset> {
        match self {
            Self::Named(tz) => utc.with_timezone(&tz).fixed_offset(),
            Self::Fixed(offset) => utc.with_timezone(&offset),
        }
    }

    pub fn naive_local_to_utc(self, naive: NaiveDateTime) -> Option<DateTime<Utc>> {
        match self {
            Self::Named(tz) => {
                choose_local(tz.from_local_datetime(&naive)).map(|dt| dt.with_timezone(&Utc))
            }
            Self::Fixed(offset) => {
                choose_local(offset.from_local_datetime(&naive)).map(|dt| dt.with_timezone(&Utc))
            }
        }
    }
}

pub fn app_timezone() -> AppTimeZone {
    std::env::var("TZ")
        .ok()
        .as_deref()
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .and_then(parse_timezone)
        .unwrap_or(AppTimeZone::Named(chrono_tz::Asia::Taipei))
}

fn parse_timezone(raw: &str) -> Option<AppTimeZone> {
    if let Ok(tz) = raw.parse::<Tz>() {
        return Some(AppTimeZone::Named(tz));
    }

    parse_fixed_offset(raw).map(AppTimeZone::Fixed)
}

fn parse_fixed_offset(raw: &str) -> Option<FixedOffset> {
    let value = raw
        .trim()
        .strip_prefix("UTC")
        .or_else(|| raw.trim().strip_prefix("utc"))
        .or_else(|| raw.trim().strip_prefix("GMT"))
        .or_else(|| raw.trim().strip_prefix("gmt"))
        .unwrap_or(raw.trim());

    let normalized = if let Some(rest) = value.strip_prefix('+') {
        format!("+{}", rest)
    } else if let Some(rest) = value.strip_prefix('-') {
        format!("-{}", rest)
    } else if value.chars().all(|c| c.is_ascii_digit()) {
        format!("+{}", value)
    } else {
        value.to_string()
    };

    let sign = if normalized.starts_with('-') { -1 } else { 1 };
    let body = normalized
        .trim_start_matches('+')
        .trim_start_matches('-')
        .trim();

    let (hours, minutes) = if let Some((h, m)) = body.split_once(':') {
        let hours = h.trim().parse::<i32>().ok()?;
        let minutes = m.trim().parse::<i32>().ok()?;
        (hours, minutes)
    } else if body.len() == 4 && body.chars().all(|c| c.is_ascii_digit()) {
        let hours = body[0..2].parse::<i32>().ok()?;
        let minutes = body[2..4].parse::<i32>().ok()?;
        (hours, minutes)
    } else {
        let hours = body.parse::<i32>().ok()?;
        (hours, 0)
    };

    let total_seconds = sign * (hours * 3600 + minutes * 60);
    if total_seconds < -86_400 || total_seconds > 86_400 {
        return None;
    }

    FixedOffset::east_opt(total_seconds)
}

fn choose_local<T>(value: LocalResult<T>) -> Option<T> {
    match value {
        LocalResult::Single(dt) => Some(dt),
        LocalResult::Ambiguous(early, _) => Some(early),
        LocalResult::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timezone_name() {
        let parsed = parse_timezone("Asia/Taipei").expect("timezone should parse");
        match parsed {
            AppTimeZone::Named(tz) => assert_eq!(tz.name(), "Asia/Taipei"),
            _ => panic!("expected named timezone"),
        }
    }

    #[test]
    fn test_parse_timezone_offset() {
        let parsed = parse_timezone("UTC+8").expect("timezone should parse");
        match parsed {
            AppTimeZone::Fixed(offset) => {
                assert_eq!(offset.local_minus_utc(), 8 * 3600)
            }
            _ => panic!("expected fixed offset"),
        }
    }
}
