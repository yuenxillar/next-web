use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageOutputFormat, Rgba};

use imageproc::drawing::draw_text;
use rusttype::{Font, Scale};

/// Image processing errors
#[derive(Debug)]
pub enum ImageProcessingError {
    /// Error loading image
    LoadError(String),
    /// Error saving image
    SaveError(String),
    /// Error with image dimensions
    DimensionError(String),
    /// Error with font loading
    FontError(String),
    /// Error with unsupported format
    UnsupportedFormat(String),
}

/// Compress an image by reducing quality and/or changing format
///
/// # Arguments
/// * `input_path` - Path to input image
/// * `output_path` - Path to save compressed image
/// * `quality` - Quality level (0-100)
/// * `format` - Output format ("jpeg", "png", or "webp")
pub fn compress_image(
    input_path: &str,
    output_path: &str,
    quality: u8,
    format: &str,
) -> Result<(), ImageProcessingError> {
    // Load image
    let img =
        image::open(input_path).map_err(|e| ImageProcessingError::LoadError(e.to_string()))?;

    // Save with specified quality and format
    let output_format = match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => ImageOutputFormat::Jpeg(quality),
        "png" => {
            // PNG压缩级别 (0-9)
            let compression = (9 - (quality as f32 / 100.0 * 9.0) as u8).min(9);
            ImageOutputFormat::Png
        }
        "webp" => {
            // WebP质量 (0-100)
            ImageOutputFormat::WebP
        }
        _ => return Err(ImageProcessingError::UnsupportedFormat(format.to_string())),
    };
    // 创建输出文件并保存
    let mut output_file =
        std::fs::File::create(output_path).map_err(|e| ImageProcessingError::SaveError(e.to_string()))?;

    img.write_to(&mut output_file, output_format)
        .map_err(|e| ImageProcessingError::SaveError(e.to_string()))?;

    Ok(())
}

/// Resize an image to specified dimensions
///
/// # Arguments
/// * `input_path` - Path to input image
/// * `output_path` - Path to save resized image
/// * `width` - Target width
/// * `height` - Target height
/// * `maintain_aspect_ratio` - Whether to maintain aspect ratio
pub fn resize_image(
    input_path: &str,
    output_path: &str,
    width: u32,
    height: u32,
    maintain_aspect_ratio: bool,
) -> Result<(), ImageProcessingError> {
    // Load image
    let img =
        image::open(input_path).map_err(|e| ImageProcessingError::LoadError(e.to_string()))?;

    // Calculate new dimensions maintaining aspect ratio if needed
    let (new_width, new_height) = if maintain_aspect_ratio {
        let (original_width, original_height) = img.dimensions();
        let ratio = original_width as f32 / original_height as f32;

        if width as f32 / height as f32 > ratio {
            ((height as f32 * ratio) as u32, height)
        } else {
            (width, (width as f32 / ratio) as u32)
        }
    } else {
        (width, height)
    };

    // Resize image using Lanczos3 filter for high quality
    let resized_img = img
        .resize(new_width, new_height, FilterType::Lanczos3)
        .to_rgba8();

    // Save resized image
    DynamicImage::ImageRgba8(resized_img)
        .save(output_path)
        .map_err(|e| ImageProcessingError::SaveError(e.to_string()))?;

    Ok(())
}

/// Add a text watermark to an image
///
/// # Arguments
/// * `input_path` - Path to input image
/// * `output_path` - Path to save watermarked image
/// * `text` - Watermark text
/// * `font_path` - Path to TrueType font file
/// * `font_size` - Font size in points
/// * `x` - X coordinate for watermark position
/// * `y` - Y coordinate for watermark position
/// * `opacity` - Opacity (0-255)
pub fn add_watermark(
    input_path: &str,
    output_path: &str,
    text: &str,
    font_path: &str,
    font_size: f32,
    x: i32,
    y: i32,
    opacity: u8,
) -> Result<(), ImageProcessingError> {
    // Load image
    let mut img = image::open(input_path)
        .map_err(|e| ImageProcessingError::LoadError(e.to_string()))?
        .to_rgba8();

    // Load font
    let font_data =
        std::fs::read(font_path).map_err(|e| ImageProcessingError::FontError(e.to_string()))?;
    let font = Font::try_from_bytes(&font_data[..]).ok_or(ImageProcessingError::FontError(
        "Failed to load font".to_string(),
    ))?;

    // Set font scale and color with opacity
    let scale = Scale::uniform(font_size);
    let color = Rgba([255, 255, 255, opacity]);

    // Draw text on image
    let img = draw_text(&mut img, color, x, y, scale, &font, text);

    // Save watermarked image
    DynamicImage::ImageRgba8(img)
        .save(output_path)
        .map_err(|e| ImageProcessingError::SaveError(e.to_string()))?;

    Ok(())
}

/// Batch process images with multiple operations
///
/// # Arguments
/// * `input_path` - Path to input image
/// * `output_path` - Path to save processed image
/// * `resize` - Optional resize parameters (width, height, maintain_aspect_ratio)
/// * `watermark` - Optional watermark parameters (text, font_path, font_size, x, y, opacity)
/// * `compress` - Optional compression parameters (quality, format)
pub fn batch_process_image(
    input_path: &str,
    output_path: &str,
    resize: Option<(u32, u32, bool)>,
    watermark: Option<(&str, &str, f32, i32, i32, u8)>,
    compress: Option<(u8, &str)>,
) -> Result<(), ImageProcessingError> {
    // Create a temporary path for intermediate processing
    let temp_path = format!("{}.tmp", output_path);
    let mut current_path = input_path.to_string();

    // Resize if requested
    if let Some((width, height, maintain_aspect_ratio)) = resize {
        resize_image(
            &current_path,
            &temp_path,
            width,
            height,
            maintain_aspect_ratio,
        )?;
        current_path = temp_path.clone();
    }

    // Add watermark if requested
    if let Some((text, font_path, font_size, x, y, opacity)) = watermark {
        let watermark_path = if resize.is_some() {
            format!("{}.watermark.tmp", output_path)
        } else {
            temp_path.clone()
        };
        add_watermark(
            &current_path,
            &watermark_path,
            text,
            font_path,
            font_size,
            x,
            y,
            opacity,
        )?;
        current_path = watermark_path;
    }

    // Compress if requested, otherwise just copy
    if let Some((quality, format)) = compress {
        compress_image(&current_path, output_path, quality, format)?;
    } else {
        std::fs::copy(&current_path, output_path)
            .map_err(|e| ImageProcessingError::SaveError(e.to_string()))?;
    }

    // Clean up temporary files
    if std::path::Path::new(&temp_path).exists() {
        std::fs::remove_file(&temp_path).ok();
    }
    let watermark_path = format!("{}.watermark.tmp", output_path);
    if std::path::Path::new(&watermark_path).exists() {
        std::fs::remove_file(&watermark_path).ok();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::{fs, time::SystemTime};

    // 创建测试用的临时图片
    fn create_test_image(width: u32, height: u32) -> String {
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);
        // 填充红色像素
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }

        let path = get_file_path();
        let mut file = std::fs::File::create(&path).unwrap();
        img.write_to(&mut file, image::ImageOutputFormat::Png)
            .unwrap();
        path
    }

    fn get_file_path() -> String {
        format!(
            "{}.png",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        )
    }
    #[test]
    fn test_resize_image() {
        let input_path = create_test_image(200, 200);
        println!("input_path: {:?}", input_path);

        // 测试固定尺寸缩放
        let output_path = get_file_path();
        resize_image(&input_path, &output_path, 100, 100, false).expect("Resize failed");

        // 验证输出尺寸
        let resized_img = image::open(output_path).expect("Failed to open resized image");
        assert_eq!(resized_img.dimensions(), (100, 100));

        // 测试保持纵横比缩放
        let output_path2 = get_file_path();
        resize_image(&input_path, &output_path2, 150, 50, true)
            .expect("Resize with aspect ratio failed");

        // 原始200x200，按宽度150缩放应得到150x150
        let resized_img2 = image::open(output_path2).expect("Failed to open resized image");
        assert_eq!(resized_img2.dimensions(), (50, 50));
    }

    #[test]
    fn test_compress_image() {
        let input_path = create_test_image(200, 200);

        // 测试JPEG压缩
        let jpeg_path = get_file_path();
        compress_image(&input_path, &jpeg_path, 50, "jpeg").expect("JPEG compression failed");
        assert!(
            fs::metadata(jpeg_path)
                .expect("JPEG file not created")
                .len()
                > 0
        );

        // 测试PNG压缩binding
        let png_path = &get_file_path();
        compress_image(&input_path, png_path, 50, "png").expect("PNG compression failed");
        assert!(fs::metadata(png_path).expect("PNG file not created").len() > 0);
    }

    #[test]
    fn test_add_watermark() {
        let w = 200;
        let h = 200;
        let input_path = &create_test_image(w, h);

        let font_path = "C:\\Windows\\Fonts\\arial.ttf";
        if !std::path::Path::new(font_path).exists() {
            eprintln!("Font file not found, skipping watermark test");
            return;
        }

        let output_path = &get_file_path();
        add_watermark(
            input_path,
            output_path,
            "Test Watermark",
            font_path,
            20.0,
            (w / 5) as i32,
            (h / 2) as i32,
            255,
        )
        .expect("Watermark failed");
    }

    #[test]
    fn test_batch_process_image() {
        let input_path = &create_test_image(400, 400);

        let output_path = &get_file_path();
        // 注意：此测试需要系统中存在字体文件
        let font_path = "C:\\Windows\\Fonts\\arial.ttf";
        if !std::path::Path::new(font_path).exists() {
            eprintln!("Font file not found, skipping batch process test");
            return;
        }

        // 批量处理：缩放+水印+压缩
        batch_process_image(
            input_path,
            output_path,
            Some((200, 200, true)),                             // 缩放
            Some(("Batch Test", font_path, 14.0, 10, 10, 150)), // 水印
            Some((70, "jpeg")),                                 // 压缩
        )
        .expect("Batch processing failed");

        assert!(
            fs::metadata(output_path)
                .expect("Batch processed file not created")
                .len()
                > 0
        );

        // 验证最终尺寸
        let final_img = image::open(output_path).expect("Failed to open batch processed image");
        assert_eq!(final_img.dimensions(), (200, 200));
    }
}
