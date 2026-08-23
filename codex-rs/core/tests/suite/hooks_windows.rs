use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use codex_protocol::models::PermissionProfile;
use core_test_support::hooks::trust_discovered_hooks;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use serde_json::Value;
use tempfile::TempDir;

const WINDOWS_POWERSHELL: &str =
    r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";

fn powershell_single_quoted_path(path: &Path) -> String {
    path.display().to_string().replace('\'', "''")
}

fn write_windows_pre_tool_use_hook(home: &Path) -> Result<()> {
    let script_path = home.join("pre_tool_use_hook.ps1");
    let log_path = home.join("pre_tool_use_hook_log.jsonl");
    let log_path = powershell_single_quoted_path(&log_path);
    let script = format!(
        r#"$payload = [Console]::In.ReadToEnd()
[System.IO.File]::AppendAllText('{log_path}', $payload + [Environment]::NewLine)
[Console]::Out.WriteLine('{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"blocked by windows pre hook"}}}}')
"#,
    );
    let hook_command = format!(
        r#"{WINDOWS_POWERSHELL} -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "{}""#,
        script_path.display(),
    );
    let hooks = serde_json::json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "^Bash$",
                "hooks": [{
                    "type": "command",
                    "command": hook_command,
                    "statusMessage": "running Windows pre tool use hook",
                }]
            }]
        }
    });

    fs::write(&script_path, script).context("write Windows pre tool use hook script")?;
    fs::write(home.join("hooks.json"), hooks.to_string()).context("write hooks.json")?;
    Ok(())
}

fn read_windows_pre_tool_use_hook_input(home: &Path) -> Result<Value> {
    let log = fs::read_to_string(home.join("pre_tool_use_hook_log.jsonl"))
        .context("read Windows pre tool use hook log")?;
    let line = log
        .lines()
        .find(|line| !line.trim().is_empty())
        .context("Windows pre tool use hook log should contain one payload")?;
    serde_json::from_str(line).context("parse Windows pre tool use hook payload")
}

#[tokio::test]
async fn pre_tool_use_json_deny_blocks_exec_command_before_execution_on_windows() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "pretooluse-exec-command-windows";
    let marker_dir = TempDir::new()?;
    let marker = marker_dir.path().join("exec-command-ran.txt");
    let marker_path = powershell_single_quoted_path(&marker);
    let command = format!(
        r#"{WINDOWS_POWERSHELL} -NoLogo -NoProfile -NonInteractive -Command "New-Item -ItemType File -Force -LiteralPath '{marker_path}' | Out-Null""#,
    );
    let args = serde_json::json!({ "cmd": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(call_id, "exec_command", &serde_json::to_string(&args)?),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "hook blocked it"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            write_windows_pre_tool_use_hook(home)
                .expect("failed to write Windows pre tool use hook fixture");
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    test.submit_turn_with_permission_profile(
        "run the blocked Windows shell command",
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    let output_item = requests[1].function_call_output(call_id);
    let output = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("shell command output string");
    assert!(
        output.contains("Command blocked by PreToolUse hook: blocked by windows pre hook"),
        "blocked tool output should surface the Windows hook reason; output={output:?}",
    );
    assert!(
        output.contains(&format!("Command: {command}")),
        "blocked tool output should surface the blocked command; output={output:?}",
    );
    assert!(
        !marker.exists(),
        "blocked Windows command must not create the execution marker at {}",
        marker.display(),
    );

    let hook_input = read_windows_pre_tool_use_hook_input(test.codex_home_path())?;
    assert_eq!(hook_input["hook_event_name"], "PreToolUse");
    assert_eq!(hook_input["tool_name"], "Bash");
    assert_eq!(hook_input["tool_use_id"], call_id);
    assert_eq!(hook_input["tool_input"]["command"], command);

    Ok(())
}
