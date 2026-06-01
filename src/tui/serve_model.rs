//! Minimal serve-screen model for `sunscreen chain serve`.

/// Minimum terminal width supported by the serve UI.
pub const MIN_WIDTH: u16 = 80;

/// Minimum terminal height supported by the serve UI.
pub const MIN_HEIGHT: u16 = 24;

/// One named panel in the serve screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServePanel {
    /// Stable panel label.
    pub title: &'static str,
    /// Current human-readable status.
    pub status: String,
}

/// Serve UI model with the five panels promised by the Phase 3 roadmap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeModel {
    /// Validator/runtime panel.
    pub validator: ServePanel,
    /// Build pipeline panel.
    pub build: ServePanel,
    /// Faucet panel.
    pub faucet: ServePanel,
    /// Frontend panel.
    pub frontend: ServePanel,
    /// Logs panel.
    pub logs: ServePanel,
}

impl ServeModel {
    /// Create the default Phase 3 serve model.
    #[must_use]
    pub fn new(runtime: impl Into<String>, frontend_enabled: bool) -> Self {
        let frontend_status = if frontend_enabled {
            "watching reload sentinel"
        } else {
            "disabled"
        };
        Self {
            validator: ServePanel {
                title: "validator",
                status: runtime.into(),
            },
            build: ServePanel {
                title: "build",
                status: "watching workspace".into(),
            },
            faucet: ServePanel {
                title: "faucet",
                status: "local airdrop available".into(),
            },
            frontend: ServePanel {
                title: "frontend",
                status: frontend_status.into(),
            },
            logs: ServePanel {
                title: "logs",
                status: "streaming".into(),
            },
        }
    }

    /// Whether a terminal size can show the model without dropping panels.
    #[must_use]
    pub fn fits(width: u16, height: u16) -> bool {
        width >= MIN_WIDTH && height >= MIN_HEIGHT
    }

    /// Stable panel order used by text and future ratatui renderers.
    #[must_use]
    pub fn panels(&self) -> [&ServePanel; 5] {
        [
            &self.validator,
            &self.build,
            &self.faucet,
            &self.frontend,
            &self.logs,
        ]
    }

    /// Render a compact text representation for non-headless terminals.
    #[must_use]
    pub fn render_text(&self) -> String {
        self.panels()
            .iter()
            .map(|panel| format!("{}: {}", panel.title, panel.status))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_model_has_phase3_panels_in_stable_order() {
        let model = ServeModel::new("surfpool", true);
        let titles: Vec<_> = model.panels().iter().map(|panel| panel.title).collect();

        assert_eq!(titles, ["validator", "build", "faucet", "frontend", "logs"]);
        assert!(ServeModel::fits(80, 24));
        assert!(!ServeModel::fits(79, 24));
        assert!(model.render_text().contains("validator: surfpool"));
    }

    #[test]
    fn serve_model_marks_frontend_disabled() {
        let model = ServeModel::new("test-validator", false);

        assert_eq!(model.frontend.status, "disabled");
    }
}
