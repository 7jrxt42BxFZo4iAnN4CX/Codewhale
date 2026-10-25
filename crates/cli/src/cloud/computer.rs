use super::*;
use serde_json::{Value, json};

#[derive(Debug, Default, Args)]
pub(crate) struct ComputerOptions {
    #[arg(
        long,
        value_name = "ID",
        help = "Run on this account cloud computer (preview)"
    )]
    pub computer: Option<String>,
    #[arg(
        long,
        value_name = "ID",
        help = "Continue or inspect a desktop/CLI cloud conversation"
    )]
    pub thread: Option<String>,
    #[arg(
        long,
        requires = "cloud_model",
        help = "Provider on the cloud computer"
    )]
    pub cloud_provider: Option<String>,
    #[arg(
        long,
        requires = "cloud_provider",
        help = "Model on the cloud computer"
    )]
    pub cloud_model: Option<String>,
    #[arg(
        long,
        help = "Read a provider key from stdin into VM memory for this running computer"
    )]
    pub key_stdin: bool,
    #[arg(long, help = "Stop the current turn of --thread")]
    pub stop: bool,
    #[arg(
        long,
        value_name = "ID",
        conflicts_with = "deny",
        help = "Allow one pending tool request in --thread"
    )]
    pub approve: Option<String>,
    #[arg(
        long,
        value_name = "ID",
        help = "Deny one pending tool request in --thread"
    )]
    pub deny: Option<String>,
    #[arg(long, help = "Print the session receipt as JSON")]
    pub json: bool,
}

impl ComputerOptions {
    pub fn selected(&self) -> bool {
        self.computer.is_some()
            || self.thread.is_some()
            || self.cloud_provider.is_some()
            || self.cloud_model.is_some()
            || self.key_stdin
            || self.stop
            || self.approve.is_some()
            || self.deny.is_some()
            || self.json
    }
}

#[derive(Debug, Args)]
pub(crate) struct SessionArgs {
    #[arg(value_name = "PROMPT")]
    pub prompt: Vec<String>,
    #[command(flatten)]
    pub options: ComputerOptions,
    #[arg(
        long,
        help = "Reuse this key and the same prompt after an uncertain send"
    )]
    pub operation_key: Option<String>,
}

struct SessionClient<'a, 'b, T: CloudTransport> {
    client: &'a CloudClient<'b, T>,
    account: String,
    session: String,
}

impl<T: CloudTransport> SessionClient<'_, '_, T> {
    fn snapshot(&self) -> Result<AccountSessionSnapshot> {
        let snapshot = self.client.account_store.snapshot()?;
        let stored = snapshot
            .load()?
            .context("Sign in with `codewhale login` first")?;
        if stored.bundle.user.as_ref().map(|user| &user.id) != Some(&self.account)
            || stored.bundle.session.as_ref().map(|session| &session.id) != Some(&self.session)
        {
            bail!("The signed-in account changed. Recover the original session before retrying.");
        }
        Ok(snapshot)
    }

    fn request(&self, method: HttpMethod, path: &str, body: Option<Value>) -> Result<Value> {
        let snapshot = self.snapshot()?;
        let (response, _) = self.client.execute_from_snapshot(
            method,
            path,
            body.as_ref().map(json_body).transpose()?,
            snapshot,
        )?;
        self.snapshot()?;
        expect_json(response, &[200, 201, 202])
    }

    fn operation(
        &self,
        thread: &str,
        computer: &str,
        runtime: &str,
        kind: &str,
        input: Value,
    ) -> Result<Value> {
        let value = self.request(
            HttpMethod::Post,
            &format!("/v1/threads/{thread}/computer"),
            Some(
                json!({ "computerId": computer, "operation": { "kind": kind, "request": input } }),
            ),
        )?;
        check_binding(&value, thread, computer)?;
        if value["runtimeThreadId"].as_str() != Some(runtime)
            || !value.get("result").is_some_and(Value::is_object)
        {
            bail!("The computer returned a different session. Refresh before continuing.");
        }
        Ok(value["result"].clone())
    }
}

fn check_binding(value: &Value, thread: &str, computer: &str) -> Result<String> {
    if value["threadId"].as_str() != Some(thread)
        || value["computerId"].as_str() != Some(computer)
        || value["state"] != "bound"
    {
        bail!("The cloud conversation is not bound to the selected computer.");
    }
    let runtime = value["runtimeThreadId"]
        .as_str()
        .context("The computer did not return its session ID")?;
    validate_turn_id(runtime)?;
    Ok(runtime.to_string())
}

pub(super) fn run<T: CloudTransport, W: Write>(
    args: SessionArgs,
    client: &CloudClient<'_, T>,
    machine: &machine::MachineKeyEnv,
    out: &mut W,
    key_reader: &mut dyn FnMut(KeyReadMode) -> Result<String>,
) -> Result<()> {
    if machine.is_present() {
        bail!(
            "Cloud conversations require an account login; unset CODEWHALE_API_KEY and run `codewhale login`."
        );
    }
    let options = args.options;
    let prompt = args.prompt.join(" ").trim().to_string();
    if [
        !prompt.is_empty(),
        options.stop,
        options.approve.is_some(),
        options.deny.is_some(),
    ]
    .into_iter()
    .filter(|x| *x)
    .count()
        > 1
    {
        bail!("Choose a prompt, --stop, --approve, or --deny.");
    }
    if prompt.chars().count() > 128_000 {
        bail!("The cloud prompt is too long; keep it under 128,000 characters.");
    }
    if options.thread.is_none() && (options.computer.is_none() || prompt.is_empty()) {
        bail!("Start with --computer <id> and a prompt, or continue with --thread <id>.");
    }
    if options.thread.is_none()
        && (options.cloud_provider.is_none() || options.cloud_model.is_none())
    {
        bail!("Choose --cloud-provider and --cloud-model for a new cloud conversation.");
    }
    if let Some(value) = &options.thread {
        validate_resource_id(value, "Conversation")?;
    }
    if let Some(value) = &options.computer {
        validate_resource_id(value, "Computer")?;
    }
    if let (Some(provider), Some(model)) = (&options.cloud_provider, &options.cloud_model) {
        validate_model_route(provider, model)?;
        if model.len() > 200 {
            bail!("Cloud model names must be at most 200 bytes.");
        }
    }
    let operation_key = args
        .operation_key
        .unwrap_or_else(|| format!("dispatch-{}", dispatch_stamp(&prompt)));
    validate_operation_key(&operation_key)?;
    if !operation_key.starts_with(|value: char| value.is_ascii_alphanumeric()) {
        bail!("The operation key must start with a letter or number.");
    }
    let user = client.me()?;
    let stored = client
        .load_auth()?
        .context("Sign in with `codewhale login` first")?;
    let session = stored
        .bundle
        .session
        .as_ref()
        .filter(|session| !session.id.is_empty())
        .context("Sign in again to establish a durable account session")?
        .id
        .clone();
    let account = SessionClient {
        client,
        account: user.id,
        session,
    };
    account.snapshot()?;
    let key = if options.key_stdin {
        Some(zeroize::Zeroizing::new(key_reader(KeyReadMode::Stdin)?))
    } else {
        None
    };
    let mut thread = if let Some(id) = &options.thread {
        account.request(HttpMethod::Get, &format!("/v1/threads/{id}"), None)?["thread"].clone()
    } else {
        Value::Null
    };
    if let Some(id) = &options.thread {
        if thread["id"].as_str() != Some(id)
            || thread["userId"].as_str() != Some(&account.account)
            || thread["archivedAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        {
            bail!("This account conversation is unavailable.");
        }
    }
    let computer = options
        .computer
        .as_deref()
        .or_else(|| thread["computerId"].as_str())
        .context("This conversation has no cloud computer")?
        .to_string();
    validate_resource_id(&computer, "Computer")?;
    if options.thread.is_some() && thread["computerId"].as_str() != Some(&computer) {
        bail!("The conversation belongs to a different computer.");
    }
    let machine = account.request(HttpMethod::Get, &format!("/api/computers/{computer}"), None)?;
    if machine["computer"]["id"].as_str() != Some(&computer)
        || machine["computer"]["ownerId"].as_str() != Some(&account.account)
    {
        bail!("This computer does not belong to the signed-in account.");
    }
    if machine["computer"]["status"] != "running" {
        bail!(
            "Start this computer with `codewhale account computers start {computer}`, then retry the same session."
        );
    }
    if !prompt.is_empty() {
        eprintln!(
            "Replay key: {operation_key}. Keep the same target, prompt, and --operation-key when retrying an uncertain send."
        );
    }
    if options.thread.is_none() {
        let value = account.request(HttpMethod::Post, "/v1/threads", Some(json!({
            "title": prompt.chars().take(120).collect::<String>(), "agentId": "", "computerId": computer,
            "productMode": "chat", "mode": "chat", "billingMode": "byok_external", "requiresByok": true,
            "modelProvider": options.cloud_provider, "model": options.cloud_model, "operationKey": operation_key,
        })))?;
        thread = value.get("thread").unwrap_or(&value).clone();
        if thread["computerId"].as_str() != Some(&computer)
            || thread["userId"].as_str() != Some(&account.account)
        {
            bail!("Conversation creation is unconfirmed. Retry with the same operation key.");
        }
    }
    let thread_id = thread["id"]
        .as_str()
        .context("Conversation creation did not return an ID")?;
    validate_resource_id(thread_id, "Conversation")?;
    let binding_path = format!("/v1/threads/{thread_id}/computer");
    let existing_binding = if options.thread.is_some() {
        account.request(HttpMethod::Get, &binding_path, None)?
    } else {
        Value::Null
    };
    let binding = if existing_binding["state"] == "bound" {
        existing_binding
    } else if options.stop {
        bail!("This conversation has no connected task to stop.");
    } else {
        account.request(
            HttpMethod::Post,
            &binding_path,
            Some(json!({ "computerId": computer })),
        )?
    };
    let runtime = check_binding(&binding, thread_id, &computer)?;
    let detail = account.operation(thread_id, &computer, &runtime, "read", json!({}))?;
    if detail["thread"]["id"].as_str() != Some(&runtime)
        || !detail["turns"].is_array()
        || !detail["items"].is_array()
    {
        bail!("The computer did not confirm this conversation's task history.");
    }
    let turns = detail["turns"].as_array().unwrap();
    if turns
        .iter()
        .any(|turn| turn["thread_id"].as_str() != Some(&runtime))
    {
        bail!("The computer returned a task from a different conversation.");
    }
    let latest_id = detail["thread"]["latest_turn_id"]
        .as_str()
        .filter(|id| !id.is_empty());
    let latest = latest_id.and_then(|id| {
        turns
            .iter()
            .find(|turn| turn["id"] == id && turn["routing_settlement"] != true)
    });
    let provider = options
        .cloud_provider
        .as_deref()
        .or_else(|| latest.and_then(|turn| turn["effective_provider"].as_str()))
        .or_else(|| {
            latest_id
                .is_none()
                .then(|| detail["thread"]["model_provider"].as_str())
                .flatten()
        })
        .unwrap_or("");
    let model = options
        .cloud_model
        .as_deref()
        .or_else(|| latest.and_then(|turn| turn["effective_model"].as_str()))
        .or_else(|| {
            latest_id
                .is_none()
                .then(|| detail["thread"]["model"].as_str())
                .flatten()
        })
        .unwrap_or("");
    let provider_id = if options.cloud_provider.is_some() {
        None
    } else {
        latest
            .and_then(|turn| turn["effective_provider_id"].as_str())
            .filter(|id| !id.is_empty())
            .or_else(|| {
                latest_id
                    .is_none()
                    .then(|| {
                        detail["thread"]["model_provider_id"]
                            .as_str()
                            .filter(|id| !id.is_empty())
                    })
                    .flatten()
            })
    };
    if !prompt.is_empty() || key.is_some() {
        validate_model_route(provider, model).context("The last model route is unconfirmed. Choose --cloud-provider and --cloud-model explicitly")?;
    }
    if let Some(key) = key {
        validate_api_key(&key)?;
        account.operation(
            thread_id,
            &computer,
            &runtime,
            "key",
            json!({ "provider": provider_id.unwrap_or(provider), "key": key.as_str() }),
        )?;
    }
    let result = if !prompt.is_empty() {
        account.operation(thread_id, &computer, &runtime, "turn", json!({
            "prompt": prompt, "operation_key": operation_key, "model": model,
            "model_provider": provider, "model_provider_id": provider_id,
        })).with_context(|| format!("Session {thread_id}: retry the original request with --operation-key {operation_key}"))?
    } else if options.stop {
        if let Some(turn) = turns
            .iter()
            .rev()
            .find(|turn| matches!(turn["status"].as_str(), Some("queued" | "in_progress")))
        {
            let stopped = account.operation(
                thread_id,
                &computer,
                &runtime,
                "stop",
                json!({ "turnId": turn["id"] }),
            )?;
            if stopped["id"] != turn["id"] || stopped["thread_id"].as_str() != Some(&runtime) {
                bail!("The computer did not confirm which task received the stop request.");
            }
            json!({ "turn": stopped, "message": "Stop requested. Inspect the conversation to confirm it has stopped." })
        } else {
            json!({ "status": "idle", "message": "No running turn" })
        }
    } else if let Some(id) = options.approve.as_ref().or(options.deny.as_ref()) {
        let reply = account.operation(thread_id, &computer, &runtime, "approve", json!({ "id": id, "decision": if options.approve.is_some() { "allow" } else { "deny" } }))?;
        if reply["delivered"] != true {
            bail!("This approval is no longer waiting. Refresh the conversation.");
        }
        reply
    } else {
        detail
    };
    if let Some(turn) = result.get("turn") {
        if turn["thread_id"].as_str() != Some(&runtime) {
            bail!("The computer returned a different task");
        }
    }
    if options.json {
        writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&json!({ "threadId": thread_id, "computerId": computer,
            "runtimeThreadId": runtime, "operationKey": (!prompt.is_empty()).then_some(&operation_key), "result": result }))?
        )?;
    } else {
        writeln!(out, "Cloud conversation: {thread_id}\nComputer: {computer}")?;
        if let Some(turn) = result.get("turn") {
            writeln!(
                out,
                "Task: {} ({})",
                printable(turn["id"].as_str().unwrap_or("")),
                printable(turn["status"].as_str().unwrap_or("unconfirmed"))
            )?;
        }
        if let Some(message) = result["message"].as_str() {
            writeln!(out, "{}", printable(message))?;
        }
        if result["delivered"] == true {
            writeln!(out, "Decision delivered to the running task.")?;
        }
        if let Some(items) = result["items"].as_array() {
            for item in items {
                let text = item["detail"]
                    .as_str()
                    .filter(|text| !text.is_empty())
                    .or_else(|| item["summary"].as_str())
                    .unwrap_or("");
                writeln!(
                    out,
                    "{}: {}",
                    printable(item["kind"].as_str().unwrap_or("status")),
                    printable(text)
                )?;
            }
        }
        if let Some(approvals) = result["pending_approvals"].as_array() {
            for approval in approvals {
                writeln!(
                    out,
                    "Approval {}: {}",
                    printable(approval["id"].as_str().unwrap_or("")),
                    printable(&serde_json::to_string(approval)?)
                )?;
            }
        }
        if result["pending_user_inputs"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
        {
            writeln!(
                out,
                "A question is waiting. Open this conversation in desktop to answer it."
            )?;
        }
        writeln!(
            out,
            "Inspect: codewhale dispatch --thread {thread_id}\nContinue: codewhale dispatch --thread {thread_id} \"your next task\"\nStop: codewhale dispatch --thread {thread_id} --stop"
        )?;
    }
    Ok(())
}
