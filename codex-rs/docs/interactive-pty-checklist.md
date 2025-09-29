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
- [x] Add `Overlay::Terminal` that enters alt-screen, renders PTY output, and shows status header.
  - [x] Define `TerminalOverlay` struct + `TerminalOverlayOpen` params (command, call_id, started_at, cwd, session id).
  - [x] Render scrollback buffer using `ansi_escape_line` + wrapping helpers; show header with command + cwd.
  - [x] Track spinner / elapsed timer in header, show status badges (Running, Timed Out, Exit Code).
- [x] Forward key events (including Ctrl+C/Z, resize) to the PTY via the new op.
  - [x] Implement key-to-byte mapping (Enter, Backspace, arrows, Ctrl key combos) and send `Op::ExecWriteInput`.
  - [x] Handle window resize events – send size deltas via core once protocol supports it (stub for now).
- [x] Provide UI affordances to exit overlay (Esc, Ctrl+D when process ends, etc.).
  - [x] Close overlay when process exits cleanly and buffer drained; restore transcript scrollback.
  - [x] Allow user to exit manually with Esc/`Ctrl+[` while keeping process alive (detach behavior decision).
- [ ] Sync overlay lifecycle with `ExecCommandBegin/End` so history cells remain accurate.
  - [x] Open overlay on interactive `ExecCommandBegin`; stream deltas to buffer; update status on `ExecCommandEnd`.
  - [x] Defer history inserts while overlay active; flush backlog on close.
  - [x] Highlight active history cell or tail transcript when overlay auto-closes.
- [ ] Ensure accessibility: maintain color styling via `ansi_escape_line`, support narrow terminals.
  - [x] Clamp line width to viewport; wrap with `word_wrap_lines`; support fallback when width < 4 cols.
  - [ ] Verify screen-reader friendly hints / focus states in overlay.

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
