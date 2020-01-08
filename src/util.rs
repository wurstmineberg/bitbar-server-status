use {
    std::{
        collections::hash_map::Entry,
        convert::Infallible,
        io::{
            Cursor,
            prelude::*
        }
    },
    image::{
        DynamicImage,
        ImageFormat
    },
    mime::Mime,
    reqwest::header::CONTENT_TYPE,
    crate::Error
};

pub(crate) trait EntryExt {
    type V;
    type VRef;

    fn or_try_insert_with<E>(self, default: impl FnOnce() -> Result<Self::V, E>) -> Result<Self::VRef, E>;
}

impl<'a, K, V> EntryExt for Entry<'a, K, V> {
    type V = V;
    type VRef = &'a mut V;

    fn or_try_insert_with<E>(self, default: impl FnOnce() -> Result<V, E>) -> Result<&'a mut V, E> {
        Ok(match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()?)
        })
    }
}

pub(crate) trait ResponseExt {
    fn image(&mut self) -> Result<DynamicImage, Error>;
}

impl ResponseExt for reqwest::blocking::Response {
    fn image(&mut self) -> Result<DynamicImage, Error> {
        Ok(match self.headers().get(CONTENT_TYPE) {
            Some(content_type) => {
                let mime_type = content_type.to_str()?.parse::<Mime>()?;
                let format = match (mime_type.type_(), mime_type.subtype()) {
                    (mime::IMAGE, mime::BMP) => ImageFormat::BMP,
                    (mime::IMAGE, mime::GIF) => ImageFormat::GIF,
                    (mime::IMAGE, mime::JPEG) => ImageFormat::JPEG,
                    (mime::IMAGE, mime::PNG) => ImageFormat::PNG,
                    (mime::IMAGE, subtype) if subtype.as_ref() == "webp" => ImageFormat::WEBP,
                    _ => { return Err(Error::InvalidMime(mime_type)); }
                };
                let mut buf = Vec::default();
                self.read_to_end(&mut buf)?;
                image::load(Cursor::new(buf), format)?
            }
            None => {
                let mut buf = Vec::default();
                self.read_to_end(&mut buf)?;
                image::load_from_memory(&buf)?
            }
        })
    }
}

pub(crate) trait ResultNeverExt {
    type Ok;

    fn never_unwrap(self) -> Self::Ok;
}

impl<T> ResultNeverExt for Result<T, Infallible> {
    type Ok = T;

    fn never_unwrap(self) -> T {
        match self {
            Ok(x) => x,
            Err(never) => match never {}
        }
    }
}
