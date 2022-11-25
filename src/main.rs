#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    std::{
        borrow::Cow,
        collections::{
            BTreeMap,
            HashMap,
        },
        convert::Infallible,
        env,
        fmt,
        io,
        time::Duration,
    },
    bitbar::{
        ContentItem,
        Menu,
        MenuItem,
        attr::{
            Command,
            Image,
        },
    },
    chrono::prelude::*,
    css_color_parser::ColorParseError,
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

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] ColorParse(#[from] ColorParseError),
    #[error(transparent)] Header(#[from] reqwest::header::ToStrError),
    #[error(transparent)] Image(#[from] ImageError),
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Json(#[from] serde_json::Error),
    #[error(transparent)] MimeFromStr(#[from] mime::FromStrError),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] Timespec(#[from] timespec::Error),
    #[error(transparent)] Url(#[from] url::ParseError),
    #[error(transparent)] Xdg(#[from] xdg::BaseDirectoriesError),
    #[error("BitBar command should have 1–6 parameters including the command name, but this one has {0}")]
    CommandLength(usize),
    #[error("given timespec matches no dates")]
    EmptyTimespec,
    #[error("{0} is not a known image MIME type")]
    InvalidMime(Mime),
    #[error("could not find your user folder")]
    MissingHomeDir,
    #[error("no profile named “{0}” in launcher data")]
    UnknownLauncherProfile(String),
    #[error("unknown world name “{1}” in versionMatch config for profile {0}")]
    UnknownWorldName(String, String),
}

impl From<Infallible> for Error {
    fn from(never: Infallible) -> Error {
        match never {}
    }
}

impl From<Error> for Menu {
    fn from(e: Error) -> Menu {
        let mut error_menu = Vec::default();
        match e {
            Error::Reqwest(e) => {
                error_menu.push(MenuItem::new(format!("reqwest error: {e}")));
                if let Some(url) = e.url() {
                    error_menu.push(ContentItem::new(format!("URL: {url}"))
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
                notify(&summary, format!("{e:?}"));
                panic!("{summary}: {e:?}");
            }
        }
    }
}

#[bitbar::command(varargs)]
fn defer(timespec: Vec<String>) -> Result<(), Error> {
    if timespec.is_empty() { return Err(Error::EmptyTimespec) }
    let mut data = Data::load()?;
    data.deferred = Some(timespec::next(timespec)?.ok_or(Error::EmptyTimespec)?);
    data.save()?;
    Ok(())
}

#[bitbar::main(
    error_template_image = "../assets/wurstpick-2x.png", //TODO use wurstpick.png for low-DPI screens?
    commands(defer),
)]
async fn main() -> Result<Menu, Error> {
    let current_exe = env::current_exe()?;
    let client = reqwest::Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .use_rustls_tls()
        .build()?;
    let data = Data::load()?;
    if data.deferred.map_or(false, |deferred| deferred >= Utc::now()) {
        return Ok(Menu::default())
    }
    let config = Config::load()?;
    let mut statuses = Status::load(&client).await?;
    for status in statuses.values_mut() {
        status.list.retain(|uid| !config.ignored_players.contains(uid));
    }
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
            menu.push(if status.running {
                let version_item = ContentItem::new(format!("Version: {}", status.version));
                match config.version_link {
                    VersionLink::Enabled => version_item.href(format!("https://minecraft.fandom.com/wiki/Java_Edition_{}", status.version))?,
                    VersionLink::Alternate => version_item.alt(ContentItem::new(format!("Version: {}", status.version)).color("blue")?.href(format!("https://minecraft.fandom.com/wiki/Java_Edition_{}", status.version))?),
                    VersionLink::Disabled => version_item,
                }.into()
            } else {
                MenuItem::new("Offline") //TODO add link to Discord channel?
            });
            for uid in status.list {
                let person = people.get(&uid).cloned().unwrap_or_default();
                let mut item = ContentItem::new(person.name.map_or_else(|| uid.to_string(), |name| name.to_string()))
                    .href(format!("https://wurstmineberg.de/people/{uid}"))?
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
        .command(("/usr/bin/open", "-a", "Minecraft"))?
        .into());
    if !config.defer_specs.is_empty() {
        menu.push(MenuItem::Sep);
        for spec in config.defer_specs {
            menu.push(ContentItem::new(format!("Defer Until {}", spec.iter().format(" ")))
                .command(
                    Command::try_from(
                        vec![format!("{}", current_exe.display()), format!("defer")]
                            .into_iter()
                            .chain(spec)
                            .collect::<Vec<_>>()
                    ).map_err(|v| Error::CommandLength(v.len()))?
                )?
                .refresh()
                .into());
        }
    }
    cache.save()?;
    Ok(Menu(menu))
}
