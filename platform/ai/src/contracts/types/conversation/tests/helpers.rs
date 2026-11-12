use crate::conversation::message::Message;
use crate::conversation::{debug_conversation_fix, fix_conversation, Conversation};

#[macro_export]
macro_rules! assert_has_issues_unordered {
    ($fixed:expr, $issues:expr, $($expected:expr),+ $(,)?) => {
        {
            let mut expected: Vec<&str> = vec![$($expected),+];
            let mut actual: Vec<&str> = $issues.iter().map(|s| s.as_str()).collect();
            expected.sort();
            actual.sort();

            if actual != expected {
                panic!(
                    "assertion failed: issues don't match\nexpected: {:?}\n  actual: {:?}. Fixed conversation is:\n{:#?}",
                    expected, $issues, $fixed,
                );
            }
        }
    };
}

pub(crate) fn run_verify(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
    let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages.clone()));

    let (_fixed, issues_with_fixed) = fix_conversation(fixed.clone());
    assert_eq!(
        issues_with_fixed.len(),
        0,
        "Fixed conversation should have no issues, but found: {:?}\n\n{}",
        issues_with_fixed,
        debug_conversation_fix(&messages, fixed.messages(), &issues)
    );
    (fixed.messages().clone(), issues)
}
