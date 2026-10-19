//! Shared catalog and wire contract for the voluntary weekly practice feature.

use serde::{Deserialize, Serialize};

/// Preserve older catalog definitions when publishing changed display copy.
pub const PRACTICE_CATALOG_VERSION: &str = "1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeAction {
    pub code: &'static str,
    pub label: &'static str,
    pub instruction: &'static str,
    pub is_small: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeGoal {
    pub code: &'static str,
    pub label: &'static str,
    pub actions: [PracticeAction; 2],
}

pub const PRACTICE_CATALOG: [PracticeGoal; 3] = [
    PracticeGoal {
        code: "express_need",
        label: "說清楚自己的需求",
        actions: [
            PracticeAction {
                code: "write_one_sentence",
                label: "先寫一句",
                instruction: "下次要討論事情前，先寫下一句自己想表達的需求。",
                is_small: true,
            },
            PracticeAction {
                code: "say_one_sentence",
                label: "說出一句",
                instruction: "下次討論時，說出一句自己想表達的需求。",
                is_small: false,
            },
        ],
    },
    PracticeGoal {
        code: "start_task",
        label: "開始一件拖延的事",
        actions: [
            PracticeAction {
                code: "prepare_first_step",
                label: "先準備一樣東西",
                instruction: "前一晚準備好隔天開始所需的一樣物品或資料。",
                is_small: true,
            },
            PracticeAction {
                code: "ten_minute_start",
                label: "開始第一步",
                instruction: "選一件事，只做可以在十分鐘內開始的第一步。",
                is_small: false,
            },
        ],
    },
    PracticeGoal {
        code: "review_choice",
        label: "想清楚一個選擇",
        actions: [
            PracticeAction {
                code: "name_one_tradeoff",
                label: "先說一項取捨",
                instruction: "面對一個非緊急選擇時，先說出自己最在意的一項取捨。",
                is_small: true,
            },
            PracticeAction {
                code: "two_options",
                label: "列出兩個選項",
                instruction: "做非緊急決定前列出兩個選項，各寫一個好處與代價。",
                is_small: false,
            },
        ],
    },
];

/// A stored plan must render with the catalog version it was created with.
/// Keep prior catalogs here if a future release changes action copy or codes.
pub fn catalog_for_version(version: &str) -> Option<&'static [PracticeGoal; 3]> {
    (version == PRACTICE_CATALOG_VERSION).then_some(&PRACTICE_CATALOG)
}

pub fn valid_goal_action(goal_code: &str, action_code: &str) -> bool {
    PRACTICE_CATALOG.iter().any(|goal| {
        goal.code == goal_code && goal.actions.iter().any(|action| action.code == action_code)
    })
}

pub fn smaller_action(goal_code: &str, action_code: &str) -> Option<&'static str> {
    let goal = PRACTICE_CATALOG
        .iter()
        .find(|goal| goal.code == goal_code)?;
    let action = goal
        .actions
        .iter()
        .find(|action| action.code == action_code)?;
    (!action.is_small).then_some(goal.actions[0].code)
}

pub fn valid_helpfulness(code: &str) -> bool {
    matches!(code, "helped" | "unsure" | "not_helped")
}

pub fn valid_obstacle(code: &str) -> bool {
    matches!(code, "no_chance" | "forgot" | "too_big" | "other")
}

pub fn valid_attempt_feedback(
    tried: bool,
    helpfulness: Option<&str>,
    obstacle: Option<&str>,
) -> bool {
    if tried {
        helpfulness.is_some_and(valid_helpfulness) && obstacle.is_none()
    } else {
        obstacle.is_some_and(valid_obstacle) && helpfulness.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PracticePlan {
    pub id: String,
    pub goal_code: String,
    pub action_code: String,
    pub catalog_version: String,
    pub created_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeAttempt {
    pub id: String,
    pub plan_id: String,
    pub cycle_id: String,
    pub tried: bool,
    pub helpfulness: Option<String>,
    pub obstacle: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSummary {
    pub cycle_id: String,
    pub attempted: u32,
    pub helped: u32,
    pub unsure: u32,
    pub not_helped: u32,
    pub not_tried: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeOverview {
    pub cycle_id: String,
    pub practice_epoch: i64,
    pub total_plans: u32,
    pub total_attempts: u32,
    pub current_week: PracticeSummary,
    pub previous_week: PracticeSummary,
    pub active_plan: Option<PracticePlan>,
    pub recent_attempts: Vec<PracticeAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PracticePlanHistory {
    pub plan: PracticePlan,
    pub attempted: u32,
    pub helped: u32,
    pub unsure: u32,
    pub not_helped: u32,
    pub not_tried: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeHistoryPage {
    pub plans: Vec<PracticePlanHistory>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeAttemptPage {
    pub attempts: Vec<PracticeAttempt>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePracticePlanRequest {
    pub goal_code: String,
    pub action_code: String,
    #[serde(default)]
    pub replace_active: bool,
    pub replaces_plan_id: Option<String>,
    pub expected_epoch: i64,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePracticeAttemptRequest {
    pub tried: bool,
    pub helpfulness: Option<String>,
    pub obstacle: Option<String>,
    pub request_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalog_accepts_only_matching_goal_and_action() {
        assert_eq!(PRACTICE_CATALOG.len(), 3);
        assert!(valid_goal_action("express_need", "say_one_sentence"));
        assert!(valid_goal_action("start_task", "ten_minute_start"));
        assert!(valid_goal_action("review_choice", "two_options"));
        assert!(!valid_goal_action("express_need", "ten_minute_start"));
        assert!(!valid_goal_action("unknown", "say_one_sentence"));
    }

    #[test]
    fn history_catalog_lookup_is_version_pinned() {
        assert_eq!(catalog_for_version("1"), Some(&PRACTICE_CATALOG));
        assert_eq!(catalog_for_version("future"), None);
        assert_eq!(catalog_for_version(""), None);
    }

    #[test]
    fn smaller_action_stays_within_goal_and_stops_at_smallest() {
        assert_eq!(
            smaller_action("express_need", "say_one_sentence"),
            Some("write_one_sentence")
        );
        assert_eq!(
            smaller_action("start_task", "ten_minute_start"),
            Some("prepare_first_step")
        );
        assert_eq!(
            smaller_action("review_choice", "two_options"),
            Some("name_one_tradeoff")
        );
        assert_eq!(smaller_action("express_need", "write_one_sentence"), None);
        assert_eq!(smaller_action("express_need", "two_options"), None);
    }

    #[test]
    fn catalog_has_one_small_and_one_larger_action_per_goal() {
        let mut goal_codes = std::collections::HashSet::new();
        let mut action_codes = std::collections::HashSet::new();
        for goal in PRACTICE_CATALOG {
            assert!(goal_codes.insert(goal.code));
            assert!(goal.actions[0].is_small);
            assert!(!goal.actions[1].is_small);
            for action in goal.actions {
                assert!(action_codes.insert(action.code));
                assert!(!action.instruction.is_empty());
            }
        }
    }

    #[test]
    fn feedback_requires_exactly_the_matching_choice() {
        assert!(valid_attempt_feedback(true, Some("helped"), None));
        assert!(valid_attempt_feedback(false, None, Some("too_big")));
        assert!(!valid_attempt_feedback(true, None, None));
        assert!(!valid_attempt_feedback(false, None, None));
        assert!(!valid_attempt_feedback(
            true,
            Some("helped"),
            Some("forgot")
        ));
        assert!(!valid_attempt_feedback(
            false,
            Some("unsure"),
            Some("forgot")
        ));
        assert!(!valid_attempt_feedback(true, Some("diagnosis"), None));
    }

    #[test]
    fn request_and_response_use_camel_case_wire_fields() {
        let request: CreatePracticePlanRequest = serde_json::from_value(json!({
            "goalCode": "express_need", "actionCode": "say_one_sentence",
            "replaceActive": true, "replacesPlanId": "old", "expectedEpoch": 2,
            "requestId": "0b484bfc-7239-4547-85be-3412490b7001"
        }))
        .unwrap();
        assert_eq!(request.goal_code, "express_need");
        assert!(request.replace_active);
        let overview = PracticeOverview {
            cycle_id: "2026-10-19".into(),
            practice_epoch: 2,
            total_plans: 1,
            total_attempts: 3,
            current_week: PracticeSummary {
                cycle_id: "2026-10-19".into(),
                attempted: 1,
                helped: 1,
                unsure: 0,
                not_helped: 0,
                not_tried: 2,
            },
            previous_week: PracticeSummary {
                cycle_id: "2026-10-12".into(),
                attempted: 0,
                helped: 0,
                unsure: 0,
                not_helped: 0,
                not_tried: 0,
            },
            active_plan: None,
            recent_attempts: vec![],
        };
        let value = serde_json::to_value(overview).unwrap();
        assert_eq!(value["practiceEpoch"], 2);
        assert_eq!(value["currentWeek"]["notTried"], 2);
        assert!(value.get("practice_epoch").is_none());
    }
}
