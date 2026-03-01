use chrono::{DateTime, Utc};
use std::str::FromStr;
use utils::app_timezone;

pub(crate) const FALLBACK_LOOP_SECONDS: u64 = 5;
pub(crate) const SEEN_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;

pub(crate) fn compute_next_run(cron_expr: &str, now: DateTime<Utc>) -> DateTime<Utc> {
    if let Ok(seconds) = cron_expr.trim().parse::<u64>() {
        return now + chrono::Duration::seconds(seconds.max(5) as i64);
    }

    if let Some(seconds) = parse_every_seconds(cron_expr) {
        return now + chrono::Duration::seconds(seconds.max(5) as i64);
    }

    match cron::Schedule::from_str(cron_expr) {
        Ok(schedule) => {
            let local_now = app_timezone().utc_to_fixed_local(now);
            schedule
                .after(&local_now)
                .next()
                .map(|next| next.with_timezone(&Utc))
                .unwrap_or_else(|| now + chrono::Duration::seconds(60))
        }
        Err(_) => now + chrono::Duration::seconds(60),
    }
}

fn parse_every_seconds(expr: &str) -> Option<u64> {
    let value = expr.trim();
    let body = value.strip_prefix("@every ")?.trim();

    if let Some(raw) = body.strip_suffix('s') {
        return raw.trim().parse::<u64>().ok();
    }
    if let Some(raw) = body.strip_suffix('m') {
        return raw
            .trim()
            .parse::<u64>()
            .ok()
            .map(|minutes| minutes.saturating_mul(60));
    }
    if let Some(raw) = body.strip_suffix('h') {
        return raw
            .trim()
            .parse::<u64>()
            .ok()
            .map(|hours| hours.saturating_mul(60 * 60));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_compute_next_run_with_seconds_literal() {
        let now = Utc.with_ymd_and_hms(2026, 2, 27, 12, 0, 0).unwrap();
        let next = compute_next_run("30", now);
        assert_eq!(next, now + chrono::Duration::seconds(30));
    }

    #[test]
    fn test_compute_next_run_with_every_syntax() {
        let now = Utc.with_ymd_and_hms(2026, 2, 27, 12, 0, 0).unwrap();
        let next = compute_next_run("@every 2m", now);
        assert_eq!(next, now + chrono::Duration::seconds(120));
    }

    #[test]
    fn test_compute_next_run_with_invalid_expression() {
        let now = Utc.with_ymd_and_hms(2026, 2, 27, 12, 0, 0).unwrap();
        let next = compute_next_run("not-a-cron", now);
        assert_eq!(next, now + chrono::Duration::seconds(60));
    }

    #[test]
    fn test_compute_next_run_with_cron_expression() {
        let now = Utc.with_ymd_and_hms(2026, 2, 27, 12, 0, 0).unwrap();
        let next = compute_next_run("0 */5 * * * * *", now);
        assert_eq!(next, now + chrono::Duration::seconds(300));
    }
}
