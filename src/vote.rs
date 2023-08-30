use serde::{de::Deserializer, Deserialize};

pub const CHOICE_SEPARATOR: char = '\x1F';

#[derive(Debug, Deserialize)]
pub struct Vote {
    title: String,
    choices: Choices,
    #[serde(rename(deserialize = "max-choices"))]
    max_choices: usize,
    #[serde(default)]
    anonymous: bool,
    #[serde(deserialize_with = "deserialize_password")]
    password: Option<String>,
}

impl Vote {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn choices(&self) -> &Choices {
        &self.choices
    }
}

#[derive(Debug, Deserialize)]
pub struct Choices(String);

impl Choices {
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.split(CHOICE_SEPARATOR)
    }

    pub fn collect(&self) -> Vec<&str> {
        self.iter().collect()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl AsRef<[u8]> for Choices {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

fn deserialize_password<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct V;
    impl<'de> serde::de::Visitor<'de> for V {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok((!v.is_empty()).then(|| v.to_string()))
        }
    }

    deserializer.deserialize_str(V)
}

fn deserialize_choices<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct V;
    impl<'de> serde::de::Visitor<'de> for V {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "a sequence of strings separated by \"{CHOICE_SEPARATOR:?}\""
            )
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.split(CHOICE_SEPARATOR)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect())
        }
    }

    deserializer.deserialize_str(V)
}
