use {
    std::fmt,
    bitbar::IntoColor,
    css_color_parser::ColorParseError,
    serde::{
        Deserialize,
        Serialize
    },
    serenity::model::prelude::*,
    url::Url
};

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Color {
    red: u8,
    green: u8,
    blue: u8
}

impl IntoColor for Color {
    fn into_color(self) -> Result<css_color_parser::Color, ColorParseError> {
        <&Color as IntoColor>::into_color(&self)
    }
}

impl IntoColor for &Color {
    fn into_color(self) -> Result<css_color_parser::Color, ColorParseError> {
        Ok(css_color_parser::Color {
            r: self.red,
            g: self.green,
            b: self.blue,
            a: 1.0
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct DiscordData {
    nick: Option<String>,
    snowflake: UserId,
    username: String
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
    pub(crate) name: Option<String>
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub(crate) enum Uid {
    Snowflake(UserId),
    WmbId(String)
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
            Uid::WmbId(wmb_id) => wmb_id.fmt(f)
        }
    }
}
