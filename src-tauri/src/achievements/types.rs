use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AchievementTier {
    Common,
    Rare,
    Legendary,
}

impl AchievementTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Rare => "rare",
            Self::Legendary => "legendary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AchievementCategory {
    Milestones,
    Streaks,
    Speed,
    Explorer,
    Learner,
    Secrets,
}

impl AchievementCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Milestones => "milestones",
            Self::Streaks => "streaks",
            Self::Speed => "speed",
            Self::Explorer => "explorer",
            Self::Learner => "learner",
            Self::Secrets => "secrets",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementProgress {
    pub current: i64,
    pub target: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementState {
    pub id: String,
    pub tier: String,
    pub category: String,
    pub secret: bool,
    pub unlocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlocked_at: Option<String>,
    pub seen: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<AchievementProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementsSummary {
    pub achievements: Vec<AchievementState>,
    pub unlocked_count: i64,
    pub total_count: i64,
    pub unseen_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementUnlockedPayload {
    pub id: String,
    pub tier: String,
    pub secret: bool,
}
