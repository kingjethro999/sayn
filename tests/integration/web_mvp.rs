//! End-to-end craft tests: the real `sayn` binary against a local fake
//! OpenAI-compatible provider. No real network, package manager, or credentials.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;

const FULL_APP: &str = r#"{"schema_version":1,"operations":[
  {"kind":"create","path":"package.json","content":"{\"name\":\"app\",\"scripts\":{\"build\":\"next build\"}}"},
  {"kind":"create","path":"app/layout.tsx","content":"export default function Layout({children}){return <html><body>{children}</body></html>}"},
  {"kind":"create","path":"app/page.tsx","content":"export default function Page(){return <main>first version</main>}"},
  {"kind":"create","path":"app/globals.css","content":"@import \"tailwindcss\";"}],
  "acceptance_checks":["build"],"summary":"portfolio"}"#;

const CATALOG: &str = r#"{"data":[{"id":"entitled-model"}]}"#;

#[derive(Default)]
struct Recorded {
    chat_requests: Vec<String>,
    model_requests: usize,
}

/// Fake provider: `GET /models` returns the catalog, each `POST
/// /chat/completions` returns the next queued assistant reply (or HTTP 500
/// once the queue is empty).
struct FakeProvider {
    base_url: String,
    recorded: Arc<Mutex<Recorded>>,
}

impl FakeProvider {
    fn start(replies: Vec<&str>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
        let recorded = Arc::new(Mutex::new(Recorded::default()));
        let shared = Arc::clone(&recorded);
        let mut replies: Vec<String> = replies.into_iter().map(str::to_owned).collect();
        replies.reverse();

        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let request = read_request(&mut stream);
                let (status, body) = if request.starts_with("GET") {
                    shared.lock().unwrap().model_requests += 1;
                    ("200 OK", CATALOG.to_string())
                } else {
                    shared.lock().unwrap().chat_requests.push(request);
                    match replies.pop() {
                        Some(content) => (
                            "200 OK",
                            serde_json::json!({"choices":[{"message":{"content":content}}]})
                                .to_string(),
                        ),
                        None => ("500 Internal Server Error", "{}".to_string()),
                    }
                };
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                );
            }
        });

        Self { base_url, recorded }
    }

    fn chat_count(&self) -> usize {
        self.recorded.lock().unwrap().chat_requests.len()
    }

    fn last_chat_request(&self) -> String {
        self.recorded
            .lock()
            .unwrap()
            .chat_requests
            .last()
            .cloned()
            .unwrap_or_default()
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut data = Vec::new();
    let mut chunk = [0_u8; 8192];
    while let Ok(count) = stream.read(&mut chunk) {
        if count == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..count]);
        let text = String::from_utf8_lossy(&data);
        if let Some(header_end) = text.find("\r\n\r\n") {
            let length = text[..header_end]
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            if data.len() >= header_end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&data).into_owned()
}

/// A scaffolded project plus an isolated home directory.
struct Workspace {
    _dir: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
}

impl Workspace {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let workspace = Self {
            project: dir.path().join("site"),
            home,
            _dir: dir,
        };

        let created = workspace
            .command(dir_of(&workspace.project))
            .args(["create", "--web", "site"])
            .output()
            .unwrap();
        assert!(created.status.success(), "{}", text(&created));

        // Tests never run a real package manager.
        workspace.set_install_permission("deny");
        workspace
    }

    fn set_install_permission(&self, level: &str) {
        let config_path = self.project.join("project.toml");
        let config = fs::read_to_string(&config_path).unwrap();
        let updated: Vec<String> = config
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("install_dependencies") {
                    format!("install_dependencies = \"{level}\"")
                } else {
                    line.to_string()
                }
            })
            .collect();
        assert!(updated.iter().any(|line| line.contains(level)));
        fs::write(&config_path, updated.join("\n") + "\n").unwrap();
    }

    /// Put a stand-in `pnpm` first on PATH. It logs each invocation, creates
    /// `node_modules` on install, and fails `run build` while `fail-build`
    /// exists next to it.
    #[cfg(unix)]
    fn craft_byok_with_fake_package_manager(&self, provider: &FakeProvider) -> Output {
        use std::os::unix::fs::PermissionsExt;
        let bin = self.home.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let script = format!(
            "#!/bin/sh\necho \"$PWD :: $*\" >> '{log}'\n\
             if [ \"$1\" = \"--version\" ]; then echo 9.0.0; exit 0; fi\n\
             if [ \"$1\" = \"install\" ]; then mkdir -p node_modules; fi\n\
             if [ \"$1\" = \"run\" ] && [ -f '{fail}' ]; then echo 'Type error: broken' >&2; exit 1; fi\n\
             exit 0\n",
            log = bin.join("log").display(),
            fail = bin.join("fail-build").display(),
        );
        let tool = bin.join("pnpm");
        fs::write(&tool, script).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();

        self.command(&self.project)
            .args(["craft", "--no-run"])
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .env("SAYN_API_KEY", "byok-secret-key")
            .env("SAYN_API_BASE", &provider.base_url)
            .env("SAYN_MODEL", "byok-model")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn package_manager_log(&self) -> String {
        fs::read_to_string(self.home.join("bin/log")).unwrap_or_default()
    }

    fn command(&self, cwd: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sayn"));
        command
            .current_dir(cwd)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("NO_COLOR", "1");
        command
    }

    fn craft_byok(&self, provider: &FakeProvider) -> Output {
        self.command(&self.project)
            .args(["craft", "--no-run"])
            .env("SAYN_API_KEY", "byok-secret-key")
            .env("SAYN_API_BASE", &provider.base_url)
            .env("SAYN_MODEL", "byok-model")
            .output()
            .unwrap()
    }

    fn sign_in_to_nous(&self, provider: &FakeProvider, selected_model: &str) {
        let state_dir = self.home.join(".sayn");
        fs::create_dir_all(&state_dir).unwrap();
        let expires = chrono::Utc::now() + chrono::Duration::hours(1);
        let auth = serde_json::json!({
            "portal_base_url": "http://127.0.0.1:9",
            "inference_base_url": provider.base_url,
            "client_id": "test-client",
            "scope": "inference:invoke",
            "token_type": "Bearer",
            "access_token": "nous-access-token",
            "refresh_token": null,
            "obtained_at": chrono::Utc::now().to_rfc3339(),
            "expires_at": expires.to_rfc3339(),
            "expires_in": 3600,
            "free_tier": false
        });
        fs::write(state_dir.join("auth.json"), auth.to_string()).unwrap();
        fs::write(
            state_dir.join("model.json"),
            serde_json::json!({"active_model": selected_model}).to_string(),
        )
        .unwrap();
    }

    fn generated(&self, relative: &str) -> PathBuf {
        self.project.join(".sayn/generated").join(relative)
    }

    fn edit_instructions(&self, text: &str) {
        fs::write(self.project.join("app/page.sayn"), text).unwrap();
    }

    fn latest_session(&self) -> serde_json::Value {
        let content = fs::read_to_string(self.project.join(".sayn/sessions/latest.json")).unwrap();
        serde_json::from_str(&content).unwrap()
    }

    fn manifest(&self) -> String {
        fs::read_to_string(self.project.join(".sayn/state/manifest.json")).unwrap()
    }
}

fn dir_of(project: &Path) -> &Path {
    project.parent().unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().unwrap()
}

#[test]
fn byok_craft_generates_the_application_and_records_the_run() {
    let provider = FakeProvider::start(vec![FULL_APP]);
    let workspace = Workspace::new();

    let output = workspace.craft_byok(&provider);

    assert_eq!(exit_code(&output), 0, "{}", text(&output));
    assert!(workspace.generated("app/page.tsx").exists());
    assert!(workspace.generated("package.json").exists());
    assert!(
        workspace
            .manifest()
            .contains(".sayn/generated/app/page.tsx")
    );
    let session = workspace.latest_session();
    assert_eq!(session["status"], "Completed");
    assert_eq!(session["model"], "byok-model");
    let request = provider.last_chat_request();
    assert!(request.contains("byok-model"));
    assert!(
        request
            .to_lowercase()
            .contains("authorization: bearer byok-secret-key")
    );
    // Dependency installation was denied, and craft says so instead of
    // claiming a build check that never ran.
    assert!(text(&output).contains("not run"), "{}", text(&output));
    assert!(!text(&output).contains("byok-secret-key"));
}

#[test]
fn unchanged_sources_reuse_the_output_without_another_inference_request() {
    let provider = FakeProvider::start(vec![FULL_APP]);
    let workspace = Workspace::new();
    assert_eq!(exit_code(&workspace.craft_byok(&provider)), 0);
    assert_eq!(provider.chat_count(), 1);

    let second = workspace.craft_byok(&provider);

    assert_eq!(exit_code(&second), 0, "{}", text(&second));
    assert_eq!(provider.chat_count(), 1);
}

#[test]
fn nous_craft_uses_the_stored_login_and_an_entitled_model() {
    let provider = FakeProvider::start(vec![FULL_APP]);
    let workspace = Workspace::new();
    workspace.sign_in_to_nous(&provider, "entitled-model");

    let output = workspace
        .command(&workspace.project)
        .args(["craft", "--no-run"])
        .output()
        .unwrap();

    assert_eq!(exit_code(&output), 0, "{}", text(&output));
    assert!(workspace.generated("app/page.tsx").exists());
    assert_eq!(workspace.latest_session()["provider"], "Nous Portal");
    let request = provider.last_chat_request();
    assert!(request.contains("entitled-model"));
    assert!(
        request
            .to_lowercase()
            .contains("authorization: bearer nous-access-token")
    );
    assert!(!text(&output).contains("nous-access-token"));
}

#[test]
fn model_outside_the_entitled_catalog_blocks_craft_before_inference() {
    let provider = FakeProvider::start(vec![FULL_APP]);
    let workspace = Workspace::new();
    workspace.sign_in_to_nous(&provider, "model-i-do-not-have");

    let output = workspace
        .command(&workspace.project)
        .args(["craft", "--no-run"])
        .output()
        .unwrap();

    assert_eq!(exit_code(&output), 6, "{}", text(&output));
    assert_eq!(provider.chat_count(), 0);
    assert!(!workspace.generated("app/page.tsx").exists());
}

#[test]
fn craft_without_credentials_fails_and_records_a_failed_run() {
    let workspace = Workspace::new();

    let output = workspace
        .command(&workspace.project)
        .args(["craft", "--no-run"])
        .output()
        .unwrap();

    assert_eq!(exit_code(&output), 6, "{}", text(&output));
    assert!(!workspace.generated("package.json").exists());
    assert_eq!(workspace.latest_session()["status"], "Failed");
    assert!(!workspace.project.join(".sayn/state/manifest.json").exists());
}

#[test]
fn schema_failure_preserves_the_previous_application_and_manifest() {
    let provider = FakeProvider::start(vec![FULL_APP, "I would rather chat about it."]);
    let workspace = Workspace::new();
    assert_eq!(exit_code(&workspace.craft_byok(&provider)), 0);
    let page_before = fs::read_to_string(workspace.generated("app/page.tsx")).unwrap();
    let manifest_before = workspace.manifest();
    workspace.edit_instructions("Create a portfolio with a contact page.\n");

    let output = workspace.craft_byok(&provider);

    assert_eq!(exit_code(&output), 6, "{}", text(&output));
    assert_eq!(provider.chat_count(), 2);
    assert_eq!(
        fs::read_to_string(workspace.generated("app/page.tsx")).unwrap(),
        page_before
    );
    assert_eq!(workspace.manifest(), manifest_before);
    assert_eq!(workspace.latest_session()["status"], "Failed");
    assert!(text(&output).contains("No generated files were changed"));
}

#[test]
fn validation_failure_preserves_the_previous_application_and_manifest() {
    let broken = r#"{"schema_version":1,"operations":[
      {"kind":"modify","path":"app/page.tsx","content":"second version"},
      {"kind":"delete","path":"app/layout.tsx"}]}"#;
    let provider = FakeProvider::start(vec![FULL_APP, broken]);
    let workspace = Workspace::new();
    assert_eq!(exit_code(&workspace.craft_byok(&provider)), 0);
    let manifest_before = workspace.manifest();
    workspace.edit_instructions("Create a portfolio without a layout.\n");

    let output = workspace.craft_byok(&provider);

    assert_eq!(exit_code(&output), 7, "{}", text(&output));
    assert!(
        fs::read_to_string(workspace.generated("app/page.tsx"))
            .unwrap()
            .contains("first version")
    );
    assert!(workspace.generated("app/layout.tsx").exists());
    assert_eq!(workspace.manifest(), manifest_before);
    assert_eq!(workspace.latest_session()["status"], "Failed");
}

#[test]
fn upstream_failure_after_a_good_run_keeps_the_application() {
    let provider = FakeProvider::start(vec![FULL_APP]);
    let workspace = Workspace::new();
    assert_eq!(exit_code(&workspace.craft_byok(&provider)), 0);
    workspace.edit_instructions("Create something else entirely.\n");

    // The reply queue is empty, so the provider answers HTTP 500.
    let output = workspace.craft_byok(&provider);

    assert_eq!(exit_code(&output), 6, "{}", text(&output));
    assert!(workspace.generated("app/page.tsx").exists());
    assert!(!text(&output).contains("byok-secret-key"));
}

#[cfg(unix)]
#[test]
fn permitted_build_check_runs_in_staging_before_the_application_is_replaced() {
    let provider = FakeProvider::start(vec![FULL_APP]);
    let workspace = Workspace::new();
    workspace.set_install_permission("allow");

    let output = workspace.craft_byok_with_fake_package_manager(&provider);

    assert_eq!(exit_code(&output), 0, "{}", text(&output));
    assert!(text(&output).contains("Production build check passed"));
    let log = workspace.package_manager_log();
    let staged: Vec<&str> = log
        .lines()
        .filter(|line| line.contains(".sayn/staging/"))
        .collect();
    assert!(
        staged.iter().any(|line| line.ends_with(":: install")),
        "{log}"
    );
    assert!(
        staged.iter().any(|line| line.ends_with(":: run build")),
        "{log}"
    );
    // Dependencies are then installed in the real output, which never
    // receives the staging copy's node_modules.
    assert!(
        log.lines()
            .any(|line| line.contains(".sayn/generated ::") && line.ends_with(":: install")),
        "{log}"
    );
    assert!(workspace.generated("node_modules").exists());
    assert!(
        !workspace
            .project
            .join(".sayn/staging")
            .read_dir()
            .is_ok_and(|mut d| d.next().is_some())
    );
}

#[cfg(unix)]
#[test]
fn failed_build_check_discards_the_candidate_and_keeps_the_previous_application() {
    let second = FULL_APP.replace("first version", "second version");
    let provider = FakeProvider::start(vec![FULL_APP, &second]);
    let workspace = Workspace::new();
    workspace.set_install_permission("allow");
    assert_eq!(
        exit_code(&workspace.craft_byok_with_fake_package_manager(&provider)),
        0
    );
    let manifest_before = workspace.manifest();
    workspace.edit_instructions("Create a portfolio with a typo in the code.\n");
    fs::write(workspace.home.join("bin/fail-build"), "").unwrap();

    let output = workspace.craft_byok_with_fake_package_manager(&provider);

    assert_eq!(exit_code(&output), 7, "{}", text(&output));
    assert!(
        text(&output).contains("exited with code 1"),
        "{}",
        text(&output)
    );
    assert!(
        fs::read_to_string(workspace.generated("app/page.tsx"))
            .unwrap()
            .contains("first version")
    );
    assert_eq!(workspace.manifest(), manifest_before);
    assert_eq!(workspace.latest_session()["status"], "Failed");
}
