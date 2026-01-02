use scry_cli::message::{Message, MessageType, Role};

// ============================================
// Role Tests
// ============================================

#[test]
fn test_role_user_prefix() {
    assert_eq!(Role::User.prefix(), "You: ");
}

#[test]
fn test_role_assistant_prefix() {
    assert_eq!(Role::Assistant.prefix(), "Assistant: ");
}

#[test]
fn test_role_is_copy() {
    let role = Role::User;
    let role_copy = role; // Copy, not move
    assert_eq!(role, role_copy);
}

#[test]
fn test_role_equality() {
    assert_eq!(Role::User, Role::User);
    assert_eq!(Role::Assistant, Role::Assistant);
    assert_ne!(Role::User, Role::Assistant);
}

// ============================================
// MessageType Tests
// ============================================

#[test]
fn test_message_type_default_is_chat() {
    let msg_type: MessageType = Default::default();
    assert_eq!(msg_type, MessageType::Chat);
}

#[test]
fn test_message_type_equality() {
    assert_eq!(MessageType::Chat, MessageType::Chat);
    assert_eq!(MessageType::SystemBanner, MessageType::SystemBanner);
    assert_ne!(MessageType::Chat, MessageType::SystemBanner);
}

#[test]
fn test_message_type_is_copy() {
    let msg_type = MessageType::Chat;
    let msg_type_copy = msg_type; // Copy, not move
    assert_eq!(msg_type, msg_type_copy);
}

// ============================================
// Message Construction Tests
// ============================================

#[test]
fn test_message_user_constructor() {
    let msg = Message::user("Hello!".to_string());

    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.content, "Hello!");
    assert_eq!(msg.message_type, MessageType::Chat);
}

#[test]
fn test_message_assistant_constructor() {
    let msg = Message::assistant("Hi there!".to_string());

    assert_eq!(msg.role, Role::Assistant);
    assert_eq!(msg.content, "Hi there!");
    assert_eq!(msg.message_type, MessageType::Chat);
}

#[test]
fn test_message_new_constructor() {
    let msg = Message::new(Role::User, "Test message".to_string());

    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.content, "Test message");
    assert_eq!(msg.message_type, MessageType::Chat);
}

#[test]
fn test_message_system_banner_constructor() {
    let msg = Message::system_banner("Welcome!".to_string());

    assert_eq!(msg.role, Role::Assistant);
    assert_eq!(msg.content, "Welcome!");
    assert_eq!(msg.message_type, MessageType::SystemBanner);
}

// ============================================
// Message Method Tests
// ============================================

#[test]
fn test_message_is_system_banner_true() {
    let msg = Message::system_banner("Banner text".to_string());
    assert!(msg.is_system_banner());
}

#[test]
fn test_message_is_system_banner_false_for_user() {
    let msg = Message::user("User message".to_string());
    assert!(!msg.is_system_banner());
}

#[test]
fn test_message_is_system_banner_false_for_assistant() {
    let msg = Message::assistant("Assistant message".to_string());
    assert!(!msg.is_system_banner());
}

// ============================================
// Message Content Tests
// ============================================

#[test]
fn test_message_empty_content() {
    let msg = Message::user("".to_string());
    assert_eq!(msg.content, "");
}

#[test]
fn test_message_unicode_content() {
    let content = "Hello 🌴 Miami! こんにちは".to_string();
    let msg = Message::user(content.clone());
    assert_eq!(msg.content, content);
}

#[test]
fn test_message_multiline_content() {
    let content = "Line 1\nLine 2\nLine 3".to_string();
    let msg = Message::assistant(content.clone());
    assert_eq!(msg.content, content);
}

#[test]
fn test_message_clone() {
    let msg = Message::user("Original".to_string());
    let cloned = msg.clone();

    assert_eq!(cloned.role, msg.role);
    assert_eq!(cloned.content, msg.content);
    assert_eq!(cloned.message_type, msg.message_type);
}
