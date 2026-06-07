use core::str::FromStr;
use zenoh_proto::keyexpr;

/// Operation types that can be subject to access control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclOperation {
    Put,
    Delete,
    Get,
    Subscribe,
    DeclareQueryable,
    DeclareLivelinessToken,
    QueryReply,
}

impl FromStr for AclOperation {
    type Err = AclError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "put" => Ok(Self::Put),
            "delete" => Ok(Self::Delete),
            "get" => Ok(Self::Get),
            "subscribe" | "sub" => Ok(Self::Subscribe),
            "declare_queryable" | "queryable" => Ok(Self::DeclareQueryable),
            "declare_liveliness_token" | "liveliness" | "token" => Ok(Self::DeclareLivelinessToken),
            "query_reply" | "reply" => Ok(Self::QueryReply),
            _ => Err(AclError::ParseError),
        }
    }
}

/// ACL rule action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclAction {
    Allow,
    Deny,
}

impl FromStr for AclAction {
    type Err = AclError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        // no_std-compatible case-insensitive comparison
        let bytes = s.as_bytes();
        let matches = |expected: &[u8]| -> bool {
            if bytes.len() != expected.len() {
                return false;
            }
            bytes
                .iter()
                .zip(expected.iter())
                .all(|(&a, &b)| a.eq_ignore_ascii_case(&b))
        };
        if matches(b"allow") || matches(b"permit") {
            Ok(Self::Allow)
        } else if matches(b"deny") || matches(b"block") || matches(b"reject") {
            Ok(Self::Deny)
        } else {
            Err(AclError::ParseError)
        }
    }
}

/// A single ACL rule: allow or deny an operation on a key expression prefix.
#[derive(Debug, Clone)]
pub struct AclRule {
    pub action: AclAction,
    pub operation: AclOperation,
    /// Key expression prefix to match against. Uses `keyexpr` prefix matching.
    pub key_expr_prefix: &'static keyexpr,
    pub description: Option<&'static str>,
}

impl AclRule {
    pub fn new(
        action: AclAction,
        operation: AclOperation,
        key_expr_prefix: &'static keyexpr,
    ) -> Self {
        Self {
            action,
            operation,
            key_expr_prefix,
            description: None,
        }
    }

    pub fn with_description(mut self, desc: &'static str) -> Self {
        self.description = Some(desc);
        self
    }

    pub fn matches(&self, operation: AclOperation, ke: &keyexpr) -> bool {
        if self.operation != operation {
            return false;
        }
        self.key_expr_prefix.intersects(ke)
    }
}

/// Access control list: an ordered list of rules evaluated in sequence.
///
/// The first matching rule determines the result. If no rule matches,
/// the default policy applies.
#[derive(Debug, Clone)]
pub struct AccessControl<const MAX_RULES: usize> {
    rules: heapless::Vec<AclRule, MAX_RULES>,
    default_deny: bool,
}

impl<const MAX_RULES: usize> Default for AccessControl<MAX_RULES> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_RULES: usize> AccessControl<MAX_RULES> {
    pub const fn new() -> Self {
        Self {
            rules: heapless::Vec::new(),
            default_deny: false,
        }
    }

    pub fn with_default_deny(mut self) -> Self {
        self.default_deny = true;
        self
    }

    pub fn add_rule(&mut self, rule: AclRule) -> Result<(), AclError> {
        self.rules.push(rule).map_err(|_| AclError::RuleCapacity)
    }

    pub fn rules(&self) -> &[AclRule] {
        &self.rules
    }

    pub fn check(&self, operation: AclOperation, ke: &keyexpr) -> bool {
        for rule in &self.rules {
            if rule.matches(operation, ke) {
                return rule.action == AclAction::Allow;
            }
        }
        !self.default_deny
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclError {
    RuleCapacity,
    Denied,
    ParseError,
}

/// Check whether an operation on a key expression is allowed by the ACL.
///
/// Returns `Ok(())` if allowed, `Err(AclError::Denied)` if denied by a rule
/// or the default policy.
pub fn check_permission<const N: usize>(
    acl: &AccessControl<N>,
    operation: AclOperation,
    ke: &keyexpr,
) -> Result<(), AclError> {
    if acl.check(operation, ke) {
        Ok(())
    } else {
        Err(AclError::Denied)
    }
}

/// Zero-cost guard that proves an ACL check has passed.
///
/// Created at the start of each session operation. When no ACL is configured
/// (`None`), the guard is never created and the check is skipped entirely.
/// When created successfully, it acts as a proof token that the operation
/// is permitted.
#[derive(Debug, PartialEq, Eq)]
pub struct AclGuard;

impl AclGuard {
    /// Perform an ACL check against the session's access control list.
    ///
    /// Returns `Ok(AclGuard)` if permitted, `Err(AclError::Denied)` if denied.
    /// When the session has no ACL configured, the check always passes.
    pub fn check<const N: usize>(
        acl: Option<&AccessControl<N>>,
        operation: AclOperation,
        ke: &keyexpr,
    ) -> Result<Self, AclError> {
        match acl {
            Some(acl) => check_permission(acl, operation, ke).map(|()| AclGuard),
            None => Ok(AclGuard),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ke(s: &'static str) -> &'static keyexpr {
        keyexpr::from_str_unchecked(s)
    }

    #[test]
    fn test_allow_all() {
        let acl = AccessControl::<4>::new();
        assert!(acl.check(AclOperation::Put, ke("a/b/c")));
        assert!(acl.check(AclOperation::Get, ke("x/y/z")));
    }

    #[test]
    fn test_default_deny() {
        let acl = AccessControl::<4>::new().with_default_deny();
        assert!(!acl.check(AclOperation::Put, ke("a/b/c")));
    }

    #[test]
    fn test_single_allow_rule() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("a/b/*"),
        ))
        .unwrap();
        assert!(acl.check(AclOperation::Put, ke("a/b/c")));
        assert!(!acl.check(AclOperation::Put, ke("d/e/f")));
        assert!(!acl.check(AclOperation::Get, ke("a/b/c")));
    }

    #[test]
    fn test_first_match_wins() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Put,
            ke("a/b/*"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("a/b/c"),
        ))
        .unwrap();
        assert!(!acl.check(AclOperation::Put, ke("a/b/c")));
    }

    #[test]
    fn test_deny_specific_allow_general() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        // Specific deny first: deny a/b/c, allow rest under a/b/
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Put,
            ke("a/b/c"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("a/b/*"),
        ))
        .unwrap();
        assert!(acl.check(AclOperation::Put, ke("a/b/d")));
        assert!(!acl.check(AclOperation::Put, ke("a/b/c")));
    }

    #[test]
    fn test_multiple_operations() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Get,
            ke("a/**"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("a/**"),
        ))
        .unwrap();
        assert!(acl.check(AclOperation::Get, ke("a/thing")));
        assert!(acl.check(AclOperation::Put, ke("a/something")));
        assert!(!acl.check(AclOperation::Put, ke("b/something")));
        assert!(!acl.check(AclOperation::Delete, ke("a/something")));
    }

    #[test]
    fn test_capacity_exceeded() {
        let mut acl = AccessControl::<2>::new();
        assert!(
            acl.add_rule(AclRule::new(AclAction::Allow, AclOperation::Put, ke("a")))
                .is_ok()
        );
        assert!(
            acl.add_rule(AclRule::new(AclAction::Allow, AclOperation::Get, ke("b")))
                .is_ok()
        );
        assert!(
            acl.add_rule(AclRule::new(
                AclAction::Allow,
                AclOperation::Delete,
                ke("c")
            ))
            .is_err()
        );
    }

    #[test]
    fn test_rule_matches() {
        let rule = AclRule::new(AclAction::Deny, AclOperation::Subscribe, ke("secret/**"));
        assert!(rule.matches(AclOperation::Subscribe, ke("secret/thing")));
        assert!(!rule.matches(AclOperation::Subscribe, ke("public/thing")));
        assert!(!rule.matches(AclOperation::Put, ke("secret/thing")));
    }

    #[test]
    fn test_wildcard_intersects() {
        let rule = AclRule::new(AclAction::Allow, AclOperation::Subscribe, ke("a/b/**"));
        assert!(rule.matches(AclOperation::Subscribe, ke("a/b/c/d")));
        assert!(rule.matches(AclOperation::Subscribe, ke("a/b/c")));
        let rule2 = AclRule::new(AclAction::Allow, AclOperation::Subscribe, ke("a/b/*"));
        assert!(rule2.matches(AclOperation::Subscribe, ke("a/b/c")));
        assert!(!rule2.matches(AclOperation::Subscribe, ke("a/x/c")));
    }

    #[test]
    fn test_liveliness_operation() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::DeclareLivelinessToken,
            ke("alive/**"),
        ))
        .unwrap();
        assert!(acl.check(AclOperation::DeclareLivelinessToken, ke("alive/device1")));
        assert!(!acl.check(AclOperation::DeclareLivelinessToken, ke("other/device")));
    }

    #[test]
    fn test_query_reply_operation() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::QueryReply,
            ke("service/**"),
        ))
        .unwrap();
        assert!(acl.check(AclOperation::QueryReply, ke("service/echo")));
        assert!(!acl.check(AclOperation::QueryReply, ke("other/echo")));
    }

    #[test]
    fn test_with_description() {
        let rule = AclRule::new(AclAction::Deny, AclOperation::Put, ke("admin/**"))
            .with_description("Block writes to admin space");
        assert_eq!(rule.description, Some("Block writes to admin space"));
        assert!(rule.matches(AclOperation::Put, ke("admin/config")));
    }

    #[test]
    fn test_rules_reflect_added() {
        let mut acl = AccessControl::<4>::new();
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Delete,
            ke("**"),
        ))
        .unwrap();
        assert_eq!(acl.rules().len(), 1);
    }

    #[test]
    fn test_parse_acl_operation_put() {
        assert_eq!("put".parse::<AclOperation>().unwrap(), AclOperation::Put);
    }

    #[test]
    fn test_parse_acl_operation_get() {
        assert_eq!("get".parse::<AclOperation>().unwrap(), AclOperation::Get);
    }

    #[test]
    fn test_parse_acl_operation_subscribe() {
        assert_eq!(
            "subscribe".parse::<AclOperation>().unwrap(),
            AclOperation::Subscribe
        );
        assert_eq!(
            "sub".parse::<AclOperation>().unwrap(),
            AclOperation::Subscribe
        );
    }

    #[test]
    fn test_parse_acl_operation_liveliness() {
        assert_eq!(
            "liveliness".parse::<AclOperation>().unwrap(),
            AclOperation::DeclareLivelinessToken
        );
        assert_eq!(
            "token".parse::<AclOperation>().unwrap(),
            AclOperation::DeclareLivelinessToken
        );
    }

    #[test]
    fn test_parse_acl_operation_reply() {
        assert_eq!(
            "reply".parse::<AclOperation>().unwrap(),
            AclOperation::QueryReply
        );
    }

    #[test]
    fn test_parse_acl_operation_invalid() {
        assert!("invalid".parse::<AclOperation>().is_err());
    }

    #[test]
    fn test_parse_acl_action_allow() {
        assert_eq!("allow".parse::<AclAction>().unwrap(), AclAction::Allow);
        assert_eq!("permit".parse::<AclAction>().unwrap(), AclAction::Allow);
    }

    #[test]
    fn test_parse_acl_action_deny() {
        assert_eq!("deny".parse::<AclAction>().unwrap(), AclAction::Deny);
        assert_eq!("block".parse::<AclAction>().unwrap(), AclAction::Deny);
        assert_eq!("reject".parse::<AclAction>().unwrap(), AclAction::Deny);
    }

    #[test]
    fn test_parse_acl_action_invalid() {
        assert!("unknown".parse::<AclAction>().is_err());
    }

    #[test]
    fn test_check_permission_allowed() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("a/b/*"),
        ))
        .unwrap();
        assert!(check_permission(&acl, AclOperation::Put, ke("a/b/c")).is_ok());
        assert!(check_permission(&acl, AclOperation::Put, ke("d/e/f")).is_err());
    }

    #[test]
    fn test_check_permission_denied_error() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Put,
            ke("admin/**"),
        ))
        .unwrap();
        let result = check_permission(&acl, AclOperation::Put, ke("admin/config"));
        assert_eq!(result, Err(AclError::Denied));
    }

    #[test]
    fn test_check_permission_default_allow() {
        let acl = AccessControl::<4>::new();
        let result = check_permission(&acl, AclOperation::Put, ke("anything"));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_check_permission_default_deny() {
        let acl = AccessControl::<4>::new().with_default_deny();
        let result = check_permission(&acl, AclOperation::Put, ke("anything"));
        assert_eq!(result, Err(AclError::Denied));
    }

    /// Integration-style test: create ACL with multiple rules and run a scenario.
    #[test]
    fn test_acl_integration_scenario() {
        let mut acl = AccessControl::<8>::new().with_default_deny();

        // Allow publish on public space
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("public/**"),
        ))
        .unwrap();
        // Allow subscribe on public space
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Subscribe,
            ke("public/**"),
        ))
        .unwrap();
        // Allow query on service space
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Get,
            ke("service/**"),
        ))
        .unwrap();
        // Deny access to admin space for all operations
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Put,
            ke("admin/**"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Get,
            ke("admin/**"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Subscribe,
            ke("admin/**"),
        ))
        .unwrap();
        // Allow liveliness tokens on alive space
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::DeclareLivelinessToken,
            ke("alive/**"),
        ))
        .unwrap();

        // --- Scenario: Publisher ---
        assert!(acl.check(AclOperation::Put, ke("public/news")));
        assert!(!acl.check(AclOperation::Put, ke("admin/config")));
        assert!(!acl.check(AclOperation::Put, ke("other/thing")));

        // --- Scenario: Subscriber ---
        assert!(acl.check(AclOperation::Subscribe, ke("public/news")));
        assert!(!acl.check(AclOperation::Subscribe, ke("admin/config")));
        assert!(!acl.check(AclOperation::Subscribe, ke("other/thing")));

        // --- Scenario: Querier ---
        assert!(acl.check(AclOperation::Get, ke("service/echo")));
        assert!(!acl.check(AclOperation::Get, ke("admin/config")));
        assert!(!acl.check(AclOperation::Get, ke("other/thing")));

        // --- Scenario: Liveliness ---
        assert!(acl.check(AclOperation::DeclareLivelinessToken, ke("alive/device1")));
        assert!(!acl.check(AclOperation::DeclareLivelinessToken, ke("other/device")));

        // --- Scenario: Query reply ---
        assert!(!acl.check(AclOperation::QueryReply, ke("service/echo")));
    }

    #[test]
    fn test_acl_first_deny_then_allow_same_prefix() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Put,
            ke("data/**"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("data/public/**"),
        ))
        .unwrap();
        // First-match-wins: deny matches first
        assert!(!acl.check(AclOperation::Put, ke("data/public/foo")));
        assert!(!acl.check(AclOperation::Put, ke("data/private")));
    }

    #[test]
    fn test_acl_guard_no_acl_allows_all() {
        let result = AclGuard::check(None::<&AccessControl<4>>, AclOperation::Put, ke("a/b/c"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_acl_guard_denies_blocked_operation() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("allowed/**"),
        ))
        .unwrap();
        let result = AclGuard::check(Some(&acl), AclOperation::Put, ke("blocked/thing"));
        assert_eq!(result, Err(AclError::Denied));
    }

    #[test]
    fn test_acl_guard_allows_permitted_operation() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("allowed/**"),
        ))
        .unwrap();
        let result = AclGuard::check(Some(&acl), AclOperation::Put, ke("allowed/thing"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_acl_guard_all_operations_passthrough_no_acl() {
        for op in &[
            AclOperation::Put,
            AclOperation::Delete,
            AclOperation::Get,
            AclOperation::Subscribe,
            AclOperation::DeclareQueryable,
            AclOperation::DeclareLivelinessToken,
            AclOperation::QueryReply,
        ] {
            let result = AclGuard::check(None::<&AccessControl<4>>, *op, ke("any/key"));
            assert!(result.is_ok(), "Operation {:?} should pass without ACL", op);
        }
    }

    #[test]
    fn test_acl_guard_default_deny_rejects_unknown() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("put/**"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Get,
            ke("get/**"),
        ))
        .unwrap();
        // Subscribe is not allowed -> default deny kicks in
        let result = AclGuard::check(Some(&acl), AclOperation::Subscribe, ke("get/thing"));
        assert_eq!(result, Err(AclError::Denied));
    }

    #[test]
    fn test_acl_guard_multiple_operations() {
        let mut acl = AccessControl::<8>::new().with_default_deny();
        // Deny first (takes precedence for matching keys)
        acl.add_rule(AclRule::new(
            AclAction::Deny,
            AclOperation::Put,
            ke("pub/admin/**"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Put,
            ke("pub/**"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Subscribe,
            ke("sub/**"),
        ))
        .unwrap();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Get,
            ke("query/**"),
        ))
        .unwrap();

        // Allowed
        assert!(AclGuard::check(Some(&acl), AclOperation::Put, ke("pub/hello")).is_ok());
        assert!(AclGuard::check(Some(&acl), AclOperation::Subscribe, ke("sub/world")).is_ok());
        assert!(AclGuard::check(Some(&acl), AclOperation::Get, ke("query/echo")).is_ok());

        // Denied: specific deny takes precedence (first-match-wins)
        assert_eq!(
            AclGuard::check(Some(&acl), AclOperation::Put, ke("pub/admin/config")),
            Err(AclError::Denied)
        );

        // Denied: operation not allowed for this key prefix
        assert_eq!(
            AclGuard::check(Some(&acl), AclOperation::Get, ke("pub/hello")),
            Err(AclError::Denied)
        );

        // Denied: no rule matches -> default deny
        assert_eq!(
            AclGuard::check(Some(&acl), AclOperation::Delete, ke("anything")),
            Err(AclError::Denied)
        );

        // Denied: key prefix not covered
        assert_eq!(
            AclGuard::check(Some(&acl), AclOperation::Put, ke("other/thing")),
            Err(AclError::Denied)
        );
    }

    #[test]
    fn test_acl_guard_liveliness_tokens() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::DeclareLivelinessToken,
            ke("alive/**"),
        ))
        .unwrap();
        assert!(
            AclGuard::check(
                Some(&acl),
                AclOperation::DeclareLivelinessToken,
                ke("alive/device1")
            )
            .is_ok()
        );
        assert_eq!(
            AclGuard::check(
                Some(&acl),
                AclOperation::DeclareLivelinessToken,
                ke("other/device")
            ),
            Err(AclError::Denied)
        );
    }

    #[test]
    fn test_acl_guard_queryable() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::DeclareQueryable,
            ke("service/**"),
        ))
        .unwrap();
        assert!(
            AclGuard::check(
                Some(&acl),
                AclOperation::DeclareQueryable,
                ke("service/echo")
            )
            .is_ok()
        );
        assert_eq!(
            AclGuard::check(
                Some(&acl),
                AclOperation::DeclareQueryable,
                ke("other/service")
            ),
            Err(AclError::Denied)
        );
    }

    #[test]
    fn test_acl_guard_delete() {
        let mut acl = AccessControl::<4>::new().with_default_deny();
        acl.add_rule(AclRule::new(
            AclAction::Allow,
            AclOperation::Delete,
            ke("writable/**"),
        ))
        .unwrap();
        assert!(AclGuard::check(Some(&acl), AclOperation::Delete, ke("writable/data")).is_ok());
        assert_eq!(
            AclGuard::check(Some(&acl), AclOperation::Delete, ke("readonly/data")),
            Err(AclError::Denied)
        );
    }
}

#[cfg(all(test, feature = "proptest"))]
mod proptest_tests {
    extern crate std;

    use super::*;
    use proptest::prelude::*;
    use std::boxed::Box;

    fn make_ke(s: &str) -> &'static keyexpr {
        let b = s.to_string().into_boxed_str();
        keyexpr::from_str_unchecked(Box::leak(b))
    }

    fn acl_operation_strategy() -> impl Strategy<Value = AclOperation> {
        prop_oneof![
            Just(AclOperation::Put),
            Just(AclOperation::Delete),
            Just(AclOperation::Get),
            Just(AclOperation::Subscribe),
            Just(AclOperation::DeclareQueryable),
            Just(AclOperation::DeclareLivelinessToken),
            Just(AclOperation::QueryReply),
        ]
    }

    fn acl_action_strategy() -> impl Strategy<Value = AclAction> {
        prop_oneof![Just(AclAction::Allow), Just(AclAction::Deny),]
    }

    fn valid_keyexpr_string() -> impl Strategy<Value = String> {
        prop_oneof![
            4 => r"[a-z]{1,5}",
            4 => (r"[a-z]{1,5}", r"[a-z]{1,5}")
                .prop_map(|(a, b): (String, String)| format!("{}/{}", a, b)),
            2 => (r"[a-z]{1,5}", r"[a-z]{1,5}", r"[a-z]{1,5}")
                .prop_map(|(a, b, c): (String, String, String)| format!("{}/{}/{}", a, b, c)),
            2 => r"[a-z]{1,5}".prop_map(|a: String| format!("{}/*", a)),
            2 => r"[a-z]{1,5}".prop_map(|a: String| format!("{}/**", a)),
            1 => Just("**".to_string()),
            1 => Just("*".to_string()),
            1 => (r"[a-z]{1,5}", r"[a-z]{1,5}")
                .prop_map(|(a, b): (String, String)| format!("{}/{}/*", a, b)),
            1 => (r"[a-z]{1,5}", r"[a-z]{1,5}")
                .prop_map(|(a, b): (String, String)| format!("{}/{}/**", a, b)),
        ]
    }

    fn rule_strategy() -> impl Strategy<Value = (AclAction, AclOperation, String)> {
        (
            acl_action_strategy(),
            acl_operation_strategy(),
            valid_keyexpr_string(),
        )
    }

    fn rule_list_strategy() -> impl Strategy<Value = Vec<(AclAction, AclOperation, String)>> {
        proptest::collection::vec(rule_strategy(), 0..=8)
    }

    proptest! {
        /// Property 1: determinism — calling `check()` twice with the same arguments
        /// returns the same result for any ACL configuration and operation.
        #[test]
        fn determinism(
            rules in rule_list_strategy(),
            op in acl_operation_strategy(),
            ke_str in valid_keyexpr_string(),
        ) {
            let ke = match keyexpr::new(&ke_str) {
                Ok(k) => k,
                Err(_) => return Err(proptest::test_runner::TestCaseError::reject("invalid keyexpr")),
            };
            let mut acl = AccessControl::<8>::new();
            for (action, operation, prefix_str) in &rules {
                let prefix = make_ke(prefix_str);
                let _ = acl.add_rule(AclRule::new(*action, *operation, prefix));
            }
            let r1 = acl.check(op, ke);
            let r2 = acl.check(op, ke);
            prop_assert_eq!(r1, r2);
        }
    }

    proptest! {
        /// Property 2: rule_order_matters — first matching rule wins.
        /// When the first rule is Deny and the second is Allow for the same key
        /// expression and operation, the result is Deny.
        #[test]
        fn rule_order_matters(
            op in acl_operation_strategy(),
            ke_str in valid_keyexpr_string(),
        ) {
            let ke_prefix = make_ke(&ke_str);

            let mut acl = AccessControl::<8>::new();
            let _ = acl.add_rule(AclRule::new(AclAction::Deny, op, ke_prefix));
            let _ = acl.add_rule(AclRule::new(AclAction::Allow, op, ke_prefix));

            let ke = match keyexpr::new(&ke_str) {
                Ok(k) => k,
                Err(_) => return Err(proptest::test_runner::TestCaseError::reject("invalid keyexpr")),
            };

            prop_assert!(!acl.check(op, ke));
        }
    }

    proptest! {
        /// Property 3: no_acl_always_passes — when no ACL is configured,
        /// all operations pass (returns Ok/true).
        #[test]
        fn no_acl_always_passes(
            op in acl_operation_strategy(),
            ke_str in valid_keyexpr_string(),
        ) {
            let acl = AccessControl::<8>::new();
            let ke = match keyexpr::new(&ke_str) {
                Ok(k) => k,
                Err(_) => return Err(proptest::test_runner::TestCaseError::reject("invalid keyexpr")),
            };
            prop_assert!(acl.check(op, ke));
        }
    }

    proptest! {
        /// Property 4: default_deny_blocks_all — when ACL is configured with
        /// a single default-deny rule, all operations are denied.
        #[test]
        fn default_deny_blocks_all(
            op in acl_operation_strategy(),
            ke_str in valid_keyexpr_string(),
        ) {
            let acl = AccessControl::<8>::new().with_default_deny();
            let ke = match keyexpr::new(&ke_str) {
                Ok(k) => k,
                Err(_) => return Err(proptest::test_runner::TestCaseError::reject("invalid keyexpr")),
            };
            prop_assert!(!acl.check(op, ke));
        }
    }

    proptest! {
        /// Property 5: allow_override_deny — a specific Allow rule with default-deny
        /// permits matching operations while blocking non-matching ones.
        #[test]
        fn allow_override_deny(
            op in acl_operation_strategy(),
            specific_ke_str in valid_keyexpr_string(),
            other_ke_str in valid_keyexpr_string(),
        ) {
            prop_assume!(specific_ke_str != other_ke_str);

            let specific_ke_ref = keyexpr::new(&specific_ke_str)
                .map_err(|_| proptest::test_runner::TestCaseError::reject("invalid keyexpr"))?;
            let other_ke_ref = keyexpr::new(&other_ke_str)
                .map_err(|_| proptest::test_runner::TestCaseError::reject("invalid keyexpr"))?;

            if specific_ke_ref.intersects(other_ke_ref) {
                return Err(proptest::test_runner::TestCaseError::reject("specific rule matches other"));
            }

            let mut acl = AccessControl::<8>::new().with_default_deny();
            let _ = acl.add_rule(AclRule::new(AclAction::Allow, op, make_ke(&specific_ke_str)));

            prop_assert!(acl.check(op, specific_ke_ref));
            prop_assert!(!acl.check(op, other_ke_ref));
        }
    }

    proptest! {
        /// Property 6: operation_type_filtering — a rule targeting one operation
        /// type (e.g. Put) does not affect other operation types (e.g. Get) on
        /// the same key expression.
        #[test]
        fn operation_type_filtering(
            allowed_op in acl_operation_strategy(),
            blocked_op in acl_operation_strategy(),
            ke_str in valid_keyexpr_string(),
        ) {
            prop_assume!(allowed_op != blocked_op);

            let rule_ke = make_ke(&ke_str);

            let mut acl = AccessControl::<8>::new().with_default_deny();
            let _ = acl.add_rule(AclRule::new(AclAction::Allow, allowed_op, rule_ke));

            let ke = match keyexpr::new(&ke_str) {
                Ok(k) => k,
                Err(_) => return Err(proptest::test_runner::TestCaseError::reject("invalid keyexpr")),
            };

            prop_assert!(acl.check(allowed_op, ke));
            prop_assert!(!acl.check(blocked_op, ke));
        }
    }
}
