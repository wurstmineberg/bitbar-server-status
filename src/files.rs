use {
    std::{
        collections::{
            BTreeMap,
            btree_map,
        },
        fmt,
        fs::File,
        path::PathBuf,
    },
    bitbar::Image,
    chrono::prelude::*,
    directories::UserDirs,
    image::{
        ImageFormat,
        imageops::FilterType,
    },
    num_traits::One,
    serde::{
        Deserialize,
        Deserializer,
        Serialize,
        de::Visitor,
    },
    serde_json::Value as Json,
    crate::{
        AvatarInfo,
        Error,
        Uid,
        util::ResponseExt as _,
    },
};

#[derive(Debug)]
pub(crate) enum VersionLink {
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
pub(crate) struct Config {
    #[serde(default)]
    pub(crate) defer_specs: Vec<Vec<String>>,
    #[serde(default)]
    pub(crate) show_if_empty: bool,
    #[serde(default)]
    pub(crate) show_if_offline: bool,
    #[serde(default = "make_true")]
    pub(crate) single_color: bool,
    #[serde(default)]
    pub(crate) version_link: VersionLink,
    #[serde(default)]
    pub(crate) version_match: BTreeMap<String, String>,
    #[serde(default = "One::one")]
    pub(crate) zoom: u8,
}

impl Config {
    pub(crate) fn load() -> Result<Config, Error> {
        let dirs = xdg_basedir::get_config_home().into_iter().chain(xdg_basedir::get_config_dirs());
        Ok(dirs.filter_map(|data_dir| File::open(data_dir.join("bitbar/plugins/wurstmineberg.json")).ok())
            .next().map_or(Ok(Config::default()), serde_json::from_reader)?)
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            defer_specs: Vec::default(),
            show_if_empty: false,
            show_if_offline: false,
            single_color: true,
            version_link: VersionLink::Enabled,
            version_match: BTreeMap::default(),
            zoom: 1,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Data {
    pub(crate) deferred: Option<DateTime<Utc>>,
}

impl Data {
    pub(crate) fn load() -> Result<Data, Error> {
        let dirs = xdg_basedir::get_data_home().into_iter().chain(xdg_basedir::get_data_dirs());
        Ok(dirs.filter_map(|data_dir| File::open(data_dir.join("bitbar/plugin-cache/wurstmineberg.json")).ok())
            .next().map_or(Ok(Data::default()), serde_json::from_reader)?)
    }

    pub(crate) fn save(&mut self) -> Result<(), Error> {
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
pub(crate) struct Cache(BTreeMap<Uid, Vec<u8>>);

impl Cache {
    pub(crate) fn load() -> Result<Cache, Error> {
        let path = xdg_basedir::get_cache_home()?.join("bitbar/plugin/wurstmineberg/avatars.json");
        Ok(if path.exists() {
            serde_json::from_reader(File::open(path)?)?
        } else {
            Cache::default()
        })
    }

    pub(crate) fn save(self) -> Result<(), Error> {
        let path = xdg_basedir::get_cache_home()?.join("bitbar/plugin/wurstmineberg/avatars.json");
        Ok(serde_json::to_writer(File::create(path)?, &self)?)
    }

    pub(crate) async fn get_img(&mut self, client: &reqwest::Client, uid: Uid, _ /*zoom*/: u8) -> Result<Image, Error> {
        Ok(match self.0.entry(uid.clone()) {
            btree_map::Entry::Occupied(entry) => entry.get().into(),
            btree_map::Entry::Vacant(entry) => (&entry.insert({
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

#[derive(Deserialize, Serialize)]
pub(crate) struct LauncherData {
    pub(crate) profiles: BTreeMap<String, LauncherProfile>,
    #[serde(flatten)]
    _extra: BTreeMap<String, Json>,
}

impl LauncherData {
    fn path() -> Result<PathBuf, Error> {
        Ok(UserDirs::new().ok_or(Error::MissingHomeDir)?.home_dir().join("Library").join("Application Support").join("minecraft").join("launcher_profiles.json"))
    }

    pub(crate) fn load() -> Result<LauncherData, Error> {
        Ok(serde_json::from_reader(File::open(Self::path()?)?)?)
    }

    pub(crate) fn save(&mut self) -> Result<(), Error> {
        Ok(serde_json::to_writer_pretty(File::create(Self::path()?)?, &self)?)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LauncherProfile {
    pub(crate) last_version_id: String,
    #[serde(flatten)]
    _extra: BTreeMap<String, Json>,
}

fn make_true() -> bool { true }
