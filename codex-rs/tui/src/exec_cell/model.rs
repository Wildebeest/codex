use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

use codex_core::protocol::ExecOutputStream;
use codex_protocol::parse_command::ParsedCommand;

const LIVE_STREAM_TAIL_CAP_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Default)]
pub(crate) struct LiveCommandOutput {
    stdout: LiveStreamTail,
    stderr: LiveStreamTail,
    last_stream: Option<ExecOutputStream>,
}

impl LiveCommandOutput {
    pub(crate) fn append(&mut self, stream: ExecOutputStream, chunk: &[u8]) {
        match stream {
            ExecOutputStream::Stdout => self.stdout.push(chunk),
            ExecOutputStream::Stderr => self.stderr.push(chunk),
        }
        self.last_stream = Some(stream);
    }

    pub(crate) fn clear(&mut self) {
        self.stdout.clear();
        self.stderr.clear();
        self.last_stream = None;
    }

    pub(crate) fn to_command_output(&self) -> Option<CommandOutput> {
        let stdout = self.stdout.as_string();
        let stderr = self.stderr.as_string();

        if stdout.is_empty() && stderr.is_empty() {
            return None;
        }

        let exit_code = if matches!(self.last_stream, Some(ExecOutputStream::Stderr)) {
            1
        } else {
            0
        };

        Some(CommandOutput {
            exit_code,
            stdout,
            stderr,
            formatted_output: String::new(),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct LiveStreamTail {
    buf: VecDeque<u8>,
}

impl LiveStreamTail {
    fn push(&mut self, chunk: &[u8]) {
        self.buf.extend(chunk);
        let excess = self.buf.len().saturating_sub(LIVE_STREAM_TAIL_CAP_BYTES);
        if excess > 0 {
            for _ in 0..excess {
                self.buf.pop_front();
            }
        }
    }

    fn clear(&mut self) {
        self.buf.clear();
    }

    fn as_string(&self) -> String {
        if self.buf.is_empty() {
            return String::new();
        }

        let mut bytes = Vec::with_capacity(self.buf.len());
        let (a, b) = self.buf.as_slices();
        bytes.extend_from_slice(a);
        bytes.extend_from_slice(b);
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CommandOutput {
    pub(crate) exit_code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) formatted_output: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExecCall {
    pub(crate) call_id: String,
    pub(crate) command: Vec<String>,
    pub(crate) parsed: Vec<ParsedCommand>,
    pub(crate) output: Option<CommandOutput>,
    pub(crate) start_time: Option<Instant>,
    pub(crate) duration: Option<Duration>,
    pub(crate) live_output: LiveCommandOutput,
}

#[derive(Debug)]
pub(crate) struct ExecCell {
    pub(crate) calls: Vec<ExecCall>,
}

impl ExecCell {
    pub(crate) fn new(call: ExecCall) -> Self {
        Self { calls: vec![call] }
    }

    pub(crate) fn with_added_call(
        &self,
        call_id: String,
        command: Vec<String>,
        parsed: Vec<ParsedCommand>,
    ) -> Option<Self> {
        let call = ExecCall {
            call_id,
            command,
            parsed,
            output: None,
            start_time: Some(Instant::now()),
            duration: None,
            live_output: LiveCommandOutput::default(),
        };
        if self.is_exploring_cell() && Self::is_exploring_call(&call) {
            Some(Self {
                calls: [self.calls.clone(), vec![call]].concat(),
            })
        } else {
            None
        }
    }

    pub(crate) fn complete_call(
        &mut self,
        call_id: &str,
        output: CommandOutput,
        duration: Duration,
    ) {
        if let Some(call) = self.calls.iter_mut().rev().find(|c| c.call_id == call_id) {
            call.output = Some(output);
            call.duration = Some(duration);
            call.start_time = None;
            call.live_output.clear();
        }
    }

    pub(crate) fn should_flush(&self) -> bool {
        !self.is_exploring_cell() && self.calls.iter().all(|c| c.output.is_some())
    }

    pub(crate) fn mark_failed(&mut self) {
        for call in self.calls.iter_mut() {
            if call.output.is_none() {
                let elapsed = call
                    .start_time
                    .map(|st| st.elapsed())
                    .unwrap_or_else(|| Duration::from_millis(0));
                call.start_time = None;
                call.duration = Some(elapsed);
                call.output = Some(CommandOutput {
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: String::new(),
                    formatted_output: String::new(),
                });
                call.live_output.clear();
            }
        }
    }

    pub(crate) fn is_exploring_cell(&self) -> bool {
        self.calls.iter().all(Self::is_exploring_call)
    }

    pub(crate) fn is_active(&self) -> bool {
        self.calls.iter().any(|c| c.output.is_none())
    }

    pub(crate) fn active_start_time(&self) -> Option<Instant> {
        self.calls
            .iter()
            .find(|c| c.output.is_none())
            .and_then(|c| c.start_time)
    }

    pub(crate) fn iter_calls(&self) -> impl Iterator<Item = &ExecCall> {
        self.calls.iter()
    }

    pub(crate) fn append_live_chunk(
        &mut self,
        call_id: &str,
        stream: ExecOutputStream,
        chunk: &[u8],
    ) -> bool {
        if let Some(call) = self
            .calls
            .iter_mut()
            .rev()
            .find(|c| c.call_id == call_id && c.output.is_none())
        {
            call.live_output.append(stream, chunk);
            return true;
        }
        false
    }

    pub(super) fn is_exploring_call(call: &ExecCall) -> bool {
        !call.parsed.is_empty()
            && call.parsed.iter().all(|p| {
                matches!(
                    p,
                    ParsedCommand::Read { .. }
                        | ParsedCommand::ListFiles { .. }
                        | ParsedCommand::Search { .. }
                )
            })
    }
}
