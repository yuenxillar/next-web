use std::path::Path;
use std::fs::File;
use std::io::Read;

/// 文件类型枚举
/// 
/// File type enumeration
#[derive(Debug, PartialEq, Eq)]
pub enum FileType {
    /// 图片类型
    /// 
    /// Image type
    Image(ImageType),
    /// 文档类型
    /// 
    /// Document type
    Document(DocumentType),
    /// 文本类型
    /// 
    /// Text type
    Text(TextType),
    /// 压缩包类型
    /// 
    /// Archive type
    Archive(ArchiveType),
    /// 未知类型
    /// 
    /// Unknown type
    Unknown,
}

/// 图片类型枚举
/// 
/// Image type enumeration
#[derive(Debug, PartialEq, Eq)]
pub enum ImageType {
    /// JPEG格式
    /// 
    /// JPEG format
    Jpeg,
    /// PNG格式
    /// 
    /// PNG format
    Png,
    /// GIF格式
    /// 
    /// GIF format
    Gif,
    /// BMP格式
    /// 
    /// BMP format
    Bmp,
    /// WebP格式
    /// 
    /// WebP format
    WebP,
    /// SVG格式
    /// 
    /// SVG format
    Svg,
}

/// 文档类型枚举
/// 
/// Document type enumeration
#[derive(Debug, PartialEq, Eq)]
pub enum DocumentType {
    /// PDF文档
    /// 
    /// PDF document
    Pdf,
    /// Word文档
    /// 
    /// Word document
    Doc,
    /// Word文档（新格式）
    /// 
    /// Word document (new format)
    Docx,
    /// Excel表格
    /// 
    /// Excel spreadsheet
    Xls,
    /// Excel表格（新格式）
    /// 
    /// Excel spreadsheet (new format)
    Xlsx,
    /// PowerPoint演示文稿
    /// 
    /// PowerPoint presentation
    Ppt,
    /// PowerPoint演示文稿（新格式）
    /// 
    /// PowerPoint presentation (new format)
    Pptx,
}

/// 文本类型枚举
/// Text type enumeration
#[derive(Debug, PartialEq, Eq)]
pub enum TextType {
    /// HTML文件
    /// 
    /// HTML file
    Html,
    /// XML文件
    /// 
    /// XML file
    Xml,
    /// JSON文件
    /// 
    /// JSON file
    Json,
    /// CSV文件
    /// 
    /// CSV file
    Csv,
    /// 纯文本文件
    /// 
    /// Plain text file
    Txt,
    /// Markdown文件
    /// 
    /// Markdown file
    Markdown,
}

/// 压缩包类型枚举
/// 
/// Archive type enumeration
#[derive(Debug, PartialEq, Eq)]
pub enum ArchiveType {
    /// ZIP压缩包
    /// 
    /// ZIP archive
    Zip,
    /// RAR压缩包
    /// 
    /// RAR archive
    Rar,
    /// TAR压缩包
    /// 
    /// TAR archive
    Tar,
    /// GZ压缩包
    /// 
    /// GZ archive
    Gz,
    /// 7Z压缩包
    /// 
    /// 7Z archive
    SevenZip,
}

/// 文件类型检测器
/// 
/// File type detector
pub struct FileTypeDetector;

impl FileTypeDetector {
    /// 通过文件路径检测文件类型
    /// 
    /// Detect file type by file path
    pub fn detect(path: &Path) -> FileType {
        // 首先检查文件扩展名
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                match ext_str.to_lowercase().as_str() {
                    // 图片类型
                    "jpg" | "jpeg" => return FileType::Image(ImageType::Jpeg),
                    "png" => return FileType::Image(ImageType::Png),
                    "gif" => return FileType::Image(ImageType::Gif),
                    "bmp" => return FileType::Image(ImageType::Bmp),
                    "webp" => return FileType::Image(ImageType::WebP),
                    "svg" => return FileType::Image(ImageType::Svg),
                    
                    // 文档类型
                    "pdf" => return FileType::Document(DocumentType::Pdf),
                    "doc" => return FileType::Document(DocumentType::Doc),
                    "docx" => return FileType::Document(DocumentType::Docx),
                    "xls" => return FileType::Document(DocumentType::Xls),
                    "xlsx" => return FileType::Document(DocumentType::Xlsx),
                    "ppt" => return FileType::Document(DocumentType::Ppt),
                    "pptx" => return FileType::Document(DocumentType::Pptx),
                    
                    // 文本类型
                    "html" | "htm" => return FileType::Text(TextType::Html),
                    "xml" => return FileType::Text(TextType::Xml),
                    "json" => return FileType::Text(TextType::Json),
                    "csv" => return FileType::Text(TextType::Csv),
                    "txt" => return FileType::Text(TextType::Txt),
                    "md" | "markdown" => return FileType::Text(TextType::Markdown),
                    
                    // 压缩包类型
                    "zip" => return FileType::Archive(ArchiveType::Zip),
                    "rar" => return FileType::Archive(ArchiveType::Rar),
                    "tar" => return FileType::Archive(ArchiveType::Tar),
                    "gz" => return FileType::Archive(ArchiveType::Gz),
                    "7z" => return FileType::Archive(ArchiveType::SevenZip),
                    
                    _ => {}
                }
            }
        }

        // 如果扩展名无法确定，尝试读取文件头
        if let Ok(mut file) = File::open(path) {
            let mut buffer = [0; 8];
            if let Ok(_) = file.read_exact(&mut buffer) {
                // 检查文件头
                match buffer {
                    // JPEG
                    [0xFF, 0xD8, 0xFF, ..] => return FileType::Image(ImageType::Jpeg),
                    // PNG
                    [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] => return FileType::Image(ImageType::Png),
                    // GIF
                    [0x47, 0x49, 0x46, 0x38, ..] => return FileType::Image(ImageType::Gif),
                    // BMP
                    [0x42, 0x4D, ..] => return FileType::Image(ImageType::Bmp),
                    // PDF
                    [0x25, 0x50, 0x44, 0x46, ..] => return FileType::Document(DocumentType::Pdf),
                    // ZIP
                    [0x50, 0x4B, 0x03, 0x04, ..] => return FileType::Archive(ArchiveType::Zip),
                    // RAR
                    [0x52, 0x61, 0x72, 0x21, ..] => return FileType::Archive(ArchiveType::Rar),
                    _ => {}
                }
            }
        }

        FileType::Unknown
    }

    /// 通过字节数组检测文件类型
    /// 
    /// Detect file type by byte array
    pub fn detect_from_bytes(bytes: &[u8]) -> FileType {
        if bytes.len() < 8 {
            return FileType::Unknown;
        }

        // 检查文件头
        match &bytes[0..8] {
            // JPEG
            [0xFF, 0xD8, 0xFF, ..] => return FileType::Image(ImageType::Jpeg),
            // PNG
            [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] => return FileType::Image(ImageType::Png),
            // GIF
            [0x47, 0x49, 0x46, 0x38, ..] => return FileType::Image(ImageType::Gif),
            // BMP
            [0x42, 0x4D, ..] => return FileType::Image(ImageType::Bmp),
            // PDF
            [0x25, 0x50, 0x44, 0x46, ..] => return FileType::Document(DocumentType::Pdf),
            // ZIP
            [0x50, 0x4B, 0x03, 0x04, ..] => return FileType::Archive(ArchiveType::Zip),
            // RAR
            [0x52, 0x61, 0x72, 0x21, ..] => return FileType::Archive(ArchiveType::Rar),
            // XML
            [0x3C, 0x3F, 0x78, 0x6D, 0x6C, 0x20, ..] => return FileType::Text(TextType::Xml),
            // HTML
            [0x3C, 0x21, 0x44, 0x4F, 0x43, 0x54, 0x59, 0x50] => return FileType::Text(TextType::Html),
            // JSON
            [0x7B, ..] | [0x5B, ..] => return FileType::Text(TextType::Json),
            _ => {}
        }

        // 检查文件内容特征
        if let Ok(content) = std::str::from_utf8(bytes) {
            if content.contains("<?xml") {
                return FileType::Text(TextType::Xml);
            }
            if content.contains("<html") || content.contains("<!DOCTYPE html") {
                return FileType::Text(TextType::Html);
            }
            if content.starts_with('{') || content.starts_with('[') {
                return FileType::Text(TextType::Json);
            }
        }

        FileType::Unknown
    }

    /// 判断是否为图片类型
    /// 
    /// Check if the file type is an image
    pub fn is_image(file_type: &FileType) -> bool {
        matches!(file_type, FileType::Image(_))
    }

    /// 判断是否为文档类型
    /// 
    /// Check if the file type is a document
    pub fn is_document(file_type: &FileType) -> bool {
        matches!(file_type, FileType::Document(_))
    }

    /// 判断是否为文本类型
    /// 
    /// Check if the file type is a text file
    pub fn is_text(file_type: &FileType) -> bool {
        matches!(file_type, FileType::Text(_))
    }

    /// 判断是否为压缩包类型
    /// 
    /// Check if the file type is an archive
    pub fn is_archive(file_type: &FileType) -> bool {
        matches!(file_type, FileType::Archive(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 测试图片类型检测
    /// 
    /// Test image type detection
    #[test]
    fn test_detect_image() {
        let path = PathBuf::from("test.jpg");
        let file_type = FileTypeDetector::detect(&path);
        assert!(matches!(file_type, FileType::Image(ImageType::Jpeg)));
    }


    /// 测试文档类型检测
    /// 
    /// Test document type detection
    #[test]
    fn test_detect_document() {
        let path = PathBuf::from("test.pdf");
        let file_type = FileTypeDetector::detect(&path);
        assert!(matches!(file_type, FileType::Document(DocumentType::Pdf)));
    }

    /// 测试文本类型检测
    /// 
    /// Test text type detection
    #[test]
    fn test_detect_text() {
        let path = PathBuf::from("test.html");
        let file_type = FileTypeDetector::detect(&path);
        assert!(matches!(file_type, FileType::Text(TextType::Html)));
    }

    /// 测试压缩包类型检测
    /// 
    /// Test archive type detection
    #[test]
    fn test_detect_archive() {
        let path = PathBuf::from("test.zip");
        let file_type = FileTypeDetector::detect(&path);
        assert!(matches!(file_type, FileType::Archive(ArchiveType::Zip)));
    }

    /// 测试字节数组检测
    /// 
    /// Test byte array detection
    #[test]
    fn test_detect_from_bytes() {
        // 测试JPEG
        let jpeg_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        let file_type = FileTypeDetector::detect_from_bytes(&jpeg_bytes);
        assert!(matches!(file_type, FileType::Image(ImageType::Jpeg)));

        // 测试PNG
        let png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let file_type = FileTypeDetector::detect_from_bytes(&png_bytes);
        assert!(matches!(file_type, FileType::Image(ImageType::Png)));

        // 测试XML
        let xml_bytes = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>";
        let file_type = FileTypeDetector::detect_from_bytes(xml_bytes);
        assert!(matches!(file_type, FileType::Text(TextType::Xml)));

        // 测试JSON
        let json_bytes = b"{\"key\": \"value\"}";
        let file_type = FileTypeDetector::detect_from_bytes(json_bytes);
        assert!(matches!(file_type, FileType::Text(TextType::Json)));
    }
}
