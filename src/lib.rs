//! SigmaOS-apt-utils — apt preferences/pinning rule parser.
//!
//! Inspired by Debian's `apt-utils` family and `/etc/apt/preferences`:
//! parses pinning stanzas (`Package`, `Pin`, `Pin-Priority`) that control
//! which candidate version apt selects — the mechanism half of version
//! selection policy. Stanzas are deb822-like, blank-line separated.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Errors raised by this component.
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    /// A stanza was missing a required field.
    #[error("stanza missing field: {0}")]
    MissingField(&'static str),
}

/// Result alias for component operations.
pub type ComponentResult<T> = Result<T, ComponentError>;

/// One pinning rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    /// Package glob this rule applies to (e.g. `*`, `bash`, `linux-image-*`).
    pub package: String,
    /// Pin expression (e.g. `release a=stable`, `version 1.2*`, `origin X`).
    pub pin: String,
    /// Priority integer (higher wins; negative forbids).
    pub priority: i32,
}

/// Parsed preferences file: ordered list of pin rules.
#[derive(Debug, Default)]
pub struct Preferences {
    rules: Vec<Pin>,
}

impl Preferences {
    /// Empty preferences.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse `/etc/apt/preferences` text.
    pub fn parse(input: &str) -> ComponentResult<Self> {
        let mut prefs = Preferences::new();
        let mut pkg = String::new();
        let mut pin = String::new();
        let mut prio: Option<i32> = None;
        let mut have = false;

        macro_rules! flush {
            () => {
                if have {
                    if pkg.is_empty() { return Err(ComponentError::MissingField("Package")); }
                    if pin.is_empty() { return Err(ComponentError::MissingField("Pin")); }
                    let p = prio.unwrap_or(500);
                    prefs.rules.push(Pin { package: pkg.clone(), pin: pin.clone(), priority: p });
                }
                pkg.clear(); pin.clear(); prio = None; have = false;
            };
        }

        for raw in input.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') { flush!(); continue; }
            let Some((k, v)) = line.split_once(':') else { continue };
            match k.trim() {
                "Package" => { pkg = v.trim().to_string(); have = true; }
                "Pin" => { pin = v.trim().to_string(); }
                "Pin-Priority" => { prio = v.trim().parse().ok(); }
                _ => {}
            }
        }
        flush!();
        Ok(prefs)
    }

    /// All rules in file order.
    pub fn rules(&self) -> &[Pin] {
        &self.rules
    }

    /// Whether `package` name matches a rule's `Package` glob.
    /// Supports `*` wildcard and exact names.
    pub fn matches(rule_pkg: &str, name: &str) -> bool {
        if rule_pkg == "*" { return true; }
        if !rule_pkg.contains('*') { return rule_pkg == name; }
        // Simple glob: split on '*', ensure ordered substring match.
        let parts: Vec<&str> = rule_pkg.split('*').collect();
        let mut rest = name;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() { continue; }
            if i == 0 && !rule_pkg.starts_with('*') {
                if !rest.starts_with(part) { return false; }
                rest = &rest[part.len()..];
            } else if i == parts.len() - 1 && !rule_pkg.ends_with('*') {
                return rest.ends_with(part);
            } else {
                match rest.find(part) {
                    Some(pos) => rest = &rest[pos + part.len()..],
                    None => return false,
                }
            }
        }
        true
    }

    /// Rules applicable to `package`, in file order.
    pub fn for_package(&self, package: &str) -> Vec<&Pin> {
        self.rules.iter().filter(|r| Self::matches(&r.package, package)).collect()
    }

    /// The highest-priority rule for `package`, if any.
    pub fn best_for(&self, package: &str) -> Option<&Pin> {
        self.for_package(package).into_iter().max_by_key(|r| r.priority)
    }

    /// Number of rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREFS: &str = "Package: *\nPin: release a=stable\nPin-Priority: 500\n\nPackage: bash\nPin: version 5.2*\nPin-Priority: 990\n\nPackage: linux-image-*\nPin: release n=sid\nPin-Priority: -1\n";

    #[test]
    fn parses_pin_stanzas() {
        let p = Preferences::parse(PREFS).unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p.rules()[0].package, "*");
        assert_eq!(p.rules()[0].pin, "release a=stable");
        assert_eq!(p.rules()[0].priority, 500);
        assert_eq!(p.rules()[1].priority, 990);
    }

    #[test]
    fn glob_matching() {
        assert!(Preferences::matches("*", "anything"));
        assert!(Preferences::matches("bash", "bash"));
        assert!(!Preferences::matches("bash", "zsh"));
        assert!(Preferences::matches("linux-image-*", "linux-image-6.1"));
        assert!(!Preferences::matches("linux-image-*", "kernel"));
    }

    #[test]
    fn for_package_and_best() {
        let p = Preferences::parse(PREFS).unwrap();
        let bash = p.for_package("bash");
        assert_eq!(bash.len(), 2); // * and bash
        let best = p.best_for("bash").unwrap();
        assert_eq!(best.priority, 990); // version pin beats release pin
    }

    #[test]
    fn comments_and_blanks() {
        let p = Preferences::parse("# comment\n\nPackage: x\nPin: p\nPin-Priority: 100\n").unwrap();
        assert_eq!(p.len(), 1);
    }

    #[test]
    fn empty_ok() {
        assert!(Preferences::parse("").unwrap().is_empty());
    }
}
