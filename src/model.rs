use {
    std::fmt,
    serde::{
        Deserialize,
        Serialize,
    },
    serenity::model::prelude::*,
    url::Url,
};

#[derive(Debug, Deserialize, Clone, Copy)]
pub(crate) struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl From<Color> for bitbar::attr::Color {
    fn from(color: Color) -> Self {
        css_color_parser::Color {
            r: color.red,
            g: color.green,
            b: color.blue,
            a: 1.0,
        }.into()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct DiscordData {
    nick: Option<String>,
    snowflake: UserId,
    username: String,
}

impl DiscordData {
    pub(crate) fn name(&self) -> &str {
        self.nick.as_ref().unwrap_or(&self.username)
    }

    pub(crate) fn url(&self) -> Url {
        format!("https://discordapp.com/users/{}/", self.snowflake).parse().expect("failed to parse Discord user URL")
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Person {
    pub(crate) discord: Option<DiscordData>,
    pub(crate) fav_color: Option<Color>,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub(crate) enum Uid {
    Snowflake(UserId),
    WmbId(String),
}

impl<T: Clone + Into<Uid>> From<&T> for Uid {
    fn from(r: &T) -> Uid {
        r.clone().into()
    }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uid::Snowflake(snowflake) => snowflake.fmt(f),
            Uid::WmbId(wmb_id) => wmb_id.fmt(f),
        }
    }
}
