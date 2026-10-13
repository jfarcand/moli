/// Selects whether layout-dependent operations use Moli's deterministic
/// compatibility geometry or construct a real one-shot layout pass.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum LayoutPolicy {
    /// Preserve the pre-layout compatibility behavior and reject operations
    /// such as renderer screenshots that require a real layout backend.
    #[default]
    Mock,
    /// Only screenshot, screencast, and PDF output build layout.
    /// Screenshots and screencasts publish layout; print projections are temporary.
    /// Geometry queries and input read the last published layout, if any.
    OnDemand,
    /// `OnDemand`, except that a geometry query or input republishes layout
    /// first when the DOM, style, viewport, or visual resources changed since
    /// the last publication, as a Chromium synchronous layout flush does.
    /// A page that is not mutated between reads still builds one layout.
    Synchronous,
}

impl LayoutPolicy {
    pub const fn uses_real_layout(self) -> bool {
        matches!(self, Self::OnDemand | Self::Synchronous)
    }

    /// Whether a geometry query or input must observe mutations made since
    /// the last published layout.
    pub const fn flushes_geometry_reads(self) -> bool {
        matches!(self, Self::Synchronous)
    }
}

/// Immutable browser layout configuration, sealed before its first Page.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutConfiguration {
    pub policy: LayoutPolicy,
    pub scrollbars_hidden: bool,
}

impl LayoutConfiguration {
    pub const fn scrollbars_hidden_for(self, emulated_hidden: bool) -> bool {
        self.scrollbars_hidden || emulated_hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_policy_defaults_to_mock() {
        assert_eq!(LayoutPolicy::default(), LayoutPolicy::Mock);
        assert!(!LayoutPolicy::default().uses_real_layout());
        assert!(LayoutPolicy::OnDemand.uses_real_layout());
        assert!(!LayoutPolicy::Mock.uses_real_layout());
        assert!(LayoutPolicy::Synchronous.uses_real_layout());
    }

    #[test]
    fn only_synchronous_layout_flushes_geometry_reads() {
        assert!(!LayoutPolicy::Mock.flushes_geometry_reads());
        assert!(!LayoutPolicy::OnDemand.flushes_geometry_reads());
        assert!(LayoutPolicy::Synchronous.flushes_geometry_reads());
    }
}
