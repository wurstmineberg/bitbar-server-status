#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_qualifications, warnings)]

use {
    std::{
        borrow::Cow,
        collections::{
            BTreeMap,
            HashMap,
        },
        convert::Infallible,
        env,
        ffi::OsString,
        fmt,
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
    image::ImageError,
    itertools::Itertools as _,
    mime::Mime,
    notify_rust::Notification,
    serde::Deserialize,
    url::Url,
    crate::{
        files::{
            Cache,
            Config,
            Data,
            LauncherData,
            VersionLink,
        },
        model::*,
        util::ResultNeverExt as _,
    },
};

mod files;
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
    MissingHomeDir,
    OsString(OsString),
    Reqwest(reqwest::Error),
    Timespec(timespec::Error),
    UnknownLauncherProfile(String),
    UnknownWorldName(String, String),
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
            Error::MissingHomeDir => write!(f, "could not find your user folder"),
            Error::OsString(_) => write!(f, "command argument was not valid UTF-8"),
            Error::Reqwest(e) => if let Some(url) = e.url() {
                write!(f, "reqwest error at {}: {}", url, e)
            } else {
                write!(f, "reqwest error: {}", e)
            },
            Error::Timespec(e) => e.fmt(f),
            Error::UnknownLauncherProfile(profile_id) => write!(f, "no profile named “{}” in launcher data", profile_id),
            Error::UnknownWorldName(profile_id, world_name) => write!(f, "unknown world name “{}” in versionMatch config for profile {}", world_name, profile_id),
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
    if !config.version_match.is_empty() {
        let mut launcher_data = LauncherData::load()?;
        let mut modified = false;
        for (profile_id, world_name) in config.version_match {
            let launcher_profile = launcher_data.profiles.get_mut(&profile_id).ok_or_else(|| Error::UnknownLauncherProfile(profile_id.clone()))?;
            let world_version = &statuses.get(&world_name).ok_or_else(|| Error::UnknownWorldName(profile_id, world_name))?.version;
            if launcher_profile.last_version_id != *world_version {
                launcher_profile.last_version_id = world_version.clone();
                modified = true;
            }
        }
        if modified { launcher_data.save()? }
    }
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
    if !config.defer_specs.is_empty() {
        menu.push(MenuItem::Sep);
        for spec in config.defer_specs {
            menu.push(ContentItem::new(format!("Defer Until {}", spec.join(" ")))
                .command(
                    Command::try_from(
                        vec![format!("{}", current_exe.display()), format!("defer")]
                            .into_iter()
                            .chain(spec)
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
