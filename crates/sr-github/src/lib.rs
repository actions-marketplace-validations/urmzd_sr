use sr_core::error::ReleaseError;
use sr_core::release::VcsProvider;

/// GitHub implementation of the VcsProvider trait using the GitHub REST API.
pub struct GitHubProvider {
    owner: String,
    repo: String,
    hostname: String,
    token: String,
}

#[derive(serde::Deserialize)]
struct ReleaseResponse {
    id: u64,
    html_url: String,
    upload_url: String,
}

impl GitHubProvider {
    pub fn new(owner: String, repo: String, hostname: String, token: String) -> Self {
        Self {
            owner,
            repo,
            hostname,
            token,
        }
    }

    fn base_url(&self) -> String {
        format!("https://{}/{}/{}", self.hostname, self.owner, self.repo)
    }

    fn api_url(&self) -> String {
        if self.hostname == "github.com" {
            "https://api.github.com".to_string()
        } else {
            format!("https://{}/api/v3", self.hostname)
        }
    }

    fn agent(&self) -> ureq::Agent {
        ureq::Agent::new_with_config(ureq::config::Config::builder().https_only(true).build())
    }

    fn get_release_by_tag(&self, tag: &str) -> Result<ReleaseResponse, ReleaseError> {
        let url = format!(
            "{}/repos/{}/{}/releases/tags/{tag}",
            self.api_url(),
            self.owner,
            self.repo
        );
        let resp = self
            .agent()
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .call()
            .map_err(|e| ReleaseError::Vcs(format!("GitHub API GET {url}: {e}")))?;
        let release: ReleaseResponse = resp
            .into_body()
            .read_json()
            .map_err(|e| ReleaseError::Vcs(format!("failed to parse release response: {e}")))?;
        Ok(release)
    }
}

impl VcsProvider for GitHubProvider {
    fn create_release(
        &self,
        tag: &str,
        name: &str,
        body: &str,
        prerelease: bool,
        draft: bool,
    ) -> Result<String, ReleaseError> {
        let url = format!(
            "{}/repos/{}/{}/releases",
            self.api_url(),
            self.owner,
            self.repo
        );
        let payload = serde_json::json!({
            "tag_name": tag,
            "name": name,
            "body": body,
            "prerelease": prerelease,
            "draft": draft,
        });

        let resp = self
            .agent()
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .send_json(&payload)
            .map_err(|e| ReleaseError::Vcs(format!("GitHub API POST {url}: {e}")))?;

        let release: ReleaseResponse = resp
            .into_body()
            .read_json()
            .map_err(|e| ReleaseError::Vcs(format!("failed to parse release response: {e}")))?;

        Ok(release.html_url)
    }

    fn compare_url(&self, base: &str, head: &str) -> Result<String, ReleaseError> {
        Ok(format!("{}/compare/{base}...{head}", self.base_url()))
    }

    fn release_exists(&self, tag: &str) -> Result<bool, ReleaseError> {
        let url = format!(
            "{}/repos/{}/{}/releases/tags/{tag}",
            self.api_url(),
            self.owner,
            self.repo
        );
        match self
            .agent()
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .call()
        {
            Ok(_) => Ok(true),
            Err(ureq::Error::StatusCode(404)) => Ok(false),
            Err(e) => Err(ReleaseError::Vcs(format!("GitHub API GET {url}: {e}"))),
        }
    }

    fn repo_url(&self) -> Option<String> {
        Some(self.base_url())
    }

    fn delete_release(&self, tag: &str) -> Result<(), ReleaseError> {
        let release = self.get_release_by_tag(tag)?;
        let url = format!(
            "{}/repos/{}/{}/releases/{}",
            self.api_url(),
            self.owner,
            self.repo,
            release.id
        );
        self.agent()
            .delete(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .call()
            .map_err(|e| ReleaseError::Vcs(format!("GitHub API DELETE {url}: {e}")))?;
        Ok(())
    }

    fn update_release(
        &self,
        tag: &str,
        name: &str,
        body: &str,
        prerelease: bool,
        draft: bool,
    ) -> Result<String, ReleaseError> {
        let release = self.get_release_by_tag(tag)?;
        let url = format!(
            "{}/repos/{}/{}/releases/{}",
            self.api_url(),
            self.owner,
            self.repo,
            release.id
        );
        let payload = serde_json::json!({
            "name": name,
            "body": body,
            "prerelease": prerelease,
            "draft": draft,
        });
        let resp = self
            .agent()
            .patch(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "sr-github")
            .send_json(&payload)
            .map_err(|e| ReleaseError::Vcs(format!("GitHub API PATCH {url}: {e}")))?;
        let updated: ReleaseResponse = resp
            .into_body()
            .read_json()
            .map_err(|e| ReleaseError::Vcs(format!("failed to parse release response: {e}")))?;
        Ok(updated.html_url)
    }

    fn upload_assets(&self, tag: &str, files: &[&str]) -> Result<(), ReleaseError> {
        let release = self.get_release_by_tag(tag)?;
        // The upload_url from the API looks like:
        //   https://uploads.github.com/repos/owner/repo/releases/123/assets{?name,label}
        // Strip the {?name,label} template suffix.
        let upload_base = release
            .upload_url
            .split('{')
            .next()
            .unwrap_or(&release.upload_url);

        for file_path in files {
            let path = std::path::Path::new(file_path);
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| ReleaseError::Vcs(format!("invalid file path: {file_path}")))?;

            let data = std::fs::read(path)
                .map_err(|e| ReleaseError::Vcs(format!("failed to read asset {file_path}: {e}")))?;

            let content_type = mime_from_extension(file_name);
            let url = format!("{upload_base}?name={file_name}");

            // Retry up to 3 times for transient upload failures
            let mut last_err = None;
            for attempt in 0..3 {
                if attempt > 0 {
                    std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
                    eprintln!(
                        "Retrying upload of {file_name} (attempt {}/3)...",
                        attempt + 1
                    );
                }
                match self
                    .agent()
                    .post(&url)
                    .header("Authorization", &format!("Bearer {}", self.token))
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .header("User-Agent", "sr-github")
                    .header("Content-Type", content_type)
                    .send(&data[..])
                {
                    Ok(_) => {
                        last_err = None;
                        break;
                    }
                    Err(e) => {
                        last_err = Some(format!("GitHub API upload asset {file_name}: {e}"));
                    }
                }
            }
            if let Some(err_msg) = last_err {
                return Err(ReleaseError::Vcs(err_msg));
            }
        }

        Ok(())
    }

    fn verify_release(&self, tag: &str) -> Result<(), ReleaseError> {
        // GET the release by tag to confirm it exists and is accessible
        self.get_release_by_tag(tag)?;
        Ok(())
    }
}

/// Map file extension to MIME type for GitHub asset uploads.
fn mime_from_extension(filename: &str) -> &'static str {
    match filename.rsplit('.').next().unwrap_or("") {
        "gz" | "tgz" => "application/gzip",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "xz" => "application/x-xz",
        "bz2" => "application/x-bzip2",
        "zst" | "zstd" => "application/zstd",
        "deb" => "application/vnd.debian.binary-package",
        "rpm" => "application/x-rpm",
        "dmg" => "application/x-apple-diskimage",
        "msi" => "application/x-msi",
        "exe" => "application/vnd.microsoft.portable-executable",
        "sig" | "asc" => "application/pgp-signature",
        "sha256" | "sha512" => "text/plain",
        "json" => "application/json",
        "txt" | "md" => "text/plain",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github_com_provider() -> GitHubProvider {
        GitHubProvider::new(
            "urmzd".into(),
            "sr".into(),
            "github.com".into(),
            "test-token".into(),
        )
    }

    fn ghes_provider() -> GitHubProvider {
        GitHubProvider::new(
            "org".into(),
            "repo".into(),
            "ghes.example.com".into(),
            "test-token".into(),
        )
    }

    #[test]
    fn test_api_url_github_com() {
        assert_eq!(github_com_provider().api_url(), "https://api.github.com");
    }

    #[test]
    fn test_api_url_ghes() {
        assert_eq!(ghes_provider().api_url(), "https://ghes.example.com/api/v3");
    }

    #[test]
    fn test_base_url() {
        assert_eq!(
            github_com_provider().base_url(),
            "https://github.com/urmzd/sr"
        );
        assert_eq!(
            ghes_provider().base_url(),
            "https://ghes.example.com/org/repo"
        );
    }

    #[test]
    fn test_compare_url() {
        let p = github_com_provider();
        assert_eq!(
            p.compare_url("v0.9.0", "v1.0.0").unwrap(),
            "https://github.com/urmzd/sr/compare/v0.9.0...v1.0.0"
        );
    }

    #[test]
    fn test_repo_url() {
        assert_eq!(
            github_com_provider().repo_url().unwrap(),
            "https://github.com/urmzd/sr"
        );
    }
}
