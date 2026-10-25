# Cloud computer dispatch (preview)

Cloud conversations run on a computer in your Codewhale account. The terminal
and desktop open the same conversation ID; closing either view leaves the
task running. Availability depends on your account and the deployed service.

Sign in and select a running computer:

```sh
codewhale login
codewhale account computers list
codewhale account computers start COMPUTER_ID
codewhale dispatch --computer COMPUTER_ID --cloud-provider PROVIDER --cloud-model MODEL "your task"
```

Replace the uppercase values with IDs from your account and a model available
on that computer. Start and create operations use the account's existing
compute allowance, consent and admission checks. To create a computer, use
`codewhale account computers create --help` or Manage cloud computers in desktop.
Dispatch does not allocate or start a computer implicitly.

The returned conversation appears in desktop history for the same account.
Desktop's **Continue in terminal** control shows the matching command:

```sh
codewhale dispatch --thread CONVERSATION_ID
codewhale dispatch --thread CONVERSATION_ID "continue the task"
codewhale dispatch --thread CONVERSATION_ID --stop
codewhale dispatch --thread CONVERSATION_ID --json
```

Inspecting prints messages and pending approvals. Use `--approve APPROVAL_ID`
or `--deny APPROVAL_ID` after reviewing the request; approvals apply once.
Answer pending questions in desktop. `--stop` interrupts the turn and leaves
the computer running. Use `account computers pause COMPUTER_ID` to stop compute.

The computer's model access is separate from local terminal model credentials.
`--key-stdin` explicitly reads a key from standard input and passes it to the
selected computer's Engine with session scope. It stays in Engine memory and
is cleared on restart. Do not put keys in command arguments. Local credentials
are never automatically copied, and this does not enable the account key vault.

For a new conversation, specify both `--cloud-provider` and `--cloud-model`.
Continuation uses the Engine's last confirmed route, unless you specify both
flags again. These flags describe the remote computer; local `--provider` and
`--model` settings are not remote credentials or routes.

Every send prints a replay key before submission. After an uncertain response,
repeat the original command and prompt with `--operation-key KEY`. Keep the
original `--computer` target when retrying creation, or the original `--thread`
when retrying continuation. Reusing the key with changed content is refused.
Do not start another conversation to recover an uncertain send.

`codewhale account session` exposes the same computer conversation commands.
The earlier `dispatch` Agent Work flow, including its repository and file
context options, remains available without `--computer` or `--thread`.
Work-only flags cannot be mixed with computer conversations. This preview does
not move local folders, local credentials or repository checkouts into a VM.
