use std::{
    fs::File,
    io::{Cursor, Write},
    path::Path,
    sync::Arc,
};

use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgba, RgbaImage};
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_line_segment_mut, draw_text_mut},
    geometric_transformations::{warp_into, Interpolation, Projection},
};
use rand::Rng;
use rusttype::{Font, Scale};

use super::{
    captcha_error::CaptchaError,
    code_generator::{CodeGenerator, RandomGenerator},
};

const FONT_BYTES: &[u8] = include_bytes!("OpenSans-Semibold.ttf");

/// Core captcha behavior.
pub trait ICaptcha {
    /// Creates a new random code and matching image.
    fn create_code(&mut self) -> Result<(), CaptchaError>;

    /// Returns the verification answer.
    fn get_code(&self) -> &str;

    /// Verifies user input. Implementations should ignore ASCII case.
    fn verify(&self, user_input: &str) -> bool {
        self.get_code().eq_ignore_ascii_case(user_input.trim())
    }

    /// Writes the current captcha image to an output stream.
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), CaptchaError>;
}

/// Common state and behavior shared by captcha implementations.
pub struct AbstractCaptcha {
    width: u32,
    height: u32,
    code_count: usize,
    display_code: String,
    verify_code: String,
    image_data: Vec<u8>,
    generator: Arc<dyn CodeGenerator>,
}

impl AbstractCaptcha {
    #[must_use]
    pub fn new(width: u32, height: u32, code_count: usize) -> Self {
        Self {
            width,
            height,
            code_count,
            display_code: String::new(),
            verify_code: String::new(),
            image_data: Vec::new(),
            generator: Arc::new(RandomGenerator::new(
                "23456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz",
                code_count,
            )),
        }
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn code_count(&self) -> usize {
        self.code_count
    }

    #[must_use]
    pub fn display_code(&self) -> &str {
        &self.display_code
    }

    #[must_use]
    pub fn verify_code(&self) -> &str {
        &self.verify_code
    }

    pub fn set_generator<G>(&mut self, generator: G)
    where
        G: CodeGenerator + 'static,
    {
        self.generator = Arc::new(generator);
    }

    fn regenerate_code(&mut self) {
        let code = self.generator.generate();
        self.display_code = code.display;
        self.verify_code = code.answer;
    }

    fn set_image(&mut self, image_data: Vec<u8>) {
        self.image_data = image_data;
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), CaptchaError> {
        writer.write_all(&self.image_data)?;
        Ok(())
    }

    fn write_to_path(&self, path: impl AsRef<Path>) -> Result<(), CaptchaError> {
        let mut file = File::create(path)?;
        self.write(&mut file)
    }
}

/// Captcha with line interference.
pub struct LineCaptcha {
    base: AbstractCaptcha,
    line_count: usize,
}

impl LineCaptcha {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self::with_code_count_and_line_count(width, height, 5, 150)
    }

    #[must_use]
    pub fn with_code_count(width: u32, height: u32, code_count: usize) -> Self {
        Self::with_code_count_and_line_count(width, height, code_count, 150)
    }

    #[must_use]
    pub fn with_code_count_and_line_count(
        width: u32,
        height: u32,
        code_count: usize,
        line_count: usize,
    ) -> Self {
        let mut captcha = Self {
            base: AbstractCaptcha::new(width, height, code_count),
            line_count,
        };
        let _ = captcha.create_code();
        captcha
    }

    pub fn set_generator<G>(&mut self, generator: G)
    where
        G: CodeGenerator + 'static,
    {
        self.base.set_generator(generator);
    }

    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), CaptchaError> {
        self.base.write_to_path(path)
    }

    fn create_image(&self) -> Result<Vec<u8>, CaptchaError> {
        let mut image = blank_image(self.base.width(), self.base.height());
        draw_random_lines(&mut image, self.line_count);
        draw_code_text(&mut image, self.base.display_code())?;
        encode_png(image)
    }
}

impl ICaptcha for LineCaptcha {
    fn create_code(&mut self) -> Result<(), CaptchaError> {
        validate_dimensions(
            self.base.width(),
            self.base.height(),
            self.base.code_count(),
        )?;
        self.base.regenerate_code();
        let image = self.create_image()?;
        self.base.set_image(image);
        Ok(())
    }

    fn get_code(&self) -> &str {
        self.base.verify_code()
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), CaptchaError> {
        self.base.write(writer)
    }
}

/// Captcha with circle interference.
pub struct CircleCaptcha {
    base: AbstractCaptcha,
    circle_count: usize,
}

impl CircleCaptcha {
    #[must_use]
    pub fn new(width: u32, height: u32, code_count: usize, circle_count: usize) -> Self {
        let mut captcha = Self {
            base: AbstractCaptcha::new(width, height, code_count),
            circle_count,
        };
        let _ = captcha.create_code();
        captcha
    }

    pub fn set_generator<G>(&mut self, generator: G)
    where
        G: CodeGenerator + 'static,
    {
        self.base.set_generator(generator);
    }

    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), CaptchaError> {
        self.base.write_to_path(path)
    }

    fn create_image(&self) -> Result<Vec<u8>, CaptchaError> {
        let mut image = blank_image(self.base.width(), self.base.height());
        draw_random_circles(&mut image, self.circle_count);
        draw_code_text(&mut image, self.base.display_code())?;
        encode_png(image)
    }
}

impl ICaptcha for CircleCaptcha {
    fn create_code(&mut self) -> Result<(), CaptchaError> {
        validate_dimensions(
            self.base.width(),
            self.base.height(),
            self.base.code_count(),
        )?;
        self.base.regenerate_code();
        let image = self.create_image()?;
        self.base.set_image(image);
        Ok(())
    }

    fn get_code(&self) -> &str {
        self.base.verify_code()
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), CaptchaError> {
        self.base.write(writer)
    }
}

/// Captcha with sheared text and line interference.
pub struct ShearCaptcha {
    base: AbstractCaptcha,
    thickness: usize,
}

impl ShearCaptcha {
    #[must_use]
    pub fn new(width: u32, height: u32, code_count: usize, thickness: usize) -> Self {
        let mut captcha = Self {
            base: AbstractCaptcha::new(width, height, code_count),
            thickness,
        };
        let _ = captcha.create_code();
        captcha
    }

    pub fn set_generator<G>(&mut self, generator: G)
    where
        G: CodeGenerator + 'static,
    {
        self.base.set_generator(generator);
    }

    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), CaptchaError> {
        self.base.write_to_path(path)
    }

    fn create_image(&self) -> Result<Vec<u8>, CaptchaError> {
        let mut image = blank_image(self.base.width(), self.base.height());
        draw_shear_lines(&mut image, self.thickness);
        draw_code_text(&mut image, self.base.display_code())?;
        let sheared = shear_image(&image);
        encode_png(sheared)
    }
}

impl ICaptcha for ShearCaptcha {
    fn create_code(&mut self) -> Result<(), CaptchaError> {
        validate_dimensions(
            self.base.width(),
            self.base.height(),
            self.base.code_count(),
        )?;
        self.base.regenerate_code();
        let image = self.create_image()?;
        self.base.set_image(image);
        Ok(())
    }

    fn get_code(&self) -> &str {
        self.base.verify_code()
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), CaptchaError> {
        self.base.write(writer)
    }
}

/// Factory helpers for the built-in captcha types.
pub struct CaptchaUtil;

impl CaptchaUtil {
    #[must_use]
    pub fn create_line_captcha(width: u32, height: u32) -> LineCaptcha {
        LineCaptcha::new(width, height)
    }

    #[must_use]
    pub fn create_circle_captcha(
        width: u32,
        height: u32,
        code_count: usize,
        circle_count: usize,
    ) -> CircleCaptcha {
        CircleCaptcha::new(width, height, code_count, circle_count)
    }

    #[must_use]
    pub fn create_shear_captcha(
        width: u32,
        height: u32,
        code_count: usize,
        thickness: usize,
    ) -> ShearCaptcha {
        ShearCaptcha::new(width, height, code_count, thickness)
    }
}

fn validate_dimensions(width: u32, height: u32, code_count: usize) -> Result<(), CaptchaError> {
    if width == 0 {
        return Err(CaptchaError::WidthNotApplicable);
    }
    if height == 0 {
        return Err(CaptchaError::HeightNotApplicable);
    }
    if code_count == 0 {
        return Err(CaptchaError::CodeLengthNotApplicable);
    }
    Ok(())
}

fn blank_image(width: u32, height: u32) -> RgbaImage {
    ImageBuffer::from_pixel(width, height, Rgba([245, 248, 250, 255]))
}

fn draw_random_lines(image: &mut RgbaImage, line_count: usize) {
    let mut rng = rand::thread_rng();
    let width = image.width() as f32;
    let height = image.height() as f32;

    for _ in 0..line_count {
        let color = random_color(90, 210);
        let start = (rng.gen_range(0.0..width), rng.gen_range(0.0..height));
        let end = (rng.gen_range(0.0..width), rng.gen_range(0.0..height));
        draw_line_segment_mut(image, start, end, color);
    }
}

fn draw_random_circles(image: &mut RgbaImage, circle_count: usize) {
    let mut rng = rand::thread_rng();
    let width = image.width().max(1) as i32;
    let height = image.height().max(1) as i32;
    let max_radius = (width.min(height) / 5).max(3);

    for _ in 0..circle_count {
        let center = (rng.gen_range(0..width), rng.gen_range(0..height));
        let radius = rng.gen_range(2..=max_radius);
        draw_filled_circle_mut(image, center, radius, random_color(130, 235));
    }
}

fn draw_shear_lines(image: &mut RgbaImage, thickness: usize) {
    let mut rng = rand::thread_rng();
    let width = image.width() as f32;
    let height = image.height() as f32;

    for index in 0..thickness.max(1) {
        let y = height * ((index + 1) as f32 / (thickness.max(1) + 1) as f32);
        let offset = rng.gen_range(-height * 0.18..height * 0.18);
        draw_line_segment_mut(image, (0.0, y), (width, y + offset), random_color(80, 180));
    }
}

fn draw_code_text(image: &mut RgbaImage, code: &str) -> Result<(), CaptchaError> {
    let font = Font::try_from_bytes(FONT_BYTES).ok_or(CaptchaError::FontLoadError)?;
    let height = image.height().max(1);
    let width = image.width().max(1);
    let count = code.chars().count().max(1) as u32;
    let font_size = (height as f32 * 0.62).max(16.0);
    let scale = Scale::uniform(font_size);
    let char_space = width as f32 / count as f32;
    let y = ((height as f32 - font_size) * 0.45).max(0.0) as i32;
    let mut rng = rand::thread_rng();

    for (index, ch) in code.chars().enumerate() {
        let x = (char_space * index as f32 + char_space * 0.16).round() as i32;
        let y_offset = rng.gen_range(-(height as i32 / 14).max(1)..=(height as i32 / 14).max(1));
        draw_text_mut(
            image,
            random_color(15, 120),
            x,
            y + y_offset,
            scale,
            &font,
            &ch.to_string(),
        );
    }

    Ok(())
}

fn shear_image(image: &RgbaImage) -> RgbaImage {
    let mut output = blank_image(image.width(), image.height());
    let projection = Projection::from_matrix([
        1.0,
        0.18,
        -(image.height() as f32 * 0.09),
        0.04,
        1.0,
        0.0,
        0.0003,
        0.0,
        1.0,
    ])
    .unwrap_or_else(|| Projection::scale(1.0, 1.0));
    warp_into(
        image,
        &projection,
        Interpolation::Bilinear,
        Rgba([245, 248, 250, 255]),
        &mut output,
    );
    output
}

fn random_color(min: u8, max: u8) -> Rgba<u8> {
    let mut rng = rand::thread_rng();
    let upper = max.max(min);
    Rgba([
        rng.gen_range(min..=upper),
        rng.gen_range(min..=upper),
        rng.gen_range(min..=upper),
        255,
    ])
}

fn encode_png(image: RgbaImage) -> Result<Vec<u8>, CaptchaError> {
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut output, ImageOutputFormat::Png)
        .map_err(|err| CaptchaError::ImageEncodeError(err.to_string()))?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::captcha::code_generator::MathGenerator;

    #[test]
    fn line_captcha_creates_and_verifies_code() {
        let mut captcha = CaptchaUtil::create_line_captcha(200, 100);
        captcha.set_generator(RandomGenerator::new("0123456789", 4));
        captcha.create_code().unwrap();

        let code = captcha.get_code().to_string();
        assert_eq!(code.len(), 4);
        assert!(captcha.verify(&code));

        let mut bytes = Vec::new();
        captcha.write(&mut bytes).unwrap();
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn circle_and_shear_captcha_write_png() {
        let circle = CaptchaUtil::create_circle_captcha(200, 100, 4, 20);
        let shear = CaptchaUtil::create_shear_captcha(200, 100, 4, 4);

        let mut circle_bytes = Vec::new();
        let mut shear_bytes = Vec::new();
        circle.write(&mut circle_bytes).unwrap();
        shear.write(&mut shear_bytes).unwrap();

        assert!(circle_bytes.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(shear_bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn math_generator_uses_answer_for_verification() {
        let mut captcha = CaptchaUtil::create_shear_captcha(200, 60, 4, 4);
        captcha.set_generator(MathGenerator::default());
        captcha.create_code().unwrap();

        assert!(!captcha.get_code().contains("=?"));
        assert!(captcha.verify(captcha.get_code()));
    }
}
