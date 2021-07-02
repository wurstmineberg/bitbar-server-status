#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_qualifications, warnings)]

use {
    std::{
        borrow::Cow,
        collections::{
            BTreeMap,
            HashMap,
            hash_map,
        },
        convert::Infallible,
        env,
        ffi::OsString,
        fmt,
        fs::File,
        io,
        time::Duration,
    },
    bitbar::{
        Command,
        ContentItem,
        Image,
        Menu,
        MenuItem,
    },
    chrono::prelude::*,
    css_color_parser::ColorParseError,
    derive_more::From,
    image::{
        ImageError,
        ImageFormat,
        imageops::FilterType,
    },
    itertools::Itertools as _,
    mime::Mime,
    notify_rust::Notification,
    num_traits::One,
    serde::{
        Deserialize,
        Deserializer,
        Serialize,
        de::Visitor,
    },
    url::Url,
    crate::{
        model::*,
        util::{
            ResponseExt as _,
            ResultNeverExt as _,
        },
    },
};

mod model;
mod util;

const MAIN_WORLD: &str = "wurstmineberg";

#[derive(Debug, From)]
enum Error {
    ColorParse(ColorParseError),
    #[from(ignore)]
    CommandLength(usize),
    EmptyTimespec,
    Header(reqwest::header::ToStrError),
    InvalidMime(Mime),
    Image(ImageError),
    Io(io::Error),
    Json(serde_json::Error),
    MimeFromStr(mime::FromStrError),
    MissingCliArg,
    OsString(OsString),
    Reqwest(reqwest::Error),
    Timespec(timespec::Error),
    Url(url::ParseError),
    Xdg(xdg_basedir::Error),
}

impl From<Infallible> for Error {
    fn from(never: Infallible) -> Error {
        match never {}
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ColorParse(e) => e.fmt(f),
            Error::CommandLength(num_params) => write!(f, "BitBar command should have 1–6 parameters including the command name, but this one has {}", num_params),
            Error::EmptyTimespec => write!(f, "given timespec matches no dates"),
            Error::Header(e) => e.fmt(f),
            Error::InvalidMime(mime) => write!(f, "{} is not a known image MIME type", mime),
            Error::Image(e) => write!(f, "image error: {}", e),
            Error::Io(e) => e.fmt(f),
            Error::Json(e) => e.fmt(f),
            Error::MimeFromStr(e) => e.fmt(f),
            Error::MissingCliArg => write!(f, "missing command-line argument(s)"),
            Error::OsString(_) => write!(f, "command argument was not valid UTF-8"),
            Error::Reqwest(e) => if let Some(url) = e.url() {
                write!(f, "reqwest error at {}: {}", url, e)
            } else {
                write!(f, "reqwest error: {}", e)
            },
            Error::Timespec(e) => write!(f, "timespec error: {:?}", e), //TODO implement Display fir timespec::Error and use here
            Error::Url(e) => e.fmt(f),
            Error::Xdg(e) => e.fmt(f),
        }
    }
}

impl From<Error> for Menu {
    fn from(e: Error) -> Menu {
        let zoom = Config::load().map(|config| config.zoom).unwrap_or(1);
        let mut error_menu = vec![
            ContentItem::new("?").template_image(wurstpick(zoom)).never_unwrap().into(),
            MenuItem::Sep
        ];
        match e {
            Error::Reqwest(e) => {
                error_menu.push(MenuItem::new(format!("reqwest error: {}", e)));
                if let Some(url) = e.url() {
                    error_menu.push(ContentItem::new(format!("URL: {}", url))
                        .href(url.clone()).expect("failed to add link to error menu")
                        .color("blue").expect("failed to parse the color blue")
                        .into());
                }
            }
            e => {
                error_menu.push(MenuItem::new(&e));
                error_menu.push(MenuItem::new(format!("{:?}", e)));
            }
        }
        error_menu.push(ContentItem::new("Report a Bug")
            .href("https://github.com/wurstmineberg/bitbar-server-status/issues/new").expect("failed to add link to error menu")
            .color("blue").expect("failed to parse the color blue")
            .into());
        Menu(error_menu)
    }
}

#[derive(Debug)]
enum VersionLink {
    Enabled,
    Alternate,
    Disabled,
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
    zoom: u8,
}

impl Config {
    fn load() -> Result<Config, Error> {
        let dirs = xdg_basedir::get_config_home().into_iter().chain(xdg_basedir::get_config_dirs());
        Ok(dirs.filter_map(|data_dir| File::open(data_dir.join("bitbar/plugins/wurstmineberg.json")).ok())
            .next().map_or(Ok(Config::default()), serde_json::from_reader)?)
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
            zoom: 1,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Data {
    #[serde(default)]
    pub(crate) defer_deltas: Vec<Vec<String>>,
    pub(crate) deferred: Option<DateTime<Utc>>,
}

impl Data {
    fn load() -> Result<Data, Error> {
        let dirs = xdg_basedir::get_data_home().into_iter().chain(xdg_basedir::get_data_dirs());
        Ok(dirs.filter_map(|data_dir| File::open(data_dir.join("bitbar/plugin-cache/wurstmineberg.json")).ok())
            .next().map_or(Ok(Data::default()), serde_json::from_reader)?)
    }

    fn save(&mut self) -> Result<(), Error> {
        let dirs = xdg_basedir::get_data_home().into_iter().chain(xdg_basedir::get_data_dirs());
        for data_dir in dirs {
            let data_path = data_dir.join("bitbar/plugin-cache/wurstmineberg.json");
            if data_path.exists() {
                if let Some(()) = File::create(data_path).ok()
                    .and_then(|data_file| serde_json::to_writer_pretty(data_file, &self).ok())
                {
                    return Ok(())
                }
            }
        }
        let data_path = xdg_basedir::get_data_home()?.join("bitbar/plugin-cache/wurstmineberg.json");
        let data_file = File::create(data_path)?;
        serde_json::to_writer_pretty(data_file, &self)?;
        Ok(())
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
struct Cache(HashMap<Uid, Vec<u8>>);

impl Cache {
    fn load() -> Result<Cache, Error> {
        let path = xdg_basedir::get_cache_home()?.join("bitbar/plugin/wurstmineberg/avatars.json");
        Ok(if path.exists() {
            serde_json::from_reader(File::open(path)?)?
        } else {
            Cache::default()
        })
    }

    fn save(self) -> Result<(), Error> {
        let path = xdg_basedir::get_cache_home()?.join("bitbar/plugin/wurstmineberg/avatars.json");
        Ok(serde_json::to_writer(File::create(path)?, &self)?)
    }

    async fn get_img(&mut self, client: &reqwest::Client, uid: Uid, _ /*zoom*/: u8) -> Result<Image, Error> {
        Ok(match self.0.entry(uid.clone()) {
            hash_map::Entry::Occupied(entry) => entry.get().into(),
            hash_map::Entry::Vacant(entry) => (&entry.insert({
                let AvatarInfo { url, fallbacks } = client.get(&format!("https://wurstmineberg.de/api/v3/person/{}/avatar.json", uid))
                    .send().await?
                    .error_for_status()?
                    .json().await?;
                let response = client.get(url)
                    .send().await
                    .map_err(Error::from)
                    .and_then(|response| Ok(response.error_for_status()?));
                let mut image = match response {
                    Ok(response) => response.image().await,
                    Err(e) => Err(e),
                };
                if image.is_err() {
                    for AvatarInfo { url, .. } in fallbacks {
                        if let Ok(response) = client.get(url).send().await.and_then(|response| response.error_for_status()) {
                            if let Ok(new_image) = response.image().await {
                                image = Ok(new_image);
                                break
                            }
                        }
                    }
                }
                let image = image?;
                //TODO resize to 16 * zoom and write with DPI 72 * zoom, see https://github.com/image-rs/image/issues/911
                let mut buf = Vec::default();
                image.resize_exact(16, 16, FilterType::Nearest).write_to(&mut buf, ImageFormat::Png)?;
                buf
            })).into(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct Status {
    #[serde(default)]
    list: Vec<Uid>,
    running: bool,
    version: String,
}

impl Status {
    async fn load(client: &reqwest::Client) -> Result<BTreeMap<String, Status>, Error> {
        Ok(
            client.get("https://wurstmineberg.de/api/v3/server/worlds.json")
                .query(&[("list", "1")])
                .send().await?
                .error_for_status()?
                .json().await?
        )
    }
}

#[derive(Debug, Deserialize)]
struct People {
    people: HashMap<Uid, Person>,
}

impl People {
    async fn load(client: &reqwest::Client) -> Result<People, Error> {
        Ok(
            client.get("https://wurstmineberg.de/api/v3/people.json")
                .send().await?
                .error_for_status()?
                .json().await?
        )
    }

    fn get(&self, uid: impl Into<Uid>) -> Option<&Person> {
        self.people.get(&uid.into())
    }
}

#[derive(Debug, Deserialize)]
struct AvatarInfo {
    url: Url,
    #[serde(default)]
    fallbacks: Vec<AvatarInfo>,
}

fn make_true() -> bool { true }

fn wurstpick(zoom: u8) -> Image {
    if zoom >= 2 {
        Image::template(&include_bytes!("../assets/wurstpick-2x.png")[..]).never_unwrap()
    } else {
        Image::template(&include_bytes!("../assets/wurstpick.png")[..]).never_unwrap()
    }
}

fn notify(summary: impl fmt::Display, body: impl fmt::Display) {
    //let _ = notify_rust::set_application(&notify_rust::get_bundle_identifier_or_default("BitBar")); //TODO uncomment when https://github.com/h4llow3En/mac-notification-sys/issues/8 is fixed
    let _ = Notification::default()
        .summary(&summary.to_string())
        .sound_name("Funk")
        .body(&body.to_string())
        .show();
}

trait ResultExt {
    type Ok;

    fn notify(self, summary: impl fmt::Display) -> Self::Ok;
}

impl<T, E: fmt::Debug> ResultExt for Result<T, E> {
    type Ok = T;

    fn notify(self, summary: impl fmt::Display) -> T {
        match self {
            Ok(t) => t,
            Err(e) => {
                notify(&summary, format!("{:?}", e));
                panic!("{}: {:?}", summary, e);
            }
        }
    }
}

#[bitbar::command]
fn defer(args: impl Iterator<Item = OsString>) -> Result<(), Error> {
    let args = args.map(|arg| arg.into_string()).collect::<Result<Vec<_>, _>>()?;
    if args.is_empty() { return Err(Error::MissingCliArg) }
    let mut data = Data::load()?;
    data.deferred = Some(timespec::next(args)?.ok_or(Error::EmptyTimespec)?);
    data.save()?;
    Ok(())
}

#[bitbar::main(error_template_image = "../assets/wurstpick-2x.png")] //TODO use wurstpick.png for low-DPI screens?
async fn main() -> Result<Menu, Error> {
    let current_exe = env::current_exe()?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("bitbar-wurstmineberg-status/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .use_rustls_tls()
        .build()?;
    let data = Data::load()?;
    if data.deferred.map_or(false, |deferred| deferred >= Utc::now()) {
        return Ok(Menu::default())
    }
    let config = Config::load()?;
    let statuses = Status::load(&client).await?;
    if statuses.values().all(|status| status.list.is_empty())
    && !if statuses[MAIN_WORLD].running { config.show_if_empty } else { config.show_if_offline } {
        return Ok(Menu::default())
    }
    let people = People::load(&client).await?;
    let mut cache = Cache::load()?;
    let mut menu = vec![{
        let total = statuses.values().map(|status| status.list.len()).sum::<usize>();
        let head = ContentItem::new(if total > 0 {
            Cow::Owned(total.to_string())
        } else if !statuses[MAIN_WORLD].running {
            Cow::Borrowed("!")
        } else {
            Cow::Borrowed("")
        }).template_image(wurstpick(config.zoom))?;
        if let Some(fav_color) = (config.single_color && total == 1).then(|| ())
            .and_then(|()| people.get(statuses.values().flat_map(|status| &status.list).exactly_one().expect("total == 1 but not exactly 1 player online")))
            .and_then(|person| person.fav_color)
        { head.color(fav_color)? } else { head }.into()
    }];
    for (world_name, status) in statuses {
        if (world_name == MAIN_WORLD && !status.running) || !status.list.is_empty() {
            menu.push(MenuItem::Sep);
            menu.push(MenuItem::new(world_name));
            menu.push({
                let version_item = ContentItem::new(format!("Version: {}", status.version));
                match config.version_link {
                    VersionLink::Enabled => version_item.href(format!("https://minecraft.fandom.com/wiki/Java_Edition_{}", status.version))?,
                    VersionLink::Alternate => version_item.alt(ContentItem::new(format!("Version: {}", status.version)).color("blue")?.href(format!("https://minecraft.fandom.com/wiki/Java_Edition_{}", status.version))?),
                    VersionLink::Disabled => version_item,
                }.into()
            });
            for uid in status.list {
                let person = people.get(&uid).cloned().unwrap_or_default();
                let mut item = ContentItem::new(person.name.map_or_else(|| uid.to_string(), |name| name.to_string()))
                    .href(format!("https://wurstmineberg.de/people/{}", uid))?
                    .image(cache.get_img(&client, uid.clone(), config.zoom).await?)?;
                if let Some(fav_color) = person.fav_color {
                    item = item.color(fav_color)?;
                }
                if let Some(discord) = person.discord {
                    item = item.alt(
                        ContentItem::new(format!("@{}", discord.name()))
                            .color("blue")?
                            .href(discord.url())?
                            .image(cache.get_img(&client, uid.clone(), config.zoom).await?)?
                    );
                }
                menu.push(item.into());
            }
        }
    }
    menu.push(MenuItem::Sep);
    menu.push(ContentItem::new("Start Minecraft")
        .command(("/usr/bin/open", "-a", "Minecraft"))
        .alt(ContentItem::new("Open in Discord").color("blue")?.href("https://discordapp.com/channels/88318761228054528/388412978677940226")?)
        .into());
    //TODO fix “Defer” submenu to be based on config?
    if !data.defer_deltas.is_empty() {
        menu.push(MenuItem::Sep);
        for delta in data.defer_deltas {
            menu.push(ContentItem::new(format!("Defer Until {}", delta.join(" ")))
                .command(
                    Command::try_from(
                        vec![format!("{}", current_exe.display()), format!("defer")]
                            .into_iter()
                            .chain(delta)
                            .collect::<Vec<_>>()
                    ).map_err(|v| Error::CommandLength(v.len()))?
                )
                .refresh()
                .into());
        }
    }
    cache.save()?;
    Ok(Menu(menu))
}
