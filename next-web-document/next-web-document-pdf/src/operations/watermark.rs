use crate::{error::pdf_error::PdfError, operation::PdfOperation};
use image::GenericImageView;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::path::PathBuf;

/// 水印操作配置
pub struct PdfWatermarkOperation {
    /// 水印内容 (文字或图片)
    pub content: WatermarkType,
    pub layout: WatermarkLayout,
    /// 旋转角度 (度)
    pub angle: f64,
    /// 透明度 (0.0 - 1.0, 1.0 为不透明)
    pub opacity: f32,
    /// 中心点偏移 (x, y) - 默认为 (0,0) 即页面中心
    pub offset: (f64, f64),
}

impl PdfOperation<()> for PdfWatermarkOperation {
    fn execute(self, doc: &mut Document) -> Result<(), PdfError> {
        // 1. 准备全局资源 (ExtGState) - 只创建一次
        let gs_id = add_transparency_group(doc, self.opacity)?;
        let gs_name = "WsGS";

        // 2. 准备内容资源 (图片/字体) - 只创建一次
        let (resource_id, resource_name, _) = match &self.content {
            WatermarkType::Text { font_name, .. } => (
                ObjectId::from((0, 0)),
                font_name.as_ref().map(|s| s.clone()).unwrap_or_default(),
                "Font",
            ),
            WatermarkType::Image { path, .. } => {
                let img_id = add_image_object(doc, path)?;
                (img_id, "WsImg".to_string(), "XObject")
            }
        };

        // 3. 收集所有页面 ID
        let page_ids: Vec<ObjectId> = doc.get_pages().values().cloned().collect();

        for page_id in page_ids {
            let page = doc
                .get_object_mut(page_id)
                .and_then(|obj| obj.as_dict_mut())
                .map_err(|e| {
                    PdfError::AnalysisError(format!("Get object for page {:?}: {}", page_id, e))
                })?;

            // --- A. 注册资源 ---
            if !page.has(b"Resources") {
                page.set("Resources", Dictionary::new());
            }
            let resources = page.get_mut(b"Resources").unwrap().as_dict_mut().unwrap();

            register_resource(resources, "ExtGState", gs_name, Object::Reference(gs_id));

            match &self.content {
                WatermarkType::Text { font_name, .. } => {
                    let font_dict = Dictionary::from_iter(vec![
                        ("Type", "Font".into()),
                        ("Subtype", "Type1".into()),
                        ("BaseFont", Object::Name(font_name.as_ref().map(|s| s.to_string()).map(|s| s.as_bytes().to_vec()).unwrap_or_default())),
                    ]);
                    register_resource(resources, "Font", "WsFont", Object::Dictionary(font_dict));
                }
                WatermarkType::Image { .. } => {
                    register_resource(
                        resources,
                        "XObject",
                        &resource_name,
                        Object::Reference(resource_id),
                    );
                }
            }

            // --- B. 获取页面尺寸 ---
            let media_box = page
                .get(b"MediaBox")
                .and_then(|o| o.as_array())
                .map_err(|e| PdfError::AnalysisError(format!("Missing MediaBox, {}", e)))?;
            let page_w = media_box[2].as_f32().unwrap_or(595.0) as f64;
            let page_h = media_box[3].as_f32().unwrap_or(842.0) as f64;

            // --- C. 计算绘制坐标点 (核心逻辑更新) ---
            let coordinates = match self.layout {
                WatermarkLayout::Single { offset_x, offset_y } => {
                    // 单点：页面中心 + 偏移
                    vec![(page_w / 2.0 + offset_x, page_h / 2.0 + offset_y)]
                }
                WatermarkLayout::Tile {
                    gap_x,
                    gap_y,
                    stagger,
                } => {
                    let mut points = Vec::new();
                    // 简单的网格生成算法
                    // 从 margin 开始，每隔 gap 绘制一个
                    // 留一点边距，避免贴边太紧
                    let margin_x = gap_x / 2.0;
                    let margin_y = gap_y / 2.0;

                    let mut row_idx = 0;
                    let mut y = margin_y;
                    while y < page_h {
                        let mut x = margin_x;

                        // 如果开启交错 (stagger)，偶数行向右平移一半间距
                        if stagger && row_idx % 2 != 0 {
                            x += gap_x / 2.0;
                        }

                        while x < page_w {
                            points.push((x, y));
                            x += gap_x;
                        }
                        y += gap_y;
                        row_idx += 1;
                    }
                    points
                }
            };

            // --- D. 生成绘制指令流 ---
            // 我们把所有水印的绘制指令放在同一个 Stream 中
            let mut ops = Vec::new();

            // 预先计算旋转矩阵参数
            let angle_rad = self.angle.to_radians();
            let c = angle_rad.cos();
            let s = angle_rad.sin();

            // 遍历所有坐标点进行绘制
            for (cx, cy) in coordinates {
                // q: 保存状态 (使得每个水印的坐标变换互不影响)
                ops.push(Operation::new("q", vec![]));

                // gs: 设置透明度
                ops.push(Operation::new(
                    "gs",
                    vec![Object::Name(gs_name.as_bytes().to_vec())],
                ));

                // cm: 移动到指定中心点 (cx, cy) 并旋转
                // Matrix: [cos, sin, -sin, cos, x, y]
                ops.push(Operation::new(
                    "cm",
                    vec![
                        c.into(),
                        s.into(),
                        (-s).into(),
                        c.into(),
                        cx.into(),
                        cy.into(),
                    ],
                ));

                match &self.content {
                    WatermarkType::Text {
                        content,
                        font_size,
                        color_rgb,
                        ..
                    } => {
                        // 简单计算文字宽度用于居中 (仅 ASCII 准确)
                        let text_width = content.len() as f64 * (*font_size as f64 * 0.5);
                        let tx = -text_width / 2.0;
                        let ty = -(*font_size as f64) / 3.0;

                        ops.push(Operation::new("BT", vec![]));
                        ops.push(Operation::new(
                            "rg",
                            vec![color_rgb.0.into(), color_rgb.1.into(), color_rgb.2.into()],
                        ));
                        ops.push(Operation::new(
                            "Tf",
                            vec![Object::Name(b"WsFont".to_vec()), (*font_size).into()],
                        ));
                        ops.push(Operation::new("Td", vec![tx.into(), ty.into()]));
                        ops.push(Operation::new(
                            "Tj",
                            vec![Object::String(
                                content.as_bytes().to_vec(),
                                Default::default(),
                            )],
                        ));
                        ops.push(Operation::new("ET", vec![]));
                    }
                    WatermarkType::Image { width, height, .. } => {
                        // 图片居中绘制
                        // 因为已经移到 cx,cy 且旋转了，现在只需把图片画在 [-w/2, -h/2]
                        ops.push(Operation::new(
                            "cm",
                            vec![
                                (*width).into(),
                                0.into(),
                                0.into(),
                                (*height).into(),
                                (-width / 2.0).into(),
                                (-height / 2.0).into(),
                            ],
                        ));
                        ops.push(Operation::new(
                            "Do",
                            vec![Object::Name(resource_name.as_bytes().to_vec())],
                        ));
                    }
                }

                // Q: 恢复状态，准备画下一个
                ops.push(Operation::new("Q", vec![]));
            }

            // --- E. 写入内容流 ---
            let final_content = Content { operations: ops }.encode().unwrap();
            let stream_id = doc.add_object(Stream::new(Dictionary::new(), final_content));

            // 追加到 Contents 数组
            match doc
                .get_object_mut(page_id)
                .and_then(|obj| obj.as_dict_mut())?
                .get_mut(b"Contents")
            {
                Ok(Object::Reference(id)) => {
                    *doc.get_object_mut(page_id)
                        .and_then(|obj| obj.as_dict_mut())?
                        .get_mut(b"Contents")? =
                        Object::Array(vec![Object::Reference(*id), Object::Reference(stream_id)]);
                }
                Ok(Object::Array(arr)) => {
                    arr.push(Object::Reference(stream_id));
                }
                Err(_) => {
                    doc.get_object_mut(page_id)
                        .and_then(|obj| obj.as_dict_mut())?
                        .set("Contents", Object::Reference(stream_id));
                }
                _ => {}
            }
        }

        Ok(())
    }
}

/// 添加 ExtGState 对象来控制透明度
fn add_transparency_group(doc: &mut Document, alpha: f32) -> Result<ObjectId, PdfError> {
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"ExtGState".to_vec()));
    // ca = non-stroking alpha (填充透明度，用于文字内部或图片)
    dict.set("ca", Object::Real(alpha.into()));
    // CA = stroking alpha (描边透明度)
    dict.set("CA", Object::Real(alpha.into()));

    Ok(doc.add_object(Object::Dictionary(dict)))
}

/// 读取图片并将其作为 XObject 添加到文档中，返回 ID
fn add_image_object(doc: &mut Document, path: &std::path::Path) -> Result<ObjectId, PdfError> {
    let img = image::open(path)
        .map_err(|e| PdfError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let (width, height) = img.dimensions();
    let color_type = img.color();

    // 转换为 RGB8 (目前只演示 RGB 处理，若是 CMYK 或 Gray 需要额外逻辑)
    let rgb_img = img.into_rgb8();
    let bits_per_component = 8;

    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"XObject".to_vec()));
    dict.set("Subtype", Object::Name(b"Image".to_vec()));
    dict.set("Width", width);
    dict.set("Height", height);
    dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec())); // 假设转换为了 RGB
    dict.set("BitsPerComponent", bits_per_component);
    // 使用 FlateDecode (Zip 压缩) 存储图片数据
    dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));

    // 创建流对象
    let stream = Stream::new(dict, rgb_img.into_raw());
    Ok(doc.add_object(stream))
}

/// 辅助：向 Resources 字典中注册子资源
fn register_resource(
    resources: &mut Dictionary,
    kind: &str, // e.g., "Font", "XObject", "ExtGState"
    name: &str, // e.g., "WsFont"
    value: Object,
) {
    if let Ok(sub_dict) = resources
        .get_mut(kind.as_bytes())
        .and_then(|o| o.as_dict_mut())
    {
        sub_dict.set(name, value);
    } else {
        // 如果该类型的字典不存在，创建一个
        let mut sub_dict = Dictionary::new();
        sub_dict.set(name, value);
        resources.set(kind, sub_dict);
    }
}

/// 水印的具体类型
pub enum WatermarkType {
    /// 文字水印
    Text {
        content: String,
        /// 标准字体名称: Helvetica, Times-Roman, Courier 等
        font_name: Option<String>,
        font_size: f32,
        /// RGB 颜色 (0.0 - 1.0), 例如 (1.0, 0.0, 0.0) 是红色
        color_rgb: (f32, f32, f32),
    },
    /// 图片水印
    Image {
        path: PathBuf,
        /// 图片在 PDF 上的显示宽度 (单位: pt)
        width: f32,
        /// 图片在 PDF 上的显示高度 (单位: pt)
        height: f32,
    },
}

/// 水印布局模式
#[derive(Debug, Clone, PartialEq)]
pub enum WatermarkLayout {
    /// 单个水印 (居中 + 偏移)
    Single { offset_x: f64, offset_y: f64 },
    /// 平铺模式 (满屏重复)
    Tile {
        /// 水平间距 (例如 200.0)
        gap_x: f64,
        /// 垂直间距 (例如 200.0)
        gap_y: f64,
        /// 是否交错排列 (偶数行会向右偏移一半间距，视觉效果更好)
        stagger: bool,
    },
}
