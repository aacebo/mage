#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct RoutingKey(String);

impl std::str::FromStr for RoutingKey {
    type Err = mage_error::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_routing_key(value)?;
        Ok(Self(value.to_string()))
    }
}

impl std::fmt::Display for RoutingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for RoutingKey {
    type Error = mage_error::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<RoutingKey> for String {
    fn from(value: RoutingKey) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct BindingKey(String);

impl std::str::FromStr for BindingKey {
    type Err = mage_error::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_binding_key(value)?;
        Ok(Self(value.to_string()))
    }
}

impl std::fmt::Display for BindingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for BindingKey {
    type Error = mage_error::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<BindingKey> for String {
    fn from(value: BindingKey) -> Self {
        value.0
    }
}

fn validate_routing_key(value: &str) -> mage_error::Result<()> {
    let segments = value.split('.').collect::<Vec<_>>();

    if segments.len() != 2 || segments.iter().any(|segment| !is_literal_segment(segment)) {
        return Err(mage_error::parse(format!("invalid amqp routing key {value}")));
    }

    Ok(())
}

fn validate_binding_key(value: &str) -> mage_error::Result<()> {
    let segments = value.split('.').collect::<Vec<_>>();

    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| !matches!(*segment, "*" | "#") && !is_literal_segment(segment))
    {
        return Err(mage_error::parse(format!("invalid amqp binding key {value}")));
    }

    Ok(())
}

fn is_literal_segment(value: &str) -> bool {
    !value.is_empty() && !value.contains(['*', '#'])
}
