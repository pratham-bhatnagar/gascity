use std::time::Instant;

use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;

/// Low-level Gitea REST API client.
pub struct GiteaClient {
    http: reqwest::Client,
    pub base_url: String,
}

impl GiteaClient {
    pub fn new(base_url: &str, token: Option<&str>) -> Result<Self, reqwest::Error> {
        let mut headers = HeaderMap::new();
        if let Some(t) = token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("token {t}")).expect("valid header"),
            );
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(GiteaClient {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// Health check: GET /api/v1/version
    pub async fn version(&self) -> Result<(String, u64), reqwest::Error> {
        let start = Instant::now();
        let resp: GiteaVersion = self
            .http
            .get(format!("{}/api/v1/version", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let ms = start.elapsed().as_millis() as u64;
        Ok((resp.version, ms))
    }

    /// GET /api/v1/repos/search
    pub async fn list_repos(
        &self,
        page: u32,
        limit: u32,
        sort: Option<&str>,
        order: Option<&str>,
    ) -> Result<Vec<GiteaRepo>, reqwest::Error> {
        let mut url = format!("{}/api/v1/repos/search?page={page}&limit={limit}", self.base_url);
        if let Some(s) = sort {
            url.push_str(&format!("&sort={s}"));
        }
        if let Some(o) = order {
            url.push_str(&format!("&order={o}"));
        }
        self.http.get(&url).send().await?.error_for_status()?.json().await
    }

    /// GET /api/v1/repos/{owner}/{repo}/issues
    pub async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
        issue_type: Option<&str>,
        page: u32,
        limit: u32,
    ) -> Result<Vec<GiteaIssue>, reqwest::Error> {
        let mut url = format!(
            "{}/api/v1/repos/{owner}/{repo}/issues?page={page}&limit={limit}",
            self.base_url
        );
        if let Some(s) = state {
            url.push_str(&format!("&state={s}"));
        }
        if let Some(t) = issue_type {
            url.push_str(&format!("&type={t}"));
        }
        self.http.get(&url).send().await?.error_for_status()?.json().await
    }

    /// GET /api/v1/repos/{owner}/{repo}/pulls
    pub async fn list_pulls(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
        page: u32,
        limit: u32,
    ) -> Result<Vec<GiteaPull>, reqwest::Error> {
        let mut url = format!(
            "{}/api/v1/repos/{owner}/{repo}/pulls?page={page}&limit={limit}",
            self.base_url
        );
        if let Some(s) = state {
            url.push_str(&format!("&state={s}"));
        }
        self.http.get(&url).send().await?.error_for_status()?.json().await
    }

    /// GET /api/v1/repos/{owner}/{repo}/commits
    pub async fn list_commits(
        &self,
        owner: &str,
        repo: &str,
        sha: Option<&str>,
        page: u32,
        limit: u32,
    ) -> Result<Vec<GiteaCommit>, reqwest::Error> {
        let mut url = format!(
            "{}/api/v1/repos/{owner}/{repo}/git/commits?page={page}&limit={limit}",
            self.base_url
        );
        if let Some(s) = sha {
            url.push_str(&format!("&sha={s}"));
        }
        self.http.get(&url).send().await?.error_for_status()?.json().await
    }

    /// GET /api/v1/admin/users (requires admin token) or /api/v1/users/search
    pub async fn list_users(
        &self,
        page: u32,
        limit: u32,
    ) -> Result<Vec<GiteaUser>, reqwest::Error> {
        let url = format!(
            "{}/api/v1/users/search?page={page}&limit={limit}",
            self.base_url
        );
        let resp: UserSearchResult = self.http.get(&url).send().await?.error_for_status()?.json().await?;
        Ok(resp.data)
    }
}

// ----- API response types -----

#[derive(Debug, Deserialize)]
pub struct GiteaVersion {
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaRepo {
    pub id: i64,
    pub full_name: String,
    pub name: String,
    pub description: Option<String>,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    pub stars_count: i64,
    pub forks_count: i64,
    pub open_issues_count: i64,
    pub default_branch: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner: Option<GiteaUser>,
}

#[derive(Debug, Deserialize)]
pub struct GiteaIssue {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: Option<GiteaUser>,
    pub labels: Option<Vec<GiteaLabel>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct GiteaLabel {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct GiteaPull {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: Option<GiteaUser>,
    pub head: Option<GiteaPullRef>,
    pub base: Option<GiteaPullRef>,
    pub merged: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct GiteaPullRef {
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GiteaCommit {
    pub sha: String,
    pub commit: Option<GiteaCommitDetail>,
    pub author: Option<GiteaUser>,
}

#[derive(Debug, Deserialize)]
pub struct GiteaCommitDetail {
    pub message: Option<String>,
    pub author: Option<GiteaCommitAuthor>,
}

#[derive(Debug, Deserialize)]
pub struct GiteaCommitAuthor {
    pub name: Option<String>,
    pub email: Option<String>,
    pub date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GiteaUser {
    pub id: i64,
    pub login: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub created: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct UserSearchResult {
    data: Vec<GiteaUser>,
}
