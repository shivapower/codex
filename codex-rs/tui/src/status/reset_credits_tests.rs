use super::*;
use crate::status::RateLimitWindowDisplay;
use codex_app_server_protocol::RateLimitResetCredit;

fn credit(expires_at: Option<i64>) -> RateLimitResetCredit {
    RateLimitResetCredit {
        id: "credit".to_string(),
        reset_type: RateLimitResetType::CodexRateLimits,
        status: RateLimitResetCreditStatus::Available,
        granted_at: 0,
        expires_at,
    }
}

#[test]
fn count_only_summary_has_no_synthetic_details() {
    let summary = RateLimitResetCreditsSummary {
        available_count: 2,
        credits: None,
    };

    let display = compose_reset_credits_data(Some(&summary), &[], /*plan_type*/ None)
        .expect("reset credits display");

    pretty_assertions::assert_eq!(
        display,
        StatusResetCreditsData {
            summary: "2 available".to_string(),
            details: Vec::new(),
        }
    );
}

#[test]
fn zero_summary_ignores_unexpected_credit_details() {
    let summary = RateLimitResetCreditsSummary {
        available_count: 0,
        credits: Some(vec![credit(Some(/*expires_at*/ 1_800_000_000))]),
    };

    let display = compose_reset_credits_data(Some(&summary), &[], /*plan_type*/ None)
        .expect("reset credits display");

    pretty_assertions::assert_eq!(
        display,
        StatusResetCreditsData {
            summary: "none available".to_string(),
            details: Vec::new(),
        }
    );
}

#[test]
fn truncated_details_explain_the_omitted_count() {
    let summary = RateLimitResetCreditsSummary {
        available_count: 3,
        credits: Some(vec![credit(None)]),
    };

    let display = compose_reset_credits_data(Some(&summary), &[], /*plan_type*/ None)
        .expect("reset credits display");

    pretty_assertions::assert_eq!(
        display.details,
        vec![
            StatusResetCreditDetail {
                text: "#1 does not expire".to_string(),
                scope: Some("Full reset (Weekly + 5h)"),
            },
            StatusResetCreditDetail {
                text: "+2 more without expiry details".to_string(),
                scope: None,
            },
        ]
    );
}

#[test]
fn canonical_codex_id_selects_monthly_window_when_display_name_differs() {
    let snapshot = RateLimitSnapshotDisplay {
        limit_id: "codex".to_string(),
        limit_name: "Codex usage".to_string(),
        captured_at: Local::now(),
        primary: Some(RateLimitWindowDisplay {
            used_percent: 0.0,
            resets_at: None,
            window_minutes: Some(/*window_minutes*/ 43_200),
        }),
        secondary: None,
        credits: None,
        individual_limit: None,
    };

    pretty_assertions::assert_eq!(
        rate_limit_reset_scope([&snapshot], Some(PlanType::Business)),
        RateLimitResetScope::Monthly
    );
}
