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
        save_document(path, &doc)?;
        Ok(result)
    }
}

// Shared by role and complexity preferences: a reader sees a complete document.
fn save_document(path: &Path, doc: &toml::Table) -> Result<(), String> {
    let content = toml::to_string_pretty(doc)
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
    Ok(())
}

pub(crate) const COMPLEXITY_LEVELS: [&str; 3] = ["semplice", "media", "difficile"];
const COMPLEXITY_DEFAULTS: [&str; 3] = ["haiku", "sonnet", "opus"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Complexity(pub(crate) [String; 3]);

impl Default for Complexity {
    fn default() -> Self {
        Self(COMPLEXITY_DEFAULTS.map(str::to_owned))
    }
}

impl Complexity {
    fn from_document(doc: &toml::Table) -> Result<Self, String> {
        let mut result = Self::default();
        let Some(value) = doc.get("complessita") else {
            return Ok(result);
        };
        let table = value
            .as_table()
            .ok_or("er0s.toml: complessita deve essere una tabella")?;
        if table
            .keys()
            .any(|key| !COMPLEXITY_LEVELS.contains(&key.as_str()))
        {
            return Err(
                "er0s.toml: livello non valido in complessita; scegli semplice, media o difficile"
                    .into(),
            );
        }
        for (index, level) in COMPLEXITY_LEVELS.iter().enumerate() {
            if let Some(value) = table.get(*level) {
                let model = value.as_str().ok_or_else(|| {
                    format!("er0s.toml: complessita.{level} deve essere una stringa")
                })?;
                Self::validate(index, model)?;
                result.0[index] = model.into();
            }
        }
        Ok(result)
    }

    fn validate(index: usize, model: &str) -> Result<(), String> {
        let level = COMPLEXITY_LEVELS
            .get(index)
            .ok_or("er0s.toml: livello di complessità non valido")?;
        if !models("claude").contains(&model) {
            return Err(format!(
                "er0s.toml: complessita.{level} non valido; scegli haiku, sonnet o opus"
            ));
        }
        Ok(())
    }

    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        Self::from_document(&document(path)?)
    }

    pub(crate) fn save_level(path: &Path, index: usize, model: &str) -> Result<Self, String> {
        Self::validate(index, model)?;
        let mut doc = document(path)?;
        Self::from_document(&doc)?;
        let table = doc
            .entry("complessita")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .ok_or("er0s.toml: complessita deve essere una tabella")?;
        table.insert(
            COMPLEXITY_LEVELS[index].into(),
            toml::Value::String(model.into()),
        );
        let result = Self::from_document(&doc)?;
        save_document(path, &doc)?;
        Ok(result)
    }
}

#[derive(Debug)]
enum Values {
    Roles(Agents),
    Complexity(Complexity),
}

impl Values {
    fn reload(&self, path: &Path) -> Result<Self, String> {
        match self {
            Self::Roles(_) => Agents::load(path).map(Self::Roles),
            Self::Complexity(_) => Complexity::load(path).map(Self::Complexity),
        }
    }

    fn defaults(&self) -> Self {
        match self {
            Self::Roles(_) => Self::Roles(Agents::default()),
            Self::Complexity(_) => Self::Complexity(Complexity::default()),
        }
    }
}

/// Presentation-only editor shared by role fields and complexity models.
#[derive(Debug)]
pub(crate) struct Editor {
    path: PathBuf,
    values: Values,
    pub(crate) field: Option<usize>,
    pub(crate) message: String,
    pub(crate) loaded: bool,
    pub(crate) error: bool,
}

impl Editor {
    pub(crate) fn load(path: &Path) -> Self {
        Self::load_values(path, Values::Roles(Agents::default()))
    }

    pub(crate) fn load_complexity(path: &Path) -> Self {
        Self::load_values(path, Values::Complexity(Complexity::default()))
    }

    fn load_values(path: &Path, defaults: Values) -> Self {
        match defaults.reload(path) {
            Ok(values) => {
                let message = match &values {
                    Values::Roles(_) => "Al prossimo avvio del pannello",
                    Values::Complexity(_) => "Priorità: compito > complessità > ruolo",
                };
                Self {
                    path: path.to_owned(),
                    values,
                    field: None,
                    message: message.into(),
                    loaded: true,
                    error: false,
                }
            }
            Err(message) => Self {
                path: path.to_owned(),
                values: defaults,
                field: None,
                message,
                loaded: false,
                error: true,
            },
        }
    }

    pub(crate) fn title(&self) -> String {
        match (&self.values, self.field) {
            (Values::Roles(_), None) => "er0s · agenti".into(),
            (Values::Roles(_), Some(field)) => format!(
                "{} · {}",
                LABELS[field / 2],
                if field.is_multiple_of(2) {
                    "CLI"
                } else {
                    "modello"
                }
            ),
            (Values::Complexity(_), None) => "Complessità del compito".into(),
            (Values::Complexity(_), Some(field)) => {
                format!("Complessità · {}", COMPLEXITY_LEVELS[field])
            }
        }
    }

    pub(crate) fn choices(&self) -> Vec<String> {
        if !self.loaded {
            return Vec::new();
        }
        match (&self.values, self.field) {
            (Values::Roles(agents), None) => agents
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
            (Values::Complexity(complexity), None) => complexity
                .0
                .iter()
                .enumerate()
                .map(|(i, model)| format!("{} · modello: {model}", COMPLEXITY_LEVELS[i]))
                .collect(),
            (Values::Roles(_), Some(field)) if field.is_multiple_of(2) => {
                CLIS.iter().map(|v| (*v).into()).collect()
            }
            (Values::Roles(agents), Some(field)) => models(&agents.0[field / 2].cli)
                .iter()
                .map(|v| (*v).into())
                .collect(),
            (Values::Complexity(_), Some(_)) => {
                models("claude").iter().map(|v| (*v).into()).collect()
            }
        }
    }

    pub(crate) fn apply(&mut self, selected: usize) -> usize {
        let Some(field) = self.field else {
            if !self.loaded {
                *self = Self::load_values(&self.path, self.values.defaults());
                return 0;
            }
            if selected >= self.choices().len() {
                return 0;
            }
            let index = match &self.values {
                Values::Roles(agents) => {
                    let agent = &agents.0[selected / 2];
                    if selected.is_multiple_of(2) {
                        CLIS.iter().position(|v| *v == agent.cli).unwrap_or(0)
                    } else {
                        models(&agent.cli)
                            .iter()
                            .position(|v| *v == agent.model)
                            .unwrap_or(0)
                    }
                }
                Values::Complexity(complexity) => models("claude")
                    .iter()
                    .position(|v| *v == complexity.0[selected])
                    .unwrap_or(0),
            };
            self.field = Some(selected);
            return index;
        };
        let choices = self.choices();
        let Some(value) = choices.get(selected) else {
            return selected;
        };
        let result = match &self.values {
            Values::Roles(agents) => {
                let agent = &agents.0[field / 2];
                let (cli, model) = if field.is_multiple_of(2) {
                    (
                        value.as_str(),
                        if value == &agent.cli {
                            &agent.model
                        } else if value == DEFAULTS[field / 2].0 {
                            DEFAULTS[field / 2].1
                        } else {
                            models(value)[0]
                        },
                    )
                } else {
                    (agent.cli.as_str(), value.as_str())
                };
                Agents::save_role(&self.path, field / 2, cli, model).map(Values::Roles)
            }
            Values::Complexity(_) => {
                Complexity::save_level(&self.path, field, value).map(Values::Complexity)
            }
        };
        match result {
            Ok(values) => {
                self.values = values;
                self.field = None;
                self.message = match &self.values {
                    Values::Roles(_) => "Salvato · al prossimo avvio del pannello",
                    Values::Complexity(_) => "Salvato · al prossimo compito",
                }
                .into();
                self.error = false;
                field
            }
            Err(error) => {
                // Directory sync can fail after rename: reflect disk while keeping the error.
                if let Ok(values) = self.values.reload(&self.path) {
                    self.values = values;
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
    #[test]
    fn complexity_defaults_and_partial_table_match_delegate_contract() {
        let path = test_path();
        assert_eq!(
            Complexity::load(&path).unwrap().0,
            ["haiku", "sonnet", "opus"].map(str::to_owned)
        );
        assert!(!path.exists());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[complessita]\nmedia='opus'\n").unwrap();
        assert_eq!(
            Complexity::load(&path).unwrap().0,
            ["haiku", "opus", "opus"].map(str::to_owned)
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn complexity_keyboard_saves_all_levels_and_keeps_role_and_task_overrides() {
        let path = test_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[ruoli.cfo]\nmodello='opus'\n[ruoli.coo]\nmodello='sonnet'\n[compiti.task_example]\nmodello='haiku'\n[complessita]\nmedia='opus'\n").unwrap();
        let mut unrelated_before = document(&path).unwrap();
        unrelated_before.remove("complessita");
        for level in 0..3 {
            for (index, model) in ["opus", "sonnet", "haiku"].iter().enumerate() {
                let mut editor = Editor::load_complexity(&path);
                assert!(editor.loaded);
                assert_eq!(editor.choices().len(), 3);
                editor.apply(level);
                assert_eq!(editor.choices(), ["opus", "sonnet", "haiku"]);
                assert_eq!(editor.apply(index), level);
                assert!(editor.field.is_none());
                assert_eq!(Complexity::load(&path).unwrap().0[level], *model);
                assert_eq!(
                    document(&path).unwrap()["complessita"][COMPLEXITY_LEVELS[level]].as_str(),
                    Some(*model)
                );
                let mut unrelated_after = document(&path).unwrap();
                unrelated_after.remove("complessita");
                assert_eq!(unrelated_after, unrelated_before);
            }
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn invalid_complexity_is_rejected_without_replacing_file_and_retry_keeps_section() {
        let path = test_path();
        assert!(Complexity::save_level(&path, 3, "haiku").is_err());
        assert!(Complexity::save_level(&path, 0, "gpt-6.1-sol").is_err());
        assert!(!path.exists());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        for content in [
            "bad = [",
            "complessita=3",
            "[complessita]\nsemplice=4",
            "[complessita]\nmedia='unknown'",
            "[complessita]\nsempice='haiku'",
        ] {
            std::fs::write(&path, content).unwrap();
            let mut editor = Editor::load_complexity(&path);
            assert!(!editor.loaded);
            assert!(editor.error);
            assert!(editor.choices().is_empty());
            assert_eq!(editor.title(), "Complessità del compito");
            editor.apply(0);
            assert!(!editor.loaded);
            assert!(Complexity::save_level(&path, 0, "opus").is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        }
        let mut editor = Editor::load_complexity(&path);
        std::fs::write(&path, "[complessita]\nsemplice='sonnet'\n").unwrap();
        editor.apply(0);
        assert!(editor.loaded);
        assert_eq!(editor.title(), "Complessità del compito");
        assert!(editor.choices()[0].contains("sonnet"));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
