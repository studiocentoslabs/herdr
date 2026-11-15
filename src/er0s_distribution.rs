//! er0s distribution of herdr (see ER0S.md). Every switch here is off unless the binary was
//! built with `ER0S_DISTRIBUTION` set, so an ordinary build keeps upstream behavior and tests.

/// True in a build made with `ER0S_DISTRIBUTION` set (packaging/er0s/installa.sh).
pub(crate) const ACTIVE: bool = option_env!("ER0S_DISTRIBUTION").is_some();

const APP_DIR_NAME: &str = "er0s-terminal";
const APP_DIR_NAME_DEV: &str = "er0s-terminal-dev";

/// Directory name for config, state and sockets: distinct from `herdr` so both can be installed.
pub(crate) fn app_dir_name(debug: bool) -> &'static str {
    if debug {
        APP_DIR_NAME_DEV
    } else {
        APP_DIR_NAME
    }
}

/// Why `update` is refused: an upstream binary would replace this distribution.
pub(crate) const UPDATE_REFUSED: &str =
    "er0s-terminal does not update from the herdr channel; install a new er0s build (packaging/er0s/riallinea-fork.sh)";

#[cfg(test)]
mod tests {
    #[test]
    fn er0s_distribution_presents_its_name_and_engine() {
        if !super::ACTIVE {
            return;
        }
        let config = crate::config::Config::default();
        assert!(config.ui.window_title.starts_with("er0s"));
        assert_eq!(crate::ui::ONBOARDING_TITLE.trim(), "er0s");
        assert_eq!(
            crate::ui::ONBOARDING_SUBTITLE.trim(),
            "motore: herdr (Apache-2.0)"
        );
        let mut title = Vec::new();
        crate::terminal_effects::write_window_title(&mut title, None).unwrap();
        assert_eq!(title, b"\x1b]0;er0s\x07");
    }

    #[test]
    fn app_dir_never_collides_with_herdr() {
        for debug in [false, true] {
            let name = super::app_dir_name(debug);
            assert!(name.starts_with("er0s-terminal"));
            assert_ne!(name, "herdr");
            assert_ne!(name, "herdr-dev");
        }
    }
}
