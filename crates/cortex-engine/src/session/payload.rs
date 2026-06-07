use cortex_aql::AgentId;

#[derive(Clone, Copy)]
pub(super) enum SessionCellKind {
    Context,
    TemporaryMemory,
}

impl SessionCellKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::TemporaryMemory => "temporary_memory",
        }
    }
}

pub(super) struct SessionPayload<'a> {
    pub scope: &'a str,
    pub agent_id: AgentId,
    pub session_id: &'a str,
    pub kind: SessionCellKind,
    pub memory_type: &'a str,
    pub ttl_seconds: u64,
    pub created_unix_seconds: u64,
    pub body: &'a [u8],
}

pub(super) fn session_payload(input: SessionPayload<'_>) -> Vec<u8> {
    let lines = vec![
        format!("scope={}", sanitize_line(input.scope)),
        "status=ready".to_owned(),
        "type=memory".to_owned(),
        format!("memory_type={}", sanitize_line(input.memory_type)),
        format!("ttl_seconds={}", input.ttl_seconds),
        format!("created_unix_seconds={}", input.created_unix_seconds),
        format!("source=agent:{}", input.agent_id.0),
        format!("session_id={}", sanitize_line(input.session_id)),
        format!("session_kind={}", input.kind.as_str()),
        String::new(),
    ];
    let mut payload = lines.join("\n").into_bytes();
    payload.extend_from_slice(input.body);
    payload
}

pub(super) fn parse_session_cell(payload: &[u8]) -> Option<SessionCellMetadata> {
    let text = String::from_utf8_lossy(payload);
    let mut is_memory = false;
    let mut scope = "default".to_owned();
    let mut session_id = None;
    let mut session_kind = None;
    let mut ttl_seconds = None;
    let mut created_unix_seconds = None;
    for line in text.lines() {
        if line.trim().is_empty() {
            break;
        }
        if line.trim() == "type=memory" {
            is_memory = true;
        } else if let Some(value) = line.strip_prefix("scope=") {
            scope = value.trim().to_owned();
        } else if let Some(value) = line.strip_prefix("session_id=") {
            session_id = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("session_kind=") {
            session_kind = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("ttl_seconds=") {
            ttl_seconds = value.trim().parse::<u64>().ok();
        } else if let Some(value) = line.strip_prefix("created_unix_seconds=") {
            created_unix_seconds = value.trim().parse::<u64>().ok();
        }
    }
    is_memory.then_some(SessionCellMetadata {
        scope,
        session_id: session_id?,
        _session_kind: session_kind?,
        ttl_seconds: ttl_seconds?,
        created_unix_seconds: created_unix_seconds?,
    })
}

pub(super) struct SessionCellMetadata {
    pub scope: String,
    pub session_id: String,
    _session_kind: String,
    ttl_seconds: u64,
    created_unix_seconds: u64,
}

impl SessionCellMetadata {
    pub fn is_expired(&self, now_unix_seconds: u64) -> bool {
        now_unix_seconds >= self.created_unix_seconds.saturating_add(self.ttl_seconds)
    }
}

fn sanitize_line(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}
