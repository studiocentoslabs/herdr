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

#[cfg(test)]
mod tests {
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
