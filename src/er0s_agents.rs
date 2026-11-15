//! er0s role preferences, owned by the TUI, read by the plugin at pane launch (ER0S.md).
use std::path::{Path, PathBuf};

const SECTIONS: [(&str, &str); 5] = [
    ("ruoli", "ceo"),
    ("ruoli", "cfo"),
    ("ruoli", "cbo"),
    ("ruoli", "clo"),
    ("esecutori", "codice"),
];
pub(crate) const LABELS: [&str; 5] = ["CEO", "CFO", "CBO", "CLO", "Esecutore di codice"];
pub(crate) const CLIS: [&str; 4] = ["claude", "codex", "opencode", "agy"];

// Small, explicit catalog. No CLI subprocess or network work in the settings UI.
pub(crate) fn models(cli: &str) -> &'static [&'static str] {
    match cli {
        "claude" => &["opus", "sonnet", "haiku"],
        "codex" => &["default", "gpt-6.1-sol", "gpt-6-astra", "gpt-6.1-terra"],
        "opencode" => &[
            "default",
            "anthropic/claude-opus-4-6",
            "anthropic/claude-sonnet-4-6",
            "opencode/big-pickle",
        ],
        "agy" => &[
            "default",
            "gemini-3.8-flash-high",
            "gemini-3.8-flash-medium",
            "gemini-3.8-flash-low",
            "claude-sonnet-4-6",
            "claude-opus-4-6-thinking",
        ],
        _ => &[],
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Agent {
    pub(crate) cli: String,
    pub(crate) model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Agents(pub(crate) [Agent; 5]);

const DEFAULTS: [(&str, &str); 5] = [
    ("claude", "opus"),
    ("claude", "sonnet"),
    ("claude", "sonnet"),
    ("claude", "sonnet"),
    ("codex", "default"),
];

impl Default for Agents {
    fn default() -> Self {
        Self(std::array::from_fn(|index| Agent {
            cli: DEFAULTS[index].0.into(),
            model: DEFAULTS[index].1.into(),
        }))
    }
}

pub(crate) fn path() -> PathBuf {
    // Same override as the installed role launcher, including isolated TUI profiles.
    std::env::var_os("ER0S_TUI_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config::config_dir().join("er0s.toml"))
}

fn document(path: &Path) -> Result<toml::Table, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(toml::Table::new()),
        Err(_) => return Err("er0s.toml: impossibile leggere il file delle impostazioni".into()),
    };
    // Do not echo raw parse errors: the file may also hold private plugin settings.
    toml::from_str(&content)
        .map_err(|_| "er0s.toml: formato TOML non valido; correggi il file".into())
}

impl Agents {
    fn from_document(doc: &toml::Table) -> Result<Self, String> {
        let mut result = Self::default();
        for (section, roles) in [
            ("ruoli", &["ceo", "cfo", "cbo", "clo"][..]),
            ("esecutori", &["codice"][..]),
        ] {
            if let Some(value) = doc.get(section) {
                let table = value
                    .as_table()
                    .ok_or_else(|| format!("er0s.toml: {section} deve essere una tabella"))?;
                if table.keys().any(|role| !roles.contains(&role.as_str())) {
                    return Err(format!("er0s.toml: ruolo non valido in {section}"));
                }
            }
        }
        for (index, (section, role)) in SECTIONS.iter().enumerate() {
            let Some(value) = doc.get(*section).and_then(|v| v.get(*role)) else {
                continue;
            };
            let table = value
                .as_table()
                .ok_or_else(|| format!("er0s.toml: {section}.{role} deve essere una tabella"))?;
            if table.keys().any(|key| key != "cli" && key != "modello") {
                return Err(format!(
                    "er0s.toml: {section}.{role} ammette solo cli e modello"
                ));
            }
            let string = |key| {
                table.get(key).and_then(toml::Value::as_str).ok_or_else(|| {
                    format!("er0s.toml: {section}.{role}.{key} deve essere una stringa")
                })
            };
            let cli = string("cli")?;
            let model = string("modello")?;
            Self::validate(index, cli, model)?;
            result.0[index] = Agent {
                cli: cli.into(),
                model: model.into(),
            };
        }
        Ok(result)
    }

    fn validate(index: usize, cli: &str, model: &str) -> Result<(), String> {
        let (section, role) = SECTIONS.get(index).ok_or("er0s.toml: ruolo non valido")?;
        if !CLIS.contains(&cli) {
            return Err(format!(
                "er0s.toml: {section}.{role}.cli non valida; scegli claude, codex, opencode o agy"
            ));
        }
        if !models(cli).contains(&model) {
            return Err(format!(
                "er0s.toml: {section}.{role}.modello non valido per {cli}; correggi modello nel file"
            ));
        }
        Ok(())
    }

    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        Self::from_document(&document(path)?)
    }

    pub(crate) fn save_role(
        path: &Path,
        index: usize,
        cli: &str,
        model: &str,
    ) -> Result<Self, String> {
        Self::validate(index, cli, model)?;
        let mut doc = document(path)?;
        // Reject corrupt settings rather than overwriting them with defaults.
        Self::from_document(&doc)?;
        let (section, role) = SECTIONS[index];
        let roles = doc
            .entry(section)
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .ok_or("er0s.toml: il gruppo di ruoli deve essere una tabella")?;
        roles.insert(
            role.into(),
            toml::Value::Table(toml::Table::from_iter([
                ("cli".into(), toml::Value::String(cli.into())),
                ("modello".into(), toml::Value::String(model.into())),
            ])),
        );
        let result = Self::from_document(&doc)?;
        let content = toml::to_string_pretty(&doc)
            .map_err(|_| "er0s.toml: impossibile serializzare le impostazioni")?;
        let parent = path.parent().ok_or("er0s.toml: percorso non valido")?;
        std::fs::create_dir_all(parent).map_err(|_| "er0s.toml: impossibile creare la cartella")?;
        // Same-directory rename; a reader sees either complete version. No secret data copied.
        if std::fs::symlink_metadata(path).is_ok_and(|m| !m.is_file()) {
            return Err("er0s.toml: il percorso deve essere un file regolare".into());
        }
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
        let temp_path = parent.join(format!(
            ".er0s-{}-{}.tmp",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let mut temp = crate::platform::create_private_state_file(&temp_path)
            .map_err(|_| "er0s.toml: impossibile creare il file temporaneo")?;
        use std::io::Write;
        let written = temp
            .write_all(content.as_bytes())
            .and_then(|()| temp.sync_all());
        drop(temp);
        if written.is_err() || crate::platform::replace_file(&temp_path, path).is_err() {
            let _ = std::fs::remove_file(&temp_path);
            return Err("er0s.toml: impossibile salvare le impostazioni".into());
        }
        crate::platform::sync_parent_directory(parent)
            .map_err(|_| "er0s.toml: impossibile confermare il salvataggio della cartella")?;
        Ok(result)
    }
}

/// Presentation-only editor: ten role/field rows, then the field's choice list.
#[derive(Debug)]
pub(crate) struct Editor {
    path: PathBuf,
    pub(crate) agents: Agents,
    pub(crate) field: Option<usize>,
    pub(crate) message: String,
    pub(crate) loaded: bool,
    pub(crate) error: bool,
}

impl Editor {
    pub(crate) fn load(path: &Path) -> Self {
        match Agents::load(path) {
            Ok(agents) => Self {
                path: path.to_owned(),
                agents,
                field: None,
                message: "Al prossimo avvio del pannello".into(),
                loaded: true,
                error: false,
            },
            Err(message) => Self {
                path: path.to_owned(),
                agents: Agents::default(),
                field: None,
                message,
                loaded: false,
                error: true,
            },
        }
    }

    pub(crate) fn choices(&self) -> Vec<String> {
        if !self.loaded {
            return Vec::new();
        }
        match self.field {
            None => self
                .agents
                .0
                .iter()
                .enumerate()
                .flat_map(|(i, agent)| {
                    [
                        format!("{} · CLI: {}", LABELS[i], agent.cli),
                        format!("{} · modello: {}", LABELS[i], agent.model),
                    ]
                })
                .collect(),
            Some(field) if field.is_multiple_of(2) => CLIS.iter().map(|v| (*v).into()).collect(),
            Some(field) => models(&self.agents.0[field / 2].cli)
                .iter()
                .map(|v| (*v).into())
                .collect(),
        }
    }

    pub(crate) fn apply(&mut self, selected: usize) -> usize {
        let Some(field) = self.field else {
            if !self.loaded {
                *self = Self::load(&self.path);
                return 0;
            }
            if selected >= 10 {
                return 0;
            }
            self.field = Some(selected);
            let agent = &self.agents.0[selected / 2];
            return if selected.is_multiple_of(2) {
                CLIS.iter().position(|v| *v == agent.cli).unwrap_or(0)
            } else {
                models(&agent.cli)
                    .iter()
                    .position(|v| *v == agent.model)
                    .unwrap_or(0)
            };
        };
        let choices = self.choices();
        let Some(value) = choices.get(selected) else {
            return selected;
        };
        let agent = &self.agents.0[field / 2];
        let (cli, model) = if field.is_multiple_of(2) {
            (
                value.as_str(),
                if value == &agent.cli {
                    &agent.model
                } else if value == DEFAULTS[field / 2].0 {
                    // Returning to the role's default CLI restores its default model.
                    // CFO/CBO/CLO use sonnet, while CEO uses opus.
                    DEFAULTS[field / 2].1
                } else {
                    models(value)[0]
                },
            )
        } else {
            (agent.cli.as_str(), value.as_str())
        };
        match Agents::save_role(&self.path, field / 2, cli, model) {
            Ok(agents) => {
                self.agents = agents;
                self.field = None;
                self.message = "Salvato · al prossimo avvio del pannello".into();
                self.error = false;
                field
            }
            Err(error) => {
                // A parent-directory sync can fail after atomic replacement. Reflect disk
                // state while retaining the failure message rather than showing stale data.
                if let Ok(agents) = Agents::load(&self.path) {
                    self.agents = agents;
                }
                self.message = error;
                self.error = true;
                selected
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path() -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        std::env::temp_dir()
            .join(format!(
                "er0s-agents-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ))
            .join("er0s.toml")
    }

    #[test]
    fn missing_file_uses_role_defaults_without_writing() {
        let path = test_path();
        let agents = Agents::load(&path).unwrap();
        assert_eq!(
            agents.0[0],
            Agent {
                cli: "claude".into(),
                model: "opus".into()
            }
        );
        for role in &agents.0[1..4] {
            assert_eq!(
                role,
                &Agent {
                    cli: "claude".into(),
                    model: "sonnet".into()
                }
            );
        }
        assert_eq!(
            agents.0[4],
            Agent {
                cli: "codex".into(),
                model: "default".into()
            }
        );
        assert!(!path.exists());
    }

    #[test]
    fn keyboard_editor_saves_and_rereads_each_cli_and_model() {
        let path = test_path();
        for role in 0..5 {
            for (cli_index, cli) in CLIS.iter().enumerate() {
                let mut editor = Editor::load(&path);
                editor.apply(role * 2);
                assert_eq!(editor.choices().len(), CLIS.len());
                assert_eq!(editor.apply(cli_index), role * 2);
                assert!(editor.field.is_none());
                assert_eq!(Agents::load(&path).unwrap().0[role].cli, *cli);
                for (model_index, model) in models(cli).iter().enumerate() {
                    editor.apply(role * 2 + 1);
                    editor.apply(model_index);
                    assert_eq!(Agents::load(&path).unwrap().0[role].model, *model);
                }
            }
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn changing_back_to_role_cli_restores_its_default_model() {
        let path = test_path();
        for role in 0..5 {
            let mut editor = Editor::load(&path);
            editor.apply(role * 2);
            editor.apply(3); // agy
            editor.apply(role * 2);
            editor.apply(usize::from(role == 4)); // claude, or codex for executor
            assert_eq!(
                Agents::load(&path).unwrap().0[role],
                Agents::default().0[role]
            );
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn role_update_preserves_other_roles_and_other_plugin_settings() {
        let path = test_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "[first_run]\nenabled=true\n[ruoli.ceo]\ncli='claude'\nmodello='opus'\n",
        )
        .unwrap();
        let saved = Agents::save_role(&path, 2, "codex", "gpt-6.1-sol").unwrap();
        assert_eq!(saved.0[0].model, "opus");
        assert_eq!(saved.0[2].model, "gpt-6.1-sol");
        assert_eq!(
            document(&path).unwrap()["first_run"]["enabled"].as_bool(),
            Some(true)
        );
        // Re-read at each save preserves an intervening update to another role.
        Agents::save_role(&path, 3, "agy", "gemini-3.8-flash-high").unwrap();
        assert_eq!(Agents::load(&path).unwrap().0[2].model, "gpt-6.1-sol");
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn invalid_cli_model_and_role_are_rejected_without_writing() {
        let path = test_path();
        for (role, cli, model, message) in [
            (0, "shell", "opus", "cli non valida"),
            (1, "claude", "gpt-6.1-sol", "modello non valido"),
            (5, "codex", "default", "ruolo non valido"),
        ] {
            assert!(Agents::save_role(&path, role, cli, model)
                .unwrap_err()
                .contains(message));
            assert!(!path.exists());
        }
    }

    #[test]
    fn malformed_file_blocks_editor_and_save_without_losing_content() {
        let path = test_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        for content in [
            "invalid = [",
            "ruoli = 3",
            "[ruoli.cfo]\ncli='claude'",
            "[ruoli.ceo]\ncli='bad'\nmodello='opus'",
            "[ruoli.clo]\ncli='codex'\nmodello='opus'",
            "[ruoli.ceo]\ncli=4\nmodello='opus'",
            "[ruoli.ceoo]\ncli='claude'\nmodello='opus'",
            "[ruoli.ceo]\ncli='claude'\nmodello='opus'\nmodle='sonnet'",
        ] {
            std::fs::write(&path, content).unwrap();
            let mut editor = Editor::load(&path);
            assert!(!editor.loaded);
            assert!(editor.message.starts_with("er0s.toml:"));
            assert!(editor.choices().is_empty());
            editor.apply(0);
            assert!(Agents::save_role(&path, 2, "claude", "sonnet").is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        }
        // Correcting the file permits retry without restarting the TUI.
        std::fs::write(&path, "").unwrap();
        let mut editor = Editor::load(&path);
        editor.apply(0);
        assert!(editor.loaded);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn unreadable_file_is_not_treated_as_missing() {
        let path = test_path();
        std::fs::create_dir_all(&path).unwrap();
        assert!(Agents::load(&path)
            .unwrap_err()
            .contains("impossibile leggere"));
        assert!(Agents::save_role(&path, 0, "claude", "opus").is_err());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
