//! Session-wide mutable state.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use codex_protocol::models::ResponseItem;

use portable_pty::ChildKiller;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc;

use crate::conversation_history::ConversationHistory;
use crate::protocol::RateLimitSnapshot;
use crate::protocol::TokenUsage;
use crate::protocol::TokenUsageInfo;

/// Persistent, session-scoped state previously stored directly on `Session`.
#[derive(Default)]
pub(crate) struct SessionState {
    pub(crate) approved_commands: HashSet<Vec<String>>,
    pub(crate) history: ConversationHistory,
    pub(crate) token_info: Option<TokenUsageInfo>,
    pub(crate) latest_rate_limits: Option<RateLimitSnapshot>,
    pub(crate) interactive_execs: HashMap<String, InteractiveExecHandle>,
}

impl SessionState {
    /// Create a new session state mirroring previous `State::default()` semantics.
    pub(crate) fn new() -> Self {
        Self {
            history: ConversationHistory::new(),
            interactive_execs: HashMap::new(),
            ..Default::default()
        }
    }

    // History helpers
    pub(crate) fn record_items<I>(&mut self, items: I)
    where
        I: IntoIterator,
        I::Item: std::ops::Deref<Target = ResponseItem>,
    {
        self.history.record_items(items)
    }

    pub(crate) fn history_snapshot(&self) -> Vec<ResponseItem> {
        self.history.contents()
    }

    pub(crate) fn replace_history(&mut self, items: Vec<ResponseItem>) {
        self.history.replace(items);
    }

    // Approved command helpers
    pub(crate) fn add_approved_command(&mut self, cmd: Vec<String>) {
        self.approved_commands.insert(cmd);
    }

    pub(crate) fn approved_commands_ref(&self) -> &HashSet<Vec<String>> {
        &self.approved_commands
    }

    // Token/rate limit helpers
    pub(crate) fn update_token_info_from_usage(
        &mut self,
        usage: &TokenUsage,
        model_context_window: Option<u64>,
    ) {
        self.token_info = TokenUsageInfo::new_or_append(
            &self.token_info,
            &Some(usage.clone()),
            model_context_window,
        );
    }

    pub(crate) fn set_rate_limits(&mut self, snapshot: RateLimitSnapshot) {
        self.latest_rate_limits = Some(snapshot);
    }

    pub(crate) fn token_info_and_rate_limits(
        &self,
    ) -> (Option<TokenUsageInfo>, Option<RateLimitSnapshot>) {
        (self.token_info.clone(), self.latest_rate_limits.clone())
    }

    #[allow(dead_code)]
    pub(crate) fn insert_interactive_exec(
        &mut self,
        call_id: String,
        handle: InteractiveExecHandle,
    ) {
        self.interactive_execs.insert(call_id, handle);
    }

    #[allow(dead_code)]
    pub(crate) fn remove_interactive_exec(
        &mut self,
        call_id: &str,
    ) -> Option<InteractiveExecHandle> {
        self.interactive_execs.remove(call_id)
    }

    pub(crate) fn interactive_exec_handle(&self, call_id: &str) -> Option<InteractiveExecHandle> {
        self.interactive_execs.get(call_id).cloned()
    }

    pub(crate) fn drain_interactive_execs(&mut self) -> Vec<InteractiveExecHandle> {
        self.interactive_execs
            .drain()
            .map(|(_, handle)| handle)
            .collect()
    }
}

#[derive(Clone)]
pub(crate) struct InteractiveExecHandle {
    pub(crate) writer: mpsc::Sender<Vec<u8>>,
    killer: Arc<AsyncMutex<Option<Box<dyn ChildKiller + Send + Sync>>>>,
    #[allow(dead_code)]
    pub(crate) session_id: String,
}

impl InteractiveExecHandle {
    #[allow(dead_code)]
    pub(crate) fn new(
        writer: mpsc::Sender<Vec<u8>>,
        killer: Arc<AsyncMutex<Option<Box<dyn ChildKiller + Send + Sync>>>>,
        session_id: String,
    ) -> Self {
        Self {
            writer,
            killer,
            session_id,
        }
    }

    pub(crate) async fn kill(&self) {
        if let Some(mut killer) = self.killer.lock().await.take() {
            let _ = killer.kill();
        }
    }
}
