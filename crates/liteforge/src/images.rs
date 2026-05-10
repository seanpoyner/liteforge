//! Image generation functionality.
//!
//! Provides types and functions for generating images using AI models.

use crate::client::AsyncForgeClient;
use crate::error::ForgeError;
use serde::{Deserialize, Serialize};

/// Request for image generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    /// The prompt to generate an image from.
    pub prompt: String,
    /// The model to use for generation.
    #[serde(default = "default_model")]
    pub model: String,
    /// Number of images to generate.
    #[serde(default = "default_n")]
    pub n: u32,
    /// Size of the generated images.
    #[serde(default = "default_size")]
    pub size: ImageSize,
    /// Response format (url or b64_json).
    #[serde(default)]
    pub response_format: ResponseFormat,
    /// Quality of the generated images.
    #[serde(default)]
    pub quality: ImageQuality,
    /// Style of the generated images.
    #[serde(default)]
    pub style: Option<ImageStyle>,
    /// A unique identifier for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

fn default_model() -> String {
    "dall-e-3".to_string()
}

fn default_n() -> u32 {
    1
}

fn default_size() -> ImageSize {
    ImageSize::Size1024x1024
}

impl ImageRequest {
    /// Create a new image request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: default_model(),
            n: default_n(),
            size: default_size(),
            response_format: ResponseFormat::default(),
            quality: ImageQuality::default(),
            style: None,
            user: None,
        }
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the number of images to generate.
    pub fn n(mut self, n: u32) -> Self {
        self.n = n;
        self
    }

    /// Set the image size.
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = size;
        self
    }

    /// Set the response format.
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = format;
        self
    }

    /// Set the quality.
    pub fn quality(mut self, quality: ImageQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Set the style.
    pub fn style(mut self, style: ImageStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set the user identifier.
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
}

/// Size of generated images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImageSize {
    /// 256x256 pixels.
    #[serde(rename = "256x256")]
    Size256x256,
    /// 512x512 pixels.
    #[serde(rename = "512x512")]
    Size512x512,
    /// 1024x1024 pixels.
    #[serde(rename = "1024x1024")]
    #[default]
    Size1024x1024,
    /// 1792x1024 pixels (DALL-E 3 only).
    #[serde(rename = "1792x1024")]
    Size1792x1024,
    /// 1024x1792 pixels (DALL-E 3 only).
    #[serde(rename = "1024x1792")]
    Size1024x1792,
}

impl std::fmt::Display for ImageSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageSize::Size256x256 => write!(f, "256x256"),
            ImageSize::Size512x512 => write!(f, "512x512"),
            ImageSize::Size1024x1024 => write!(f, "1024x1024"),
            ImageSize::Size1792x1024 => write!(f, "1792x1024"),
            ImageSize::Size1024x1792 => write!(f, "1024x1792"),
        }
    }
}

/// Response format for image generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Return URLs to the generated images.
    #[default]
    Url,
    /// Return base64-encoded image data.
    B64Json,
}

/// Quality setting for image generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageQuality {
    /// Standard quality (faster).
    #[default]
    Standard,
    /// HD quality (DALL-E 3 only).
    Hd,
}

/// Style for image generation (DALL-E 3 only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageStyle {
    /// More natural, less hyperreal images.
    Natural,
    /// Vivid, hyperreal images.
    Vivid,
}

/// Response from image generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    /// Unix timestamp of when the images were created.
    pub created: u64,
    /// The generated images.
    pub data: Vec<ImageData>,
}

impl ImageResponse {
    /// Get the first image URL if available.
    pub fn url(&self) -> Option<&str> {
        self.data.first().and_then(|d| d.url.as_deref())
    }

    /// Get all image URLs.
    pub fn urls(&self) -> Vec<&str> {
        self.data.iter().filter_map(|d| d.url.as_deref()).collect()
    }

    /// Get the first base64 data if available.
    pub fn b64_json(&self) -> Option<&str> {
        self.data.first().and_then(|d| d.b64_json.as_deref())
    }

    /// Get the revised prompt if available (DALL-E 3).
    pub fn revised_prompt(&self) -> Option<&str> {
        self.data.first().and_then(|d| d.revised_prompt.as_deref())
    }
}

/// Data for a single generated image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// URL of the generated image (when response_format is "url").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Base64-encoded image data (when response_format is "b64_json").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    /// The revised prompt used for generation (DALL-E 3 only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}

/// Generate an image from a prompt.
///
/// # Example
///
/// ```no_run
/// use liteforge::{AsyncForgeClient, images::{generate_image, ImageRequest}};
///
/// # async fn example() -> Result<(), liteforge::ForgeError> {
/// let client = AsyncForgeClient::new();
/// let request = ImageRequest::new("A sunset over mountains");
/// let response = generate_image(&client, request).await?;
/// println!("Image URL: {:?}", response.url());
/// # Ok(())
/// # }
/// ```
pub async fn generate_image(
    client: &AsyncForgeClient,
    request: ImageRequest,
) -> Result<ImageResponse, ForgeError> {
    let response = client.post("images/generations", &request).await?;
    Ok(response)
}

/// Request for image editing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEditRequest {
    /// The image to edit (base64 or URL).
    pub image: String,
    /// The prompt describing the edit.
    pub prompt: String,
    /// An optional mask image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<String>,
    /// The model to use.
    #[serde(default = "default_edit_model")]
    pub model: String,
    /// Number of images to generate.
    #[serde(default = "default_n")]
    pub n: u32,
    /// Size of the generated images.
    #[serde(default = "default_size")]
    pub size: ImageSize,
    /// Response format.
    #[serde(default)]
    pub response_format: ResponseFormat,
    /// User identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

fn default_edit_model() -> String {
    "dall-e-2".to_string()
}

impl ImageEditRequest {
    /// Create a new image edit request.
    pub fn new(image: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            prompt: prompt.into(),
            mask: None,
            model: default_edit_model(),
            n: default_n(),
            size: default_size(),
            response_format: ResponseFormat::default(),
            user: None,
        }
    }

    /// Set a mask image.
    pub fn mask(mut self, mask: impl Into<String>) -> Self {
        self.mask = Some(mask.into());
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the number of images.
    pub fn n(mut self, n: u32) -> Self {
        self.n = n;
        self
    }

    /// Set the size.
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = size;
        self
    }
}

/// Edit an image using AI.
pub async fn edit_image(
    client: &AsyncForgeClient,
    request: ImageEditRequest,
) -> Result<ImageResponse, ForgeError> {
    let response = client.post("images/edits", &request).await?;
    Ok(response)
}

/// Request for creating image variations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageVariationRequest {
    /// The image to create variations of.
    pub image: String,
    /// The model to use.
    #[serde(default = "default_edit_model")]
    pub model: String,
    /// Number of variations to generate.
    #[serde(default = "default_n")]
    pub n: u32,
    /// Size of the variations.
    #[serde(default = "default_size")]
    pub size: ImageSize,
    /// Response format.
    #[serde(default)]
    pub response_format: ResponseFormat,
    /// User identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl ImageVariationRequest {
    /// Create a new variation request.
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            model: default_edit_model(),
            n: default_n(),
            size: default_size(),
            response_format: ResponseFormat::default(),
            user: None,
        }
    }

    /// Set the number of variations.
    pub fn n(mut self, n: u32) -> Self {
        self.n = n;
        self
    }

    /// Set the size.
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = size;
        self
    }
}

/// Create variations of an image.
pub async fn create_variations(
    client: &AsyncForgeClient,
    request: ImageVariationRequest,
) -> Result<ImageResponse, ForgeError> {
    let response = client.post("images/variations", &request).await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_request_builder() {
        let request = ImageRequest::new("A beautiful sunset")
            .model("dall-e-3")
            .n(2)
            .size(ImageSize::Size1792x1024)
            .quality(ImageQuality::Hd)
            .style(ImageStyle::Vivid)
            .user("user123");

        assert_eq!(request.prompt, "A beautiful sunset");
        assert_eq!(request.model, "dall-e-3");
        assert_eq!(request.n, 2);
        assert_eq!(request.size, ImageSize::Size1792x1024);
        assert_eq!(request.quality, ImageQuality::Hd);
        assert_eq!(request.style, Some(ImageStyle::Vivid));
        assert_eq!(request.user, Some("user123".to_string()));
    }

    #[test]
    fn test_image_size_display() {
        assert_eq!(ImageSize::Size256x256.to_string(), "256x256");
        assert_eq!(ImageSize::Size512x512.to_string(), "512x512");
        assert_eq!(ImageSize::Size1024x1024.to_string(), "1024x1024");
        assert_eq!(ImageSize::Size1792x1024.to_string(), "1792x1024");
        assert_eq!(ImageSize::Size1024x1792.to_string(), "1024x1792");
    }

    #[test]
    fn test_image_response_helpers() {
        let response = ImageResponse {
            created: 1234567890,
            data: vec![
                ImageData {
                    url: Some("https://example.com/image1.png".to_string()),
                    b64_json: None,
                    revised_prompt: Some("A revised prompt".to_string()),
                },
                ImageData {
                    url: Some("https://example.com/image2.png".to_string()),
                    b64_json: None,
                    revised_prompt: None,
                },
            ],
        };

        assert_eq!(response.url(), Some("https://example.com/image1.png"));
        assert_eq!(response.urls().len(), 2);
        assert_eq!(response.revised_prompt(), Some("A revised prompt"));
    }

    #[test]
    fn test_image_edit_request() {
        let request = ImageEditRequest::new("base64data", "Make it blue")
            .mask("maskdata")
            .n(2)
            .size(ImageSize::Size512x512);

        assert_eq!(request.image, "base64data");
        assert_eq!(request.prompt, "Make it blue");
        assert_eq!(request.mask, Some("maskdata".to_string()));
        assert_eq!(request.n, 2);
    }

    #[test]
    fn test_image_variation_request() {
        let request = ImageVariationRequest::new("base64data")
            .n(3)
            .size(ImageSize::Size256x256);

        assert_eq!(request.image, "base64data");
        assert_eq!(request.n, 3);
        assert_eq!(request.size, ImageSize::Size256x256);
    }

    #[test]
    fn test_defaults() {
        let request = ImageRequest::new("test");

        assert_eq!(request.model, "dall-e-3");
        assert_eq!(request.n, 1);
        assert_eq!(request.size, ImageSize::Size1024x1024);
        assert_eq!(request.response_format, ResponseFormat::Url);
        assert_eq!(request.quality, ImageQuality::Standard);
    }
}
