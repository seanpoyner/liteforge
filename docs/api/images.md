# Images

Image generation, editing, and variation APIs.

## ImageRequest

```rust
use liteforge::images::{ImageRequest, ImageSize, ImageQuality, ImageStyle};

let request = ImageRequest::new("A sunset over mountains in watercolor style")
    .size(ImageSize::Size1024x1024)
    .quality(ImageQuality::Hd)
    .style(ImageStyle::Vivid)
    .n(1);
```

### Fields

| Field | Type | Default |
|-------|------|---------|
| `prompt` | `String` | Required |
| `size` | `ImageSize` | `Size1024x1024` |
| `quality` | `ImageQuality` | `Standard` |
| `style` | `ImageStyle` | `Vivid` |
| `n` | `u32` | `1` |
| `response_format` | `ResponseFormat` | `Url` |

## ImageSize

| Variant | Resolution |
|---------|------------|
| `Size256x256` | 256x256 |
| `Size512x512` | 512x512 |
| `Size1024x1024` | 1024x1024 |
| `Size1024x1792` | 1024x1792 (portrait) |
| `Size1792x1024` | 1792x1024 (landscape) |

## ImageQuality

| Variant | Description |
|---------|-------------|
| `Standard` | Standard quality |
| `Hd` | High-definition quality |

## ImageStyle

| Variant | Description |
|---------|-------------|
| `Vivid` | Vivid, hyper-real images |
| `Natural` | Natural, less hyper-real images |

## ResponseFormat

| Variant | Description |
|---------|-------------|
| `Url` | Returns a URL to the generated image |
| `B64Json` | Returns base64-encoded image data |

## Functions

### generate_image

```rust
use liteforge::images::generate_image;

let response = generate_image(&client, request).await?;
```

### edit_image

```rust
use liteforge::images::edit_image;

let response = edit_image(&client, image_bytes, mask_bytes, "Add a rainbow").await?;
```

### create_variations

```rust
use liteforge::images::create_variations;

let response = create_variations(&client, image_bytes, 3).await?;
```

## ImageResponse / ImageData

```rust
pub struct ImageResponse {
    pub created: u64,
    pub data: Vec<ImageData>,
}

pub struct ImageData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
    pub revised_prompt: Option<String>,
}
```

## JavaScript / TypeScript

```javascript
import { ImageRequest, ImageSize, ImageQuality, ImageStyle } from '@forge/sdk';

const request = new ImageRequest('A sunset over mountains');
request.size = ImageSize.Size1024x1024;
request.quality = ImageQuality.Hd;
request.style = ImageStyle.Vivid;
```
