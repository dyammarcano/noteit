//! Host factory registry.
//!
//! Ported from `pkg/aihost/registry.go` (lensr). Go registered hosts via
//! package `init()` side effects into a global slice; Rust has no equivalent, so
//! this is an explicit owned [`HostRegistry`] the caller populates.

use super::host::Host;

/// A factory that instantiates a [`Host`].
pub type Factory = Box<dyn Fn() -> Box<dyn Host>>;

/// Ordered registry of host factories. Registration order is preserved by
/// [`HostRegistry::all`].
#[derive(Default)]
pub struct HostRegistry {
    factories: Vec<Factory>,
}

impl HostRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a host factory.
    pub fn register(&mut self, f: Factory) {
        self.factories.push(f);
    }

    /// Instantiates every registered host, in registration order.
    pub fn all(&self) -> Vec<Box<dyn Host>> {
        self.factories.iter().map(|f| f()).collect()
    }

    /// Returns the host with matching [`Host::name`] (case-sensitive).
    pub fn by_name(&self, name: &str) -> Option<Box<dyn Host>> {
        self.all().into_iter().find(|h| h.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::host::Host;
    use std::collections::BTreeMap;
    use std::io;
    use std::path::PathBuf;

    struct FakeHost {
        name: String,
    }

    impl Host for FakeHost {
        fn name(&self) -> String {
            self.name.clone()
        }
        fn install_target(&self) -> io::Result<PathBuf> {
            Ok(PathBuf::from(format!("/tmp/{}", self.name)))
        }
        fn walk(&self, _f: &mut dyn FnMut(&str, &[u8]) -> io::Result<()>) -> io::Result<()> {
            Ok(())
        }
        fn manifest_files(&self) -> io::Result<BTreeMap<String, Vec<u8>>> {
            Ok(BTreeMap::new())
        }
    }

    // Ported from registry_test.go: TestRegisterAndAll.
    #[test]
    fn register_and_all() {
        let mut reg = HostRegistry::new();
        reg.register(Box::new(|| Box::new(FakeHost { name: "h1".into() })));
        reg.register(Box::new(|| Box::new(FakeHost { name: "h2".into() })));

        let got = reg.all();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name(), "h1", "registration order preserved");
        assert_eq!(got[1].name(), "h2", "registration order preserved");
    }

    // Ported from registry_test.go: TestByName.
    #[test]
    fn by_name() {
        let mut reg = HostRegistry::new();
        reg.register(Box::new(|| {
            Box::new(FakeHost {
                name: "claude".into(),
            })
        }));
        reg.register(Box::new(|| {
            Box::new(FakeHost {
                name: "codex".into(),
            })
        }));

        let h = reg.by_name("codex").expect("ByName(codex)");
        assert_eq!(h.name(), "codex");
        assert!(reg.by_name("missing").is_none());
    }
}
