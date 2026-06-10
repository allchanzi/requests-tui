use std::collections::{HashMap, HashSet};

use super::model::{Collection, Environment};

/// A reference to a `{{var}}` that could not be resolved (absent or empty value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingVar {
    pub name: String,
    pub secret: bool,
}

/// Resolved variable values plus the set of names marked secret, used to expand
/// `{{var}}` placeholders and report which ones are still missing.
#[derive(Debug, Clone, Default)]
pub struct VarContext {
    values: HashMap<String, String>,
    secrets: HashSet<String>,
}

impl VarContext {
    /// Build the context: collection vars are the base, the selected environment
    /// overrides them, and session-entered overrides win over everything.
    pub fn build(
        collection: &Collection,
        environment: Option<&Environment>,
        overrides: &HashMap<String, String>,
    ) -> Self {
        let mut values = HashMap::new();
        let mut secrets = HashSet::new();

        // `.env` values live in their own `process.env.*` namespace (Bruno-compatible)
        // and are always treated as secret so they stay masked in the missing-var prompt.
        for (name, value) in &collection.process_env {
            let key = format!("process.env.{name}");
            secrets.insert(key.clone());
            values.insert(key, value.clone());
        }

        for entry in &collection.vars {
            if entry.enabled {
                values.insert(entry.name.clone(), entry.value.clone());
            }
        }
        if let Some(environment) = environment {
            for entry in &environment.vars {
                if entry.enabled {
                    values.insert(entry.name.clone(), entry.value.clone());
                }
            }
            for secret in &environment.secret_vars {
                secrets.insert(secret.clone());
            }
        }
        for (name, value) in overrides {
            values.insert(name.clone(), value.clone());
        }

        Self { values, secrets }
    }

    /// Expand `{{var}}` placeholders in `input`, appending any unresolved references to
    /// `missing` (deduplicated by the caller).
    pub fn expand(&self, input: &str, missing: &mut Vec<MissingVar>) -> String {
        let mut output = String::with_capacity(input.len());
        let bytes = input.as_bytes();
        let mut index = 0;

        while index < bytes.len() {
            if input[index..].starts_with("{{") {
                if let Some(end) = input[index + 2..].find("}}") {
                    let name = input[index + 2..index + 2 + end].trim();
                    match self.values.get(name) {
                        Some(value) if !value.is_empty() => output.push_str(value),
                        _ => {
                            push_missing(
                                missing,
                                MissingVar {
                                    name: name.to_string(),
                                    secret: self.secrets.contains(name)
                                        || name.starts_with("process.env."),
                                },
                            );
                            // Leave the placeholder in place so the user can see it.
                            output.push_str(&input[index..index + 2 + end + 2]);
                        }
                    }
                    index += 2 + end + 2;
                    continue;
                }
            }
            let ch = input[index..].chars().next().unwrap();
            output.push(ch);
            index += ch.len_utf8();
        }

        output
    }
}

fn push_missing(missing: &mut Vec<MissingVar>, var: MissingVar) {
    if !missing.iter().any(|existing| existing.name == var.name) {
        missing.push(var);
    }
}
