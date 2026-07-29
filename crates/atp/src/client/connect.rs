use serde_valid::Validate;

pub fn new() -> Builder {
    Builder::default()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Connect {
    pub name: String,
    pub description: String,

    #[serde(default)]
    #[validate]
    pub skills: Vec<mage_types::actors::Skill>,
}

impl Connect {
    pub fn into_signal(self) -> super::Signal {
        self.into()
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct Builder {
    _name: String,
    _description: String,
    _skills: Vec<mage_types::actors::Skill>,
}

impl Builder {
    pub fn name(mut self, value: impl std::fmt::Display) -> Self {
        self._name = value.to_string();
        self
    }

    pub fn description(mut self, value: impl std::fmt::Display) -> Self {
        self._description = value.to_string();
        self
    }

    pub fn skill(mut self, value: impl Into<mage_types::actors::Skill>) -> Self {
        self._skills.push(value.into());
        self
    }

    pub fn skills(mut self, value: impl IntoIterator<Item = impl Into<mage_types::actors::Skill>>) -> Self {
        self._skills.extend(value.into_iter().map(|v| v.into()));
        self
    }

    pub fn build(self) -> Connect {
        Connect {
            name: self._name,
            description: self._description,
            skills: self._skills,
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            _name: env!("CARGO_PKG_NAME").to_string(),
            _description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            _skills: vec![],
        }
    }
}
