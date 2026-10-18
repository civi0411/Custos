//! Repository-scoped Git and GitHub I/O. Credentials never leave this adapter.
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tokio::{io::AsyncReadExt, process::Command};

const ACCOUNT_KEY: &str = "custos.github-account";
const OUTPUT_LIMIT: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubBinding { pub host: String, pub user: String }
impl GitHubBinding {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.host.is_empty() || self.host.starts_with('-') || !self.host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            || self.user.is_empty() || self.user.starts_with('-') || !self.user.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)) {
            return Err(error("Invalid GitHub host/user binding"));
        }
        Ok(())
    }
}
fn error(message: impl Into<String>) -> DomainError { DomainError::Validation(message.into()) }

async fn execute(command: &mut Command) -> Result<(bool, Vec<u8>, Vec<u8>), DomainError> {
    command.kill_on_drop(true).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut child = command.spawn().map_err(|_| error("Could not launch Git/GitHub CLI"))?;
    let stdout = child.stdout.take().ok_or_else(|| error("Missing command output"))?;
    let stderr = child.stderr.take().ok_or_else(|| error("Missing command diagnostics"))?;
    let operation = async {
        let mut out = Vec::new(); let mut err = Vec::new();
        let mut stdout = stdout.take(OUTPUT_LIMIT + 1);
        let mut stderr = stderr.take(OUTPUT_LIMIT + 1);
        tokio::try_join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut err))
            .map_err(|_| error("Cannot read Git/GitHub CLI output"))?;
        if out.len() as u64 > OUTPUT_LIMIT || err.len() as u64 > OUTPUT_LIMIT { return Err(error("Git output exceeded the supported limit")); }
        let status = child.wait().await.map_err(|_| error("Cannot observe Git/GitHub CLI exit"))?;
        Ok((status.success(), out, err))
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), operation).await
        .map_err(|_| error("Git/GitHub command timed out; refresh state before retrying"))?
}
fn git_command(path: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command.current_dir(path).args(args).env("GIT_TERMINAL_PROMPT", "0").env("GIT_OPTIONAL_LOCKS", "0");
    command
}
async fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>, DomainError> {
    let (success, stdout, _) = execute(&mut git_command(path, args)).await?;
    if !success { return Err(error(format!("Git {} failed; inspect repository state and configuration", args.first().copied().unwrap_or("command")))); }
    Ok(stdout)
}
async fn text(path: &Path, args: &[&str]) -> Result<String, DomainError> {
    String::from_utf8(git(path,args).await?).map(|s| s.trim_end_matches('\n').to_string()).map_err(|_| error("Git returned a non-UTF8 path/ref; unsupported by this view"))
}
async fn optional(path: &Path, args: &[&str]) -> Result<Option<String>, DomainError> {
    let (success, stdout, _) = execute(&mut git_command(path,args)).await?;
    if success { return Ok(Some(String::from_utf8_lossy(&stdout).trim().to_string())); }
    // Only use for expected absence probes after repository validation.
    Ok(None)
}
async fn validate_root(path: &Path) -> Result<(), DomainError> {
    let observed = text(path, &["rev-parse", "--show-toplevel"]).await?;
    if Path::new(&observed).canonicalize().map_err(|_| error("Cannot resolve checkout root"))? != path.canonicalize().map_err(|_| error("Cannot resolve workspace path"))? {
        return Err(error("Registered path must be the exact Git checkout root"));
    }
    Ok(())
}
pub async fn account_binding(path: &Path) -> Result<Option<GitHubBinding>, DomainError> {
    validate_root(path).await?;
    let value = optional(path, &["config", "--local", "--get", ACCOUNT_KEY]).await?;
    value.map(|raw| {
        let binding: GitHubBinding = serde_json::from_str(&raw).map_err(|_| error("Repository GitHub account setting is invalid; clear it explicitly"))?;
        binding.validate()?; Ok(binding)
    }).transpose()
}
fn clean_gh(command: &mut Command) {
    for key in ["GH_TOKEN","GITHUB_TOKEN","GH_ENTERPRISE_TOKEN","GITHUB_ENTERPRISE_TOKEN","GH_HOST","GH_REPO","GH_DEBUG"] { command.env_remove(key); }
    command.env("GH_PROMPT_DISABLED","1");
}
async fn account_token(gh: &Path, binding: &GitHubBinding) -> Result<String, DomainError> {
    binding.validate()?;
    let mut command = Command::new(gh); clean_gh(&mut command);
    command.args(["auth","token","--hostname",&binding.host,"--user",&binding.user]);
    let (success, stdout, _) = execute(&mut command).await?;
    if !success { return Err(error("Selected GitHub account is unavailable; reconnect it on this machine")); }
    let token = String::from_utf8(stdout).map_err(|_| error("Invalid credential response"))?.trim().to_string();
    if token.is_empty() || token.contains(['\n','\r']) { return Err(error("Selected GitHub credential is unavailable")); }
    Ok(token)
}
pub async fn github_viewer(gh: &Path, binding: &GitHubBinding) -> Result<Value, DomainError> {
    let token = account_token(gh,binding).await?;
    let mut command = Command::new(gh); clean_gh(&mut command);
    let key = if binding.host == "github.com" || binding.host.ends_with(".ghe.com") { "GH_TOKEN" } else { "GH_ENTERPRISE_TOKEN" };
    command.env(key,token).args(["api","--hostname",&binding.host,"user"]);
    let (success, stdout, _) = execute(&mut command).await?;
    if !success { return Err(error("Selected GitHub account could not be verified; no ambient-account fallback was used")); }
    let user: Value = serde_json::from_slice(&stdout).map_err(|_| error("Invalid GitHub viewer response"))?;
    if !user["login"].as_str().is_some_and(|login| login.eq_ignore_ascii_case(&binding.user)) { return Err(error("GitHub returned a different identity than the selected account")); }
    Ok(json!({"host":binding.host,"login":user["login"],"name":user["name"],"avatar_url":user["avatar_url"]}))
}
pub async fn github_accounts(gh: &Path) -> Result<Value, DomainError> {
    let mut command = Command::new(gh); clean_gh(&mut command);
    command.args(["auth","status","--json","hosts"]);
    let (_, stdout, _) = execute(&mut command).await?;
    let raw: Value = serde_json::from_slice(&stdout).map_err(|_| error("GitHub CLI account inventory unavailable; update gh to a version supporting JSON account status"))?;
    let hosts = raw["hosts"].as_object().ok_or_else(|| error("GitHub CLI returned no account inventory"))?;
    let mut accounts = Vec::new();
    for (host, entries) in hosts {
        for entry in entries.as_array().into_iter().flatten() {
            if let Some(user) = entry["login"].as_str() {
                accounts.push(json!({"host":host,"user":user,"active":entry["active"].as_bool().unwrap_or(false),"state":entry["state"],"source":entry["tokenSource"]}));
            }
        }
    }
    Ok(json!({"accounts":accounts}))
}
pub async fn bind_account(path: &Path, gh: Option<&Path>, binding: Option<&GitHubBinding>) -> Result<(), DomainError> {
    validate_root(path).await?;
    if let Some(binding) = binding {
        github_viewer(gh.ok_or_else(|| error("GitHub CLI is required to verify this account"))?, binding).await?;
        let value = serde_json::to_string(binding).map_err(|_| error("Cannot serialize account preference"))?;
        git(path, &["config","--local",ACCOUNT_KEY,&value]).await?;
    } else if optional(path, &["config","--local","--get",ACCOUNT_KEY]).await?.is_some() {
        git(path, &["config","--local","--unset-all",ACCOUNT_KEY]).await?;
    }
    Ok(())
}
fn safe_remote_url(raw: &str) -> String {
    if let Ok(mut url) = url::Url::parse(raw) {
        let _ = url.set_username(""); let _ = url.set_password(None); url.set_query(None); url.set_fragment(None);
        return url.to_string();
    }
    // SCP-style SSH URLs are not URLs; suppress arbitrary unsupported transports.
    if raw.starts_with("git@") && !raw.contains(['\n','\r']) { return raw.to_string(); }
    "[local or non-URL transport]".to_string()
}
pub async fn repository(path: &Path) -> Result<Value, DomainError> {
    validate_root(path).await?;
    let common_dir = text(path,&["rev-parse","--path-format=absolute","--git-common-dir"]).await?;
    let common_dir = Path::new(&common_dir).canonicalize().map_err(|_| error("Cannot resolve Git common directory"))?;
    let branch = optional(path,&["symbolic-ref","--quiet","--short","HEAD"]).await?;
    let head = optional(path,&["rev-parse","--verify","HEAD"]).await?;
    let dirty = !git(path,&["status","--porcelain=v1","-z","--untracked-files=all"]).await?.is_empty();
    let refs = text(path,&["for-each-ref","--format=%(refname)%00%(objectname)%00%(upstream)%00%(upstream:track)%00%(worktreepath)","refs/heads/","refs/remotes/"]).await?;
    let branches: Vec<Value> = refs.lines().filter_map(|line| {
        let cols: Vec<&str> = line.split('\0').collect(); if cols.len()!=5 || cols[0].ends_with("/HEAD") { return None; }
        Some(json!({"refname":cols[0],"oid":cols[1],"upstream":cols[2],"tracking":cols[3],"checkout_path":cols[4],"remote":cols[0].starts_with("refs/remotes/")}))
    }).collect();
    let data = git(path,&["worktree","list","--porcelain","-z"]).await?;
    let mut worktrees = Vec::new(); let mut row = serde_json::Map::new();
    for field in data.split(|b| *b==0) {
        if field.is_empty() { if !row.is_empty() { worktrees.push(Value::Object(std::mem::take(&mut row))); } continue; }
        let field = std::str::from_utf8(field).map_err(|_| error("Non-UTF8 checkout path unsupported"))?;
        let (key,value) = field.split_once(' ').unwrap_or((field,"true")); row.insert(key.to_string(),json!(value));
    }
    if !row.is_empty() { worktrees.push(Value::Object(row)); }
    let mut remotes = Vec::new();
    for name in text(path,&["remote"]).await?.lines() {
        let fetch = text(path,&["remote","get-url","--",name]).await?;
        let push = text(path,&["remote","get-url","--push","--",name]).await?;
        remotes.push(json!({"name":name,"fetch_url":safe_remote_url(&fetch),"push_url":safe_remote_url(&push)}));
    }
    let mut history = Vec::new();
    if head.is_some() {
        let log = text(path,&["log","-30","--format=%H%x00%P%x00%an%x00%aI%x00%s"]).await?;
        for line in log.lines() { let c:Vec<&str>=line.split('\0').collect(); if c.len()==5 { history.push(json!({"oid":c[0],"parents":c[1],"author":c[2],"time":c[3],"subject":c[4]})); } }
    }
    Ok(json!({"checkout_path":path,"common_dir":common_dir,"branch":branch,"head":head,"dirty":dirty,"branches":branches,"worktrees":worktrees,"remotes":remotes,"history":history,"github_account":account_binding(path).await?,"observed_at":chrono::Utc::now()}))
}
pub async fn switch_branch(path: &Path, branch: &str, expected_head: &str) -> Result<Value, DomainError> {
    validate_root(path).await?;
    if branch.is_empty() || branch.starts_with('-') || branch.contains("@{") { return Err(error("Invalid literal local branch")); }
    git(path,&["check-ref-format","--branch",branch]).await?;
    git(path,&["show-ref","--verify",&format!("refs/heads/{branch}")]).await?;
    let observed = repository(path).await?;
    if observed["head"].as_str()!=Some(expected_head) { return Err(DomainError::Conflict("Checkout HEAD changed; refresh before switching".into())); }
    if observed["dirty"]==true { return Err(DomainError::Conflict("Checkout has local changes; create another worktree instead".into())); }
    for marker in ["MERGE_HEAD","rebase-merge","rebase-apply","CHERRY_PICK_HEAD","REVERT_HEAD","sequencer"] {
        let marker = text(path,&["rev-parse","--git-path",marker]).await?;
        if path.join(marker).exists() { return Err(DomainError::Conflict("Finish or abort the current Git operation before switching".into())); }
    }
    // Git refuses a branch occupied in another worktree; never force/reset/stash.
    git(path,&["switch","--no-guess",branch]).await?;
    let actual = repository(path).await?;
    if actual["branch"].as_str()!=Some(branch) { return Err(DomainError::Conflict("Switch outcome requires reconciliation".into())); }
    Ok(actual)
}
pub async fn fetch_remote(path: &Path, gh: Option<&Path>, remote: &str) -> Result<Value, DomainError> {
    validate_root(path).await?;
    let names=text(path,&["remote"]).await?;
    if remote.starts_with('-') || !names.lines().any(|name| name==remote) { return Err(error("Select an existing remote")); }
    let binding=account_binding(path).await?;
    let mut command=git_command(path,&[]);
    if let Some(binding)=binding {
        let remote_url=text(path,&["remote","get-url","--",remote]).await?;
        let url=url::Url::parse(&remote_url).map_err(|_| error("Account-bound fetch currently requires an HTTPS remote; SSH uses separate credentials"))?;
        if url.scheme()!="https" || !url.host_str().is_some_and(|host| host.eq_ignore_ascii_case(&binding.host)) || !url.username().is_empty() || url.password().is_some() {
            return Err(error("Remote host/protocol does not match the selected account; no credentials were sent"));
        }
        let token=account_token(gh.ok_or_else(|| error("GitHub CLI is required for the bound account"))?,&binding).await?;
        command.args(["-c","credential.helper=","-c","credential.helper=!f() { printf 'username=x-access-token\\npassword=%s\\n' \"$CUSTOS_GIT_TOKEN\"; }; f", "-c","http.followRedirects=false"]);
        command.env("CUSTOS_GIT_TOKEN",token);
    }
    command.args(["fetch","--no-recurse-submodules","--",remote]);
    let (success,_,_)=execute(&mut command).await?;
    if !success { return Err(error("Fetch failed; check remote/network/account access and refresh refs. No merge or push was requested.")); }
    repository(path).await
}
