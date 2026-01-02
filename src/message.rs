/// Represents who sent a message in the chat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    /// Returns the display prefix for this role.
    pub fn prefix(&self) -> &'static str {
        match self {
            Role::User => "You: ",
            Role::Assistant => "Assistant: ",
        }
    }
}

/// Represents the type/purpose of a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MessageType {
    /// Normal chat message
    #[default]
    Chat,
    /// System banner (welcome message, not sent to LLM)
    SystemBanner,
}

/// A single message in the chat history.
#[derive(Clone, Debug)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub message_type: MessageType,
}

impl Message {
    /// Create a new message with the given role and content.
    pub fn new(role: Role, content: String) -> Self {
        Self {
            role,
            content,
            message_type: MessageType::Chat,
        }
    }

    /// Create a new user message.
    pub fn user(content: String) -> Self {
        Self::new(Role::User, content)
    }

    /// Create a new assistant message.
    pub fn assistant(content: String) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// Create a system banner message (not sent to LLM).
    pub fn system_banner(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
            message_type: MessageType::SystemBanner,
        }
    }

    /// Returns true if this is a system banner.
    pub fn is_system_banner(&self) -> bool {
        self.message_type == MessageType::SystemBanner
    }
}
