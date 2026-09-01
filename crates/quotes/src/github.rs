use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

const QUERY_TIMEOUT: Duration = Duration::from_secs(15);
const RECENT_ACTIVITY_LIMIT: usize = 3;
const GITHUB_QUERY: &str = r#"
query DashboardGithub {
  viewer {
    login
    url
    contributionsCollection {
      contributionCalendar {
        totalContributions
        weeks {
          contributionDays {
            date
            contributionCount
            contributionLevel
          }
        }
      }
      commitContributionsByRepository(maxRepositories: 100) {
        repository { nameWithOwner }
        contributions(first: 3, orderBy: { field: OCCURRED_AT, direction: DESC }) {
          nodes { commitCount occurredAt url }
        }
      }
      pullRequestContributions(first: 3, orderBy: { direction: DESC }) {
        nodes {
          occurredAt
          url
          pullRequest { title repository { nameWithOwner } }
        }
      }
      pullRequestReviewContributions(first: 3, orderBy: { direction: DESC }) {
        nodes {
          occurredAt
          url
          pullRequestReview { state }
          pullRequest { title repository { nameWithOwner } }
        }
      }
    }
  }
}
"#;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubSnapshot {
    login: String,
    profile_url: String,
    total_contributions: u32,
    weeks: Vec<ContributionWeek>,
    recent_activity: Vec<GithubActivity>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositorySnapshot {
    pub full_name: String,
    pub description: String,
    pub owner_avatar_url: String,
    pub language: String,
    pub stars: u64,
    pub forks: u64,
    pub open_issues: u64,
    pub default_branch: String,
    pub updated_at: String,
    pub url: String,
}

#[derive(Deserialize)]
struct RepositoryWire {
    full_name: String,
    description: Option<String>,
    owner: RepositoryOwnerWire,
    language: Option<String>,
    stargazers_count: u64,
    forks_count: u64,
    open_issues_count: u64,
    default_branch: String,
    updated_at: String,
    html_url: String,
}

#[derive(Deserialize)]
struct RepositoryOwnerWire {
    avatar_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContributionWeek {
    days: Vec<ContributionDay>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContributionDay {
    date: String,
    count: u32,
    level: u8,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum GithubActivityKind {
    Commit,
    PullRequest,
    Review,
    Approve,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GithubActivity {
    kind: GithubActivityKind,
    title: String,
    repository: String,
    occurred_at: String,
    url: String,
}

#[derive(Deserialize)]
struct GraphqlResponse {
    data: Option<GraphqlData>,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct GraphqlData {
    viewer: Viewer,
}

#[derive(Deserialize)]
struct Viewer {
    login: String,
    url: String,
    #[serde(rename = "contributionsCollection")]
    contributions: Contributions,
}

#[derive(Deserialize)]
struct Contributions {
    #[serde(rename = "contributionCalendar")]
    calendar: ContributionCalendar,
    #[serde(rename = "commitContributionsByRepository")]
    commits_by_repo: Vec<RepoCommits>,
    #[serde(rename = "pullRequestContributions")]
    pull_requests: Connection<PullContribution>,
    #[serde(rename = "pullRequestReviewContributions")]
    reviews: Connection<ReviewContribution>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionCalendar {
    total_contributions: u32,
    weeks: Vec<ContributionWeekWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionWeekWire {
    contribution_days: Vec<ContributionDayWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionDayWire {
    date: String,
    contribution_count: u32,
    contribution_level: ContributionLevel,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ContributionLevel {
    None,
    FirstQuartile,
    SecondQuartile,
    ThirdQuartile,
    FourthQuartile,
}

impl ContributionLevel {
    fn value(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::FirstQuartile => 1,
            Self::SecondQuartile => 2,
            Self::ThirdQuartile => 3,
            Self::FourthQuartile => 4,
        }
    }
}

#[derive(Deserialize)]
struct Connection<T> {
    nodes: Vec<T>,
}

#[derive(Deserialize)]
struct Repository {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
}

#[derive(Deserialize)]
struct RepoCommits {
    repository: Repository,
    contributions: Connection<CommitContribution>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitContribution {
    commit_count: u32,
    occurred_at: String,
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullContribution {
    occurred_at: String,
    url: String,
    pull_request: PullRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewContribution {
    occurred_at: String,
    url: String,
    pull_request_review: PullRequestReview,
    pull_request: PullRequest,
}

#[derive(Deserialize)]
struct PullRequest {
    title: String,
    repository: Repository,
}

#[derive(Deserialize)]
struct PullRequestReview {
    state: String,
}

pub async fn read() -> Result<GithubSnapshot, String> {
    let binary = resolve_gh_binary()?;
    let query = format!("query={GITHUB_QUERY}");
    let mut command = Command::new(binary);
    command
        .args(["api", "graphql", "-f", &query])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::time::timeout(QUERY_TIMEOUT, command.output())
        .await
        .map_err(|_| "GitHub CLI timed out while loading Dashboard data".to_owned())?
        .map_err(|error| format!("Could not start GitHub CLI: {error}"))?;

    if !output.status.success() {
        return Err(
            "GitHub CLI could not load account data. Run `gh auth status` to check its login."
                .to_owned(),
        );
    }

    parse_snapshot(&output.stdout)
}

pub async fn read_repository(repository: &str) -> Result<RepositorySnapshot, String> {
    let binary = resolve_gh_binary()?;
    let endpoint = format!("repos/{repository}");
    let mut command = Command::new(binary);
    command
        .args(["api", &endpoint])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::time::timeout(QUERY_TIMEOUT, command.output())
        .await
        .map_err(|_| format!("GitHub CLI timed out while loading {repository}"))?
        .map_err(|error| format!("Could not start GitHub CLI: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GitHub CLI could not load repository {repository}. Check the repository and `gh auth status`."
        ));
    }
    let wire: RepositoryWire = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("GitHub CLI returned unsupported repository JSON: {error}"))?;
    Ok(RepositorySnapshot {
        full_name: wire.full_name,
        description: wire.description.unwrap_or_default(),
        owner_avatar_url: wire.owner.avatar_url,
        language: wire.language.unwrap_or_default(),
        stars: wire.stargazers_count,
        forks: wire.forks_count,
        open_issues: wire.open_issues_count,
        default_branch: wire.default_branch,
        updated_at: wire.updated_at,
        url: wire.html_url,
    })
}

fn parse_snapshot(bytes: &[u8]) -> Result<GithubSnapshot, String> {
    let response: GraphqlResponse = serde_json::from_slice(bytes)
        .map_err(|error| format!("GitHub CLI returned unsupported JSON: {error}"))?;
    if !response.errors.is_empty() {
        return Err("GitHub GraphQL returned an error while loading Dashboard data".to_owned());
    }
    let viewer = response
        .data
        .ok_or_else(|| "GitHub GraphQL did not return account data".to_owned())?
        .viewer;
    let Contributions {
        calendar,
        commits_by_repo,
        pull_requests,
        reviews,
    } = viewer.contributions;

    let weeks = calendar
        .weeks
        .into_iter()
        .map(|week| ContributionWeek {
            days: week
                .contribution_days
                .into_iter()
                .map(|day| ContributionDay {
                    date: day.date,
                    count: day.contribution_count,
                    level: day.contribution_level.value(),
                })
                .collect(),
        })
        .collect();

    let mut recent_activity = Vec::new();
    for group in commits_by_repo {
        for contribution in group.contributions.nodes {
            let noun = if contribution.commit_count == 1 {
                "commit"
            } else {
                "commits"
            };
            recent_activity.push(GithubActivity {
                kind: GithubActivityKind::Commit,
                title: format!("{} {noun}", contribution.commit_count),
                repository: group.repository.name_with_owner.clone(),
                occurred_at: contribution.occurred_at,
                url: contribution.url,
            });
        }
    }
    for contribution in pull_requests.nodes {
        recent_activity.push(GithubActivity {
            kind: GithubActivityKind::PullRequest,
            title: contribution.pull_request.title,
            repository: contribution.pull_request.repository.name_with_owner,
            occurred_at: contribution.occurred_at,
            url: contribution.url,
        });
    }
    for contribution in reviews.nodes {
        let kind = if contribution.pull_request_review.state == "APPROVED" {
            GithubActivityKind::Approve
        } else {
            GithubActivityKind::Review
        };
        recent_activity.push(GithubActivity {
            kind,
            title: contribution.pull_request.title,
            repository: contribution.pull_request.repository.name_with_owner,
            occurred_at: contribution.occurred_at,
            url: contribution.url,
        });
    }
    recent_activity.sort_by(|left, right| right.occurred_at.cmp(&left.occurred_at));
    recent_activity.truncate(RECENT_ACTIVITY_LIMIT);

    Ok(GithubSnapshot {
        login: viewer.login,
        profile_url: viewer.url,
        total_contributions: calendar.total_contributions,
        weeks,
        recent_activity,
    })
}

fn resolve_gh_binary() -> Result<PathBuf, String> {
    match std::env::var_os("GITHUB_CLI_BINARY").map(PathBuf::from) {
        Some(path) if path.is_file() => return Ok(path),
        Some(_) | None => {}
    }

    if let Some(path) = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|directory| directory.join(if cfg!(windows) { "gh.exe" } else { "gh" }))
            .find(|candidate| candidate.is_file())
    }) {
        return Ok(path);
    }

    #[cfg(unix)]
    {
        let shell = std::env::var("SHELL").unwrap_or("/bin/zsh".to_owned());
        if let Ok(output) = std::process::Command::new(shell)
            .args(["-lc", "command -v gh"])
            .output()
        {
            let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
            if output.status.success() && path.is_file() {
                return Ok(path);
            }
        }
    }

    Err("GitHub CLI was not found. Install `gh` and run `gh auth login` first.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_activity() {
        let snapshot = parse_snapshot(
            br#"{
              "data": { "viewer": {
                "login": "octocat",
                "url": "https://github.com/octocat",
                "contributionsCollection": {
                  "contributionCalendar": {
                    "totalContributions": 42,
                    "weeks": [{ "contributionDays": [
                      { "date": "2026-08-23", "contributionCount": 4, "contributionLevel": "SECOND_QUARTILE" }
                    ] }]
                  },
                  "commitContributionsByRepository": [{
                    "repository": { "nameWithOwner": "octocat/hello" },
                    "contributions": { "nodes": [{
                      "commitCount": 2, "occurredAt": "2026-08-24T08:00:00Z", "url": "https://github.com/octocat/hello/commits"
                    }] }
                  }],
                  "pullRequestContributions": { "nodes": [{
                    "occurredAt": "2026-08-24T09:00:00Z", "url": "https://github.com/octocat/hello/pull/1",
                    "pullRequest": { "title": "Polish the dashboard", "repository": { "nameWithOwner": "octocat/hello" } }
                  }] },
                  "pullRequestReviewContributions": { "nodes": [
                    {
                      "occurredAt": "2026-08-24T10:00:00Z", "url": "https://github.com/octocat/hello/pull/2#review",
                      "pullRequestReview": { "state": "APPROVED" },
                      "pullRequest": { "title": "Ship the feature", "repository": { "nameWithOwner": "octocat/hello" } }
                    },
                    {
                      "occurredAt": "2026-08-24T07:00:00Z", "url": "https://github.com/octocat/hello/pull/3#review",
                      "pullRequestReview": { "state": "CHANGES_REQUESTED" },
                      "pullRequest": { "title": "Adjust the query", "repository": { "nameWithOwner": "octocat/hello" } }
                    }
                  ] }
                }
              } }
            }"#,
        )
        .expect("valid GitHub response");

        assert_eq!(snapshot.total_contributions, 42);
        assert_eq!(snapshot.weeks[0].days[0].level, 2);
        assert_eq!(snapshot.recent_activity.len(), 3);
        assert_eq!(
            snapshot.recent_activity[0].kind,
            GithubActivityKind::Approve
        );
        assert_eq!(
            snapshot.recent_activity[1].kind,
            GithubActivityKind::PullRequest
        );
        assert_eq!(snapshot.recent_activity[2].kind, GithubActivityKind::Commit);
    }

    #[test]
    fn rejects_graphql_errors() {
        let error = parse_snapshot(br#"{"errors":[{"message":"bad credentials"}]}"#)
            .expect_err("GraphQL errors must fail the provider");
        assert_eq!(
            error,
            "GitHub GraphQL returned an error while loading Dashboard data"
        );
    }
}
