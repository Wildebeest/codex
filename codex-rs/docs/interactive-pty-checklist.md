# Interactive PTY Support Checklist

> Tracking tasks required to add interactive pseudo-terminal support for exec commands and surface them in the TUI.

## 🧱 Foundation & Planning
- [ ] Confirm sandbox implications (seatbelt / landlock) when swapping to PTY-backed processes.
- [ ] Decide on gating criteria for interactive mode (CLI flag, approval option, command heuristics).
- [ ] Document UX expectations (when overlay appears, how to exit, keyboard shortcuts).

## 🗄️ Protocol & State Plumbing
- [x] Extend protocol: add `interactive` flag and optional PTY session id to `ExecCommandBeginEvent`.
- [x] Introduce `Op::ExecWriteInput { call_id, data }` for feeding keystrokes into live sessions.
- [x] Update `ExecCommandContext` and related structs to carry interactive metadata.
- [x] Track interactive controllers in session/turn state for cleanup and input routing.

## ⚙️ Exec Pipeline Updates
- [x] Allow `ExecParams` / `process_exec_tool_call` to select `StdioPolicy::PseudoTerminal`.
- [x] Reuse `exec_command::SessionManager` (portable-pty) to spawn interactive shells.
- [x] Stream PTY output into `ExecCommandOutputDelta` events while aggregating for final results.
- [x] Handle interrupts/timeouts by signalling the PTY child and notifying listeners.
- [x] Mirror existing approval logic so interactive commands still respect safety gates.

## 🖥️ TUI Overlay & Interaction
- [ ] Add `Overlay::Terminal` that enters alt-screen, renders PTY output, and shows status header.
- [ ] Forward key events (including Ctrl+C/Z, resize) to the PTY via the new op.
- [ ] Provide UI affordances to exit overlay (Esc, Ctrl+D when process ends, etc.).
- [ ] Sync overlay lifecycle with `ExecCommandBegin/End` so history cells remain accurate.
- [ ] Ensure accessibility: maintain color styling via `ansi_escape_line`, support narrow terminals.

## 🔄 Non-blocking Monitoring for Non-Interactive Runs
- [ ] Emit `ExecCommandOutputDelta` events for pipe-based executions too, enabling live log tails.
- [ ] Update `ExecCell` to display streamed output snippets during execution (spinner + tail).

## 🧪 Testing & Tooling
- [x] Run `cargo test -p codex-core`.
- [ ] Add integration test covering interactive session (write stdin, receive echo, exit cleanly).
- [ ] Exercise seatbelt/landlock flows to ensure PTY path respects sandbox limits.
- [ ] Update snapshot tests for exec history cells and new overlay UI.
- [ ] Manual QA script documenting interactive run scenarios (sudo, password prompts, cancellation).

## 📝 Documentation & Rollout
- [ ] Update `docs/` with user-facing instructions for interactive mode.
- [ ] Announce in changelog with upgrade notes (new CLI flags, approvals).
- [ ] Provide fallback guidance if PTY unavailable (e.g. remote shells without TTY support).
