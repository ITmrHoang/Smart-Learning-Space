use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::model::{ApplicationRow, ApplicationStatus};

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SubmitApplicationRequest {
    #[validate(length(min = 1, max = 10, message = "Chọn 1–10 môn muốn dạy."))]
    pub subject_ids: Vec<Uuid>,
    #[serde(default)]
    #[validate(length(max = 20, message = "Chọn tối đa 20 lớp."))]
    pub grade_ids: Vec<Uuid>,
    #[validate(length(min = 20, max = 2000, message = "Giới thiệu bản thân 20–2000 ký tự."))]
    pub bio: String,
    #[serde(default)]
    #[validate(length(max = 2000, message = "Kinh nghiệm tối đa 2000 ký tự."))]
    pub experience: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RejectApplicationRequest {
    #[validate(length(min = 5, max = 500, message = "Lý do 5–500 ký tự."))]
    pub reason: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationListQuery {
    pub status: Option<ApplicationStatus>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicantInfo {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationResponse {
    pub id: Uuid,
    pub applicant: ApplicantInfo,
    pub subject_ids: Vec<Uuid>,
    pub grade_ids: Vec<Uuid>,
    pub bio: String,
    pub experience: String,
    pub status: ApplicationStatus,
    pub reject_reason: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ApplicationRow> for ApplicationResponse {
    fn from(r: ApplicationRow) -> Self {
        Self {
            id: r.id,
            applicant: ApplicantInfo { id: r.user_id, username: r.username, full_name: r.full_name, email: r.email },
            subject_ids: r.subject_ids,
            grade_ids: r.grade_ids,
            bio: r.bio,
            experience: r.experience,
            status: r.status,
            reject_reason: r.reject_reason,
            reviewed_at: r.reviewed_at,
            created_at: r.created_at,
        }
    }
}
