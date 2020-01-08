#![deny(rust_2018_idioms, unused, unused_import_braces, unused_qualifications, warnings)]

use {
    std::{
        collections::HashMap,
        convert::Infallible,
        fmt,
        fs::File,
        io
    },
    bitbar::{
        ContentItem,
        Image,
        Menu,
        MenuItem
    },
    css_color_parser::ColorParseError,
    derive_more::From,
    image::{
        FilterType,
        ImageError,
        ImageFormat
    },
    mime::Mime,
    num_traits::One,
    serde::{
        Deserialize,
        Deserializer,
        Serialize,
        de::Visitor
    },
    url::Url,
    crate::{
        model::*,
        util::{
            EntryExt as _,
            ResponseExt as _,
            ResultNeverExt as _
        }
    }
};

mod model;
mod util;

#[derive(Debug, From)]
enum Error {
    ColorParse(ColorParseError),
    Header(reqwest::header::ToStrError),
    InvalidMime(Mime),
    Image(ImageError),
    Io(io::Error),
    Json(serde_json::Error),
    MimeFromStr(mime::FromStrError),
    Reqwest(reqwest::Error),
    Url(url::ParseError),
    Xdg(xdg_basedir::Error)
}

impl From<Infallible> for Error {
    fn from(never: Infallible) -> Error {
        match never {}
    }
}

#[derive(Debug)]
enum VersionLink {
    Enabled,
    Alternate,
    Disabled
}

impl Default for VersionLink {
    fn default() -> VersionLink {
        VersionLink::Enabled
    }
}

impl<'de> Deserialize<'de> for VersionLink {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<VersionLink, D::Error> {
        deserializer.deserialize_any(VersionLinkVisitor)
    }
}

struct VersionLinkVisitor;

impl<'de> Visitor<'de> for VersionLinkVisitor {
    type Value = VersionLink;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a boolean or the string \"alt\"")
    }

    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<VersionLink, E> {
        Ok(if v { VersionLink::Enabled } else { VersionLink::Disabled })
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<VersionLink, E> {
        if v == "alt" {
            Ok(VersionLink::Alternate)
        } else {
            Err(E::invalid_value(serde::de::Unexpected::Str(v), &self))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default)]
    defer: Vec<String>,
    #[serde(default)]
    show_if_empty: bool,
    #[serde(default)]
    show_if_offline: bool,
    #[serde(default = "make_true")]
    single_color: bool,
    #[serde(default)]
    version_link: VersionLink,
    #[serde(default = "One::one")]
    zoom: u8
}

impl Config {
    fn load() -> Result<Config, Error> {
        let dirs = xdg_basedir::get_config_home().into_iter().chain(xdg_basedir::get_config_dirs());
        for cfg_dir in dirs {
            let path = cfg_dir.join("bitbar/plugins/wurstmineberg.json");
            if path.exists() {
                return Ok(serde_json::from_reader(File::open(path)?)?);
            }
        }
        Ok(Config::default())
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            defer: Vec::default(),
            show_if_empty: false,
            show_if_offline: false,
            single_color: true,
            version_link: VersionLink::Enabled,
            zoom: 1
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
struct Cache(HashMap<Uid, Vec<u8>>);

impl Cache {
    fn load() -> Result<Cache, Error> {
        let path = xdg_basedir::get_cache_home()?.join("bitbar/plugin/wurstmineberg/avatars.json");
        if path.exists() {
            return Ok(serde_json::from_reader(File::open(path)?)?);
        } else {
            Ok(Cache::default())
        }
    }

    fn save(self) -> Result<(), Error> {
        let path = xdg_basedir::get_cache_home()?.join("bitbar/plugin/wurstmineberg/avatars.json");
        Ok(serde_json::to_writer(File::create(path)?, &self)?)
    }

    fn get_img(&mut self, uid: Uid, _ /*zoom*/: u8) -> Result<Image, Error> {
        self.0.entry(uid.clone())
            .or_try_insert_with(|| {
                let avatar_info = reqwest::blocking::get(&format!("https://wurstmineberg.de/api/v3/person/{}/avatar.json", uid))?
                    .error_for_status()?
                    .json::<AvatarInfo>()?;
                let image = reqwest::blocking::get(avatar_info.url)?
                    .error_for_status()?
                    .image()?;
                //TODO resize to 16 * zoom and write with DPI 72 * zoom, see https://github.com/image-rs/image/issues/911
                let mut buf = Vec::default();
                image.resize_exact(16, 16, FilterType::Nearest).write_to(&mut buf, ImageFormat::PNG)?;
                Ok(buf)
            })
            .map(|buf| (&buf).into())
    }
}

#[derive(Debug, Deserialize)]
struct Status {
    #[serde(default)]
    list: Vec<Uid>,
    running: bool,
    version: String
}

impl Status {
    fn load() -> Result<Status, Error> {
        Ok(
            reqwest::blocking::get("https://wurstmineberg.de/api/v3/world/wurstmineberg/status.json")?
                .error_for_status()?
                .json()?
        )
    }
}

#[derive(Debug, Deserialize)]
struct People {
    people: HashMap<Uid, Person>
}

impl People {
    fn load() -> Result<People, Error> {
        Ok(
            reqwest::blocking::get("https://wurstmineberg.de/api/v3/people.json")?
                .error_for_status()?
                .json()?
        )
    }

    fn get(&self, uid: impl Into<Uid>) -> Option<&Person> {
        self.people.get(&uid.into())
    }
}

#[derive(Debug, Deserialize)]
struct AvatarInfo {
    url: Url
}

fn bitbar() -> Result<Menu, Error> {
    let config = Config::load()?;
    let status = Status::load()?;
    if !config.show_if_offline && !status.running { return Ok(Menu::default()); }
    if !config.show_if_empty && status.list.is_empty() { return Ok(Menu::default()); }
    let people = People::load()?;
    let mut cache = Cache::load()?;
    let menu = vec![
        {
            let head = ContentItem::new(if !status.running {
                "!".to_owned()
            } else if status.list.is_empty() {
                "".to_owned()
            } else {
                status.list.len().to_string()
            }).template_image(wurstpick(config.zoom))?;
            if config.single_color && status.list.len() == 1 && people.get(&status.list[0]).map_or(false, |person| person.fav_color.is_some()) {
                head.color(people.get(&status.list[0]).unwrap().fav_color.as_ref().unwrap())?
            } else {
                head
            }.into()
        },
        MenuItem::Sep,
        {
            let version_item = ContentItem::new(format!("Version: {}", status.version));
            match config.version_link {
                VersionLink::Enabled => version_item.href(format!("https://minecraft.gamepedia.com/Java_Edition_{}", status.version))?,
                VersionLink::Alternate => version_item.alt(ContentItem::new(format!("Version: {}", status.version)).color("blue")?.href(format!("https://minecraft.gamepedia.com/Java_Edition_{}", status.version))?),
                VersionLink::Disabled => version_item
            }.into()
        }
    ].into_iter()
        .chain(status.list.iter().map(|uid| {
            let person = people.get(uid).cloned().unwrap_or_default();
            let mut item = ContentItem::new(person.name.map_or_else(|| uid.to_string(), |name| name.to_string()))
                .href(format!("https://wurstmineberg.de/people/{}", uid))?
                .image(cache.get_img(uid.clone(), config.zoom)?)?;
            if let Some(fav_color) = person.fav_color {
                item = item.color(fav_color)?;
            }
            if let Some(discord) = person.discord {
                item = item.alt(
                    ContentItem::new(format!("@{}", discord.name()))
                        .color("blue")?
                        .href(discord.url())?
                        .image(cache.get_img(uid.clone(), config.zoom)?)?
                );
            }
            Ok(item.into())
        }).collect::<Result<Vec<MenuItem>, Error>>()?)
        .chain(vec![
            MenuItem::Sep,
            ContentItem::new("Start Minecraft")
                .command(("/usr/bin/open", "-a", "Minecraft"))
                .alt(ContentItem::new("Open in Discord").color("blue")?.href("https://discordapp.com/channels/88318761228054528/388412978677940226")?)
                .into()
        ])
        //TODO “Defer” submenu if configured
        .collect();
    cache.save()?;
    Ok(menu)
}

fn make_true() -> bool { true }

fn wurstpick(zoom: u8) -> Image {
    if zoom >= 2 {
        Image::template(&include_bytes!("../assets/wurstpick-2x.png")[..]).never_unwrap()
    } else {
        Image::template(&include_bytes!("../assets/wurstpick.png")[..]).never_unwrap()
    }
}

fn main() {
    match bitbar() {
        Ok(menu) => { print!("{}", menu); }
        Err(e) => {
            let zoom = Config::load().map(|config| config.zoom).unwrap_or(1);
            print!("{}", Menu(vec![
                ContentItem::new("?").template_image(wurstpick(zoom)).never_unwrap().into(),
                MenuItem::Sep,
                MenuItem::new(format!("{:?}", e))
            ]));
        }
    }
}
