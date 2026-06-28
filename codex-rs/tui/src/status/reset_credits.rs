//! Display shaping for earned usage-limit reset credits in `/status`.

use super::rate_limits::RateLimitSnapshotDisplay;
use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use codex_app_server_protocol::RateLimitResetCreditStatus;
use codex_app_server_protocol::RateLimitResetCreditsSummary;
use codex_app_server_protocol::RateLimitResetType;
use codex_protocol::account::PlanType;

pub(crate) const RESET_CREDITS_LABEL: &str = "Usage limit resets";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RateLimitResetScope {
    Monthly,
    WeeklyAndFiveHour,
}

impl RateLimitResetScope {
    pub(crate) fn status_label(self) -> &'static str {
        match self {
            Self::Monthly => "Full reset (Monthly)",
            Self::WeeklyAndFiveHour => "Full reset (Weekly + 5h)",
        }
    }

    pub(crate) fn usage_description(self) -> &'static str {
        match self {
            Self::Monthly => "Reset your current monthly usage limit.",
            Self::WeeklyAndFiveHour => "Reset your current 5-hour and weekly usage limits.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusResetCreditsData {
    pub summary: String,
    pub details: Vec<StatusResetCreditDetail>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusResetCreditDetail {
    pub text: String,
    pub scope: Option<&'static str>,
}

pub(crate) fn rate_limit_reset_scope<'a>(
    rate_limits: impl IntoIterator<Item = &'a RateLimitSnapshotDisplay>,
    plan_type: Option<PlanType>,
) -> RateLimitResetScope {
    let has_monthly_window = rate_limits
        .into_iter()
        .find(|snapshot| snapshot.limit_id.eq_ignore_ascii_case("codex"))
        .into_iter()
        .flat_map(|snapshot| [snapshot.primary.as_ref(), snapshot.secondary.as_ref()])
        .flatten()
        .any(|window| {
            crate::chatwidget::limit_label_for_window(
                window.window_minutes,
                /*is_secondary*/ false,
            ) == "monthly"
        });

    if has_monthly_window || matches!(plan_type, Some(PlanType::Free | PlanType::Go)) {
        RateLimitResetScope::Monthly
    } else {
        RateLimitResetScope::WeeklyAndFiveHour
    }
}

pub(crate) fn compose_reset_credits_data(
    summary: Option<&RateLimitResetCreditsSummary>,
    rate_limits: &[RateLimitSnapshotDisplay],
    plan_type: Option<PlanType>,
) -> Option<StatusResetCreditsData> {
    let summary = summary?;
    let available_count = summary.available_count.max(0);
    let value = if available_count == 0 {
        "none available".to_string()
    } else {
        format!("{available_count} available")
    };

    let Some(credits) = summary.credits.as_ref() else {
        return Some(StatusResetCreditsData {
            summary: value,
            details: Vec::new(),
        });
    };
    if available_count == 0 {
        return Some(StatusResetCreditsData {
            summary: value,
            details: Vec::new(),
        });
    }

    let scope = rate_limit_reset_scope(rate_limits, plan_type).status_label();
    let detail_limit = usize::try_from(available_count).unwrap_or(usize::MAX);
    let mut available_credits = credits
        .iter()
        .filter(|credit| credit.status == RateLimitResetCreditStatus::Available)
        .collect::<Vec<_>>();
    available_credits.sort_by_key(|credit| credit.expires_at.unwrap_or(i64::MAX));

    let mut details = available_credits
        .into_iter()
        .take(detail_limit)
        .enumerate()
        .map(|(index, credit)| {
            let number = index + 1;
            let text = match credit.expires_at {
                Some(expires_at) => DateTime::<Utc>::from_timestamp(expires_at, 0)
                    .map(|expires_at| {
                        format!(
                            "#{number} expires {}",
                            expires_at
                                .with_timezone(&Local)
                                .format("%H:%M on %-d %b %Y")
                        )
                    })
                    .unwrap_or_else(|| format!("#{number} has an unknown expiration")),
                None => format!("#{number} does not expire"),
            };
            let scope = match credit.reset_type {
                RateLimitResetType::CodexRateLimits => Some(scope),
            };
            StatusResetCreditDetail { text, scope }
        })
        .collect::<Vec<_>>();

    let detailed_count = i64::try_from(details.len()).unwrap_or(i64::MAX);
    let omitted_count = available_count.saturating_sub(detailed_count);
    if omitted_count > 0 {
        details.push(StatusResetCreditDetail {
            text: format!("+{omitted_count} more without expiry details"),
            scope: None,
        });
    }

    Some(StatusResetCreditsData {
        summary: value,
        details,
    })
}

#[cfg(test)]
#[path = "reset_credits_tests.rs"]
mod tests;
