use assert_cmd::Command;
use memory_domain::ScopeId;
use serde_json::Value;
use std::{env, fs, path::Path};
use tempfile::tempdir;

fn test_database_url() -> String {
    env::var("MEAT_MEMORY_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into())
}

#[test]
fn cli_remember_and_search_work_with_mock_provider_registry() {
    let tempdir = tempdir().unwrap();
    let config_path = write_test_config(tempdir.path());
    let scope_id = ScopeId::new();

    let remember = run_cli(
        &config_path,
        &[
            "remember",
            "--scope-id",
            scope_id.as_str(),
            "--title",
            "CLI gateway memory",
            "--body",
            "Meat Memory keeps long-term context for gateway and codex workflows.",
            "--memory-kind",
            "decision",
            "--json",
        ],
    );
    assert_eq!(remember["scope_id"], scope_id.as_str());
    assert_eq!(remember["memory_kind"], "decision");
    assert_eq!(remember["wrote_pg"], true);
    assert_eq!(remember["wrote_markdown"], true);

    let search = run_cli(
        &config_path,
        &[
            "search",
            "gateway codex",
            "--scope-id",
            scope_id.as_str(),
            "--limit",
            "5",
            "--json",
        ],
    );
    assert_eq!(search["scope_id"], scope_id.as_str());
    assert_eq!(search["memory_count"], 1);
    assert!(search["entity_count"].as_u64().unwrap() >= 1);
}

#[test]
fn cli_remember_and_search_support_chinese_acceptance_corpus() {
    let tempdir = tempdir().unwrap();
    let config_path = write_test_config(tempdir.path());
    let scope_id = ScopeId::new();

    let remember = run_cli(
        &config_path,
        &[
            "remember",
            "--scope-id",
            scope_id.as_str(),
            "--title",
            "中文发布验收规则",
            "--body",
            "项目 `服务网关` 依赖 `PostgreSQL`，`服务网关` 记录 `发布手册`。发布前必须通过回归与验收。",
            "--memory-kind",
            "procedure",
            "--json",
        ],
    );
    assert_eq!(remember["scope_id"], scope_id.as_str());
    assert_eq!(remember["memory_kind"], "procedure");

    let search = run_cli(
        &config_path,
        &[
            "search",
            "回归 验收",
            "--scope-id",
            scope_id.as_str(),
            "--limit",
            "5",
            "--json",
        ],
    );
    assert_eq!(search["scope_id"], scope_id.as_str());
    assert_eq!(search["memory_count"], 1);
    assert!(search["relation_count"].as_u64().unwrap() >= 1);
}

#[test]
fn cli_remember_image_returns_mock_vision_alias_and_searchable_caption() {
    let tempdir = tempdir().unwrap();
    let config_path = write_test_config(tempdir.path());
    let image_path = tempdir.path().join("login.png");
    let scope_id = ScopeId::new();

    fs::write(&image_path, [137_u8, 80, 78, 71, 13, 10, 26, 10]).unwrap();

    let remember = run_cli(
        &config_path,
        &[
            "remember-image",
            "--scope-id",
            scope_id.as_str(),
            "--title",
            "登录页截图",
            "--body",
            "Codex 登录页截图",
            "--file",
            image_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(remember["scope_id"], scope_id.as_str());
    assert_eq!(remember["vision_model_alias"], "mock_gemini_vision");
    assert!(
        remember["vision_caption"]
            .as_str()
            .unwrap()
            .contains("检测到一张 image/png 图片")
    );

    let search = run_cli(
        &config_path,
        &[
            "search",
            "image/png",
            "--scope-id",
            scope_id.as_str(),
            "--limit",
            "5",
            "--json",
        ],
    );
    assert_eq!(search["memory_count"], 1);
}

#[test]
fn cli_remember_image_returns_llm_notice_after_failover() {
    let tempdir = tempdir().unwrap();
    let config_path = write_failover_test_config(tempdir.path());
    let image_path = tempdir.path().join("failover.png");
    let scope_id = ScopeId::new();

    fs::write(&image_path, [137_u8, 80, 78, 71, 13, 10, 26, 10]).unwrap();

    let remember = run_cli(
        &config_path,
        &[
            "remember-image",
            "--scope-id",
            scope_id.as_str(),
            "--title",
            "回退截图",
            "--body",
            "主视觉模型不可用时的图片写入",
            "--file",
            image_path.to_str().unwrap(),
            "--json",
        ],
    );

    assert_eq!(remember["scope_id"], scope_id.as_str());
    assert_eq!(remember["vision_model_alias"], "claude_vision");
    assert_eq!(
        remember["llm_notice"],
        "Gemini Vision LLM 不可用，已经切换到Claude Vision"
    );
}

fn run_cli(config_path: &Path, args: &[&str]) -> Value {
    let output = Command::cargo_bin("memory-cli")
        .unwrap()
        .env("MEAT_MEMORY_CONFIG", config_path)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    parse_json_output(&output)
}

fn parse_json_output(output: &[u8]) -> Value {
    if let Ok(value) = serde_json::from_slice(output) {
        return value;
    }

    let start = output
        .iter()
        .position(|byte| *byte == b'{')
        .expect("stdout should contain JSON payload");
    serde_json::from_slice(&output[start..]).unwrap()
}

fn write_test_config(root: &Path) -> std::path::PathBuf {
    write_config(
        root,
        r#"[[models.providers]]
provider = "gemini"
display_name = "Mock Gemini"
base_url = "https://mock-gemini.invalid"
api_key_env = "GEMINI_API_KEY"
enabled = true

[[models.catalog]]
alias = "mock_gemini_reasoning"
provider = "gemini"
remote_model_id = "mock-gemini-reasoning"
display_name = "Mock Gemini Reasoning"
capabilities = ["reasoning", "extraction"]
deployment = "cloud"
locale = "zh-CN"
priority = 100
enabled = true

[[models.catalog]]
alias = "mock_gemini_vision"
provider = "gemini"
remote_model_id = "mock-gemini-vision"
display_name = "Mock Gemini Vision"
capabilities = ["vision"]
deployment = "cloud"
locale = "zh-CN"
priority = 100
enabled = true

[[models.catalog]]
alias = "mock_gemini_embedding"
provider = "gemini"
remote_model_id = "mock-gemini-embedding"
display_name = "Mock Gemini Embedding"
capabilities = ["embedding"]
deployment = "cloud"
locale = "zh-CN"
priority = 100
enabled = true

[models.routing.reasoning]
primary = "mock_gemini_reasoning"
fallbacks = []

[models.routing.extraction]
primary = "mock_gemini_reasoning"
fallbacks = []

[models.routing.vision]
primary = "mock_gemini_vision"
fallbacks = []

[models.routing.embedding]
primary = "mock_gemini_embedding"
fallbacks = []
"#,
    )
}

fn write_failover_test_config(root: &Path) -> std::path::PathBuf {
    write_config(
        root,
        r#"[[models.providers]]
provider = "gemini"
display_name = "Gemini"
base_url = "https://mock-gemini.invalid"
api_key_env = "MEAT_MEMORY_TEST_GEMINI_MISSING"
enabled = true

[[models.providers]]
provider = "anthropic"
display_name = "Claude"
base_url = "https://mock-claude.invalid"
enabled = true

[[models.catalog]]
alias = "gemini_reasoning"
provider = "gemini"
remote_model_id = "gemini-reasoning"
display_name = "Gemini Reasoning"
capabilities = ["reasoning", "extraction"]
deployment = "cloud"
locale = "zh-CN"
priority = 100
enabled = true

[[models.catalog]]
alias = "gemini_vision"
provider = "gemini"
remote_model_id = "gemini-vision"
display_name = "Gemini Vision"
capabilities = ["vision"]
deployment = "cloud"
locale = "zh-CN"
priority = 100
enabled = true

[[models.catalog]]
alias = "claude_reasoning"
provider = "anthropic"
remote_model_id = "claude-reasoning"
display_name = "Claude Reasoning"
capabilities = ["reasoning", "extraction"]
deployment = "cloud"
locale = "zh-CN"
priority = 95
enabled = true

[[models.catalog]]
alias = "claude_vision"
provider = "anthropic"
remote_model_id = "claude-vision"
display_name = "Claude Vision"
capabilities = ["vision"]
deployment = "cloud"
locale = "zh-CN"
priority = 95
enabled = true

[[models.catalog]]
alias = "claude_embedding"
provider = "anthropic"
remote_model_id = "claude-embedding"
display_name = "Claude Embedding"
capabilities = ["embedding"]
deployment = "cloud"
locale = "zh-CN"
priority = 95
enabled = true

[models.routing.reasoning]
primary = "gemini_reasoning"
fallbacks = ["claude_reasoning"]

[models.routing.extraction]
primary = "gemini_reasoning"
fallbacks = ["claude_reasoning"]

[models.routing.vision]
primary = "gemini_vision"
fallbacks = ["claude_vision"]

[models.routing.embedding]
primary = "claude_embedding"
fallbacks = []
"#,
    )
}

fn write_config(root: &Path, model_config: &str) -> std::path::PathBuf {
    let config_path = root.join("memory-cli-test.toml");
    let markdown_root = root.join("markdown");
    let assets_root = root.join("assets");

    let config = format!(
        r#"[server]
bind = "127.0.0.1:0"
shutdown_grace_period_secs = 10

[logging]
level = "info"
format = "pretty"

[markdown]
root = "{markdown_root}"

[postgres]
app_name = "meat-memory"
database_url = "{database_url}"

[assets]
root = "{assets_root}"

[models]
default_locale = "zh-CN"

{model_config}

[sync]
mode = "dual_write"

[features]
enable_pg = true
enable_markdown = true
enable_http = true
enable_mcp = true
"#,
        markdown_root = markdown_root.display(),
        database_url = test_database_url(),
        assets_root = assets_root.display(),
        model_config = model_config,
    );

    fs::write(&config_path, config).unwrap();
    config_path
}
