# Coding Agents & Rex Integration

Lumiere turns code review into a live back-and-forth loop with coding agents running alongside it in [Rex](https://rex.dev) terminal panes — and still works seamlessly without Rex via stdout export or the `pi` extension.

## How live agent discovery works

When you open `lumiere diff`, Lumiere discovers running agents in the background:

1. It scans your Rex sessions and terminal panes.
2. It matches panes running a known coding agent reporting [Rex program status](https://www.superlogical.com/rex/docs/build/program-status) whose working directory is at or inside the reviewed repository (or workspace root).
3. It attaches to the most recently active matching agent pane and shows its name and state in the footer (for example, `· pi idle` or `· codex working`).
4. The attached agent's status refreshes automatically in the footer.

### Recognized agents

An agent is discovered automatically when it runs in a Rex pane and reports [Rex program status](https://www.superlogical.com/rex/docs/build/program-status):

- **Tested:** `pi`, `claude-code`, `codex` (0.163+), `antigravity`
- **Recognized by name:** `opencode`, `hermes`, `grok`, `grok-build`

Press <kbd>A</kbd> at any time to open the **Agents** picker, switch to another running agent pane, or choose `none` to detach. You can also target an explicit pane or terminal ID at startup with `--agent-target <TARGET>` (or the `--agent-block` alias for Rex).

## Safety guards before sending

Before sending comments to a terminal pane, Lumiere checks that delivery is safe:

- **Active-agent check:** If the attached agent exits back to your shell, Lumiere detaches automatically and refuses to paste comments into your shell.
- **Waiting-for-input guard:** If the agent is paused on a permission or confirmation prompt, Lumiere keeps your comments queued and notifies `<agent> is waiting for you` instead of submitting into its prompt.
- **Safe multi-line pasting:** Multi-line comments are delivered as a single pasted message rather than triggering premature submission line by line.

## What the agent receives and how it replies

When you send one or more comments, Lumiere delivers a structured prompt containing:

- Each thread's ID, file path, line range, side (`LEFT` / `RIGHT`), and (for the first comment on a thread) the anchored code snippet.
- Instructions asking the agent to reply inside the thread first (and wait for your approval before editing code).
- The exact `lumiere thread reply` command to run (plus `lumiere thread show` when continuing a thread with earlier comments).

Agents interact with review threads using three CLI subcommands:

```bash
# List threads that have been sent to the agent (--awaiting filters to ones waiting for a reply)
lumiere thread list [--awaiting] [--json] [--repo PATH]

# Inspect full history and anchored code for one thread
lumiere thread show <thread-id> [--json] [--repo PATH]

# Post a reply into the thread (immediately reflected in the open Lumiere UI)
lumiere thread reply <thread-id> --body "Agreed — I can extract that helper." [--repo PATH] [--author NAME]
echo "Multiline reply" | lumiere thread reply <thread-id> --stdin [--repo PATH]
```

> **Privacy of drafts:** Queued comments that you have not sent yet are invisible to `lumiere thread list`, `show`, and `reply`. Resolved threads reject new agent replies until you reopen them.

## Using `/lumiere` inside `pi`

After symlinking `pi-extension/` into `~/.pi/agent/extensions/lumiere` and running `/reload`:

```text
/lumiere                    # runs: lumiere diff origin/main...- --watch
/lumiere diff HEAD~3..-     # passes any custom lumiere arguments
```

- **Inside Rex:** `/lumiere` opens Lumiere in a zoomed split pane beside `pi`, passing `--agent-block` automatically so comments flow straight to that `pi` session while `pi` stays active behind the zoom. Unzoom the pane to view both side by side.
- **Outside Rex:** `/lumiere` temporarily suspends `pi`'s TUI while you review. Leave comments, then press <kbd>s</kbd> and <kbd>Enter</kbd> to exit and hand the formatted review directly to `pi` as a user message. Pressing <kbd>q</kbd> discards sending.

Set `LUMIERE_BIN=/path/to/lumiere` if your `lumiere` binary is not on `PATH`.
