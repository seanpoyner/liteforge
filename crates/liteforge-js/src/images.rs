use napi::bindgen_prelude::*;
use liteforge::images::{
    ImageQuality as RustImageQuality, ImageRequest as RustImageRequest,
    ImageResponse as RustImageResponse, ImageSize as RustImageSize, ImageStyle as RustImageStyle,
    ResponseFormat as RustResponseFormat,
};

#[napi(string_enum)]
pub enum ImageSize {
    Size256x256,
    Size512x512,
    Size1024x1024,
    Size1792x1024,
    Size1024x1792,
}

fn js_image_size_to_rust(s: &ImageSize) -> RustImageSize {
    match s {
        ImageSize::Size256x256 => RustImageSize::Size256x256,
        ImageSize::Size512x512 => RustImageSize::Size512x512,
        ImageSize::Size1024x1024 => RustImageSize::Size1024x1024,
        ImageSize::Size1792x1024 => RustImageSize::Size1792x1024,
        ImageSize::Size1024x1792 => RustImageSize::Size1024x1792,
    }
}

#[napi(string_enum)]
pub enum ImageQuality {
    Standard,
    Hd,
}

fn js_image_quality_to_rust(q: &ImageQuality) -> RustImageQuality {
    match q {
        ImageQuality::Standard => RustImageQuality::Standard,
        ImageQuality::Hd => RustImageQuality::Hd,
    }
}

#[napi(string_enum)]
pub enum ImageStyle {
    Natural,
    Vivid,
}

fn js_image_style_to_rust(s: &ImageStyle) -> RustImageStyle {
    match s {
        ImageStyle::Natural => RustImageStyle::Natural,
        ImageStyle::Vivid => RustImageStyle::Vivid,
    }
}

#[napi(string_enum)]
pub enum ImageResponseFormat {
    Url,
    B64Json,
}

fn js_format_to_rust(f: &ImageResponseFormat) -> RustResponseFormat {
    match f {
        ImageResponseFormat::Url => RustResponseFormat::Url,
        ImageResponseFormat::B64Json => RustResponseFormat::B64Json,
    }
}

#[napi(object)]
pub struct JsImageData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
    pub revised_prompt: Option<String>,
}

#[napi(object)]
pub struct JsImageResponse {
    pub created: i64,
    pub data: Vec<JsImageData>,
}

fn rust_image_response_to_js(r: &RustImageResponse) -> JsImageResponse {
    JsImageResponse {
        created: r.created as i64,
        data: r
            .data
            .iter()
            .map(|d| JsImageData {
                url: d.url.clone(),
                b64_json: d.b64_json.clone(),
                revised_prompt: d.revised_prompt.clone(),
            })
            .collect(),
    }
}

#[napi]
pub struct ImageRequest {
    inner: RustImageRequest,
}

#[napi]
impl ImageRequest {
    #[napi(constructor)]
    pub fn new(prompt: String) -> Self {
        Self {
            inner: RustImageRequest::new(prompt),
        }
    }

    #[napi]
    pub fn model(&mut self, model: String) -> &Self {
        self.inner = self.inner.clone().model(model);
        self
    }

    #[napi]
    pub fn n(&mut self, n: u32) -> &Self {
        self.inner = self.inner.clone().n(n);
        self
    }

    #[napi]
    pub fn size(&mut self, size: ImageSize) -> &Self {
        self.inner = self.inner.clone().size(js_image_size_to_rust(&size));
        self
    }

    #[napi]
    pub fn quality(&mut self, quality: ImageQuality) -> &Self {
        self.inner = self
            .inner
            .clone()
            .quality(js_image_quality_to_rust(&quality));
        self
    }

    #[napi]
    pub fn style(&mut self, style: ImageStyle) -> &Self {
        self.inner = self.inner.clone().style(js_image_style_to_rust(&style));
        self
    }

    #[napi]
    pub fn response_format(&mut self, format: ImageResponseFormat) -> &Self {
        self.inner = self
            .inner
            .clone()
            .response_format(js_format_to_rust(&format));
        self
    }
}

#[napi]
pub async fn generate_image(
    client: &crate::client::AsyncForgeClient,
    request: &ImageRequest,
) -> Result<JsImageResponse> {
    let result = liteforge::images::generate_image(&client.inner, request.inner.clone())
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(rust_image_response_to_js(&result))
}
