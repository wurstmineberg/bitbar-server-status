use {
    std::{
        convert::Infallible,
        io::Cursor,
    },
    async_trait::async_trait,
    image::{
        DynamicImage,
        ImageFormat,
    },
    mime::Mime,
    reqwest::header::CONTENT_TYPE,
    crate::Error,
};

#[async_trait]
pub(crate) trait ResponseExt {
    async fn image(self) -> Result<DynamicImage, Error>;
}

#[async_trait]
impl ResponseExt for reqwest::Response {
    async fn image(self) -> Result<DynamicImage, Error> {
        Ok(match self.headers().get(CONTENT_TYPE) {
            Some(content_type) => {
                let mime_type = content_type.to_str()?.parse::<Mime>()?;
                let format = match (mime_type.type_(), mime_type.subtype()) {
                    (mime::IMAGE, mime::BMP) => ImageFormat::Bmp,
                    (mime::IMAGE, mime::GIF) => ImageFormat::Gif,
                    (mime::IMAGE, mime::JPEG) => ImageFormat::Jpeg,
                    (mime::IMAGE, mime::PNG) => ImageFormat::Png,
                    (mime::IMAGE, subtype) if subtype.as_ref() == "webp" => ImageFormat::WebP,
                    _ => return Err(Error::InvalidMime(mime_type)),
                };
                let buf = self.bytes().await?;
                image::load(Cursor::new(buf), format)?
            }
            None => {
                let buf = self.bytes().await?;
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
            Err(never) => match never {},
        }
    }
}
