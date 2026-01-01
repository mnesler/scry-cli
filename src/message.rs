/// Represents who sent a message in the chat.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// A single message in the chat history.
#[derive(Clone, Debug)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    /// Create a new message with the given role and content.
    pub fn new(role: Role, content: String) -> Self {
        Self { role, content }
    }

    /// Create a new user message.
    pub fn user(content: String) -> Self {
        Self::new(Role::User, content)
    }

    /// Create a new assistant message.
    pub fn assistant(content: String) -> Self {
        Self::new(Role::Assistant, content)
    }
}
