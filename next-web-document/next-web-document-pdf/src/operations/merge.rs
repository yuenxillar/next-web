use lopdf::{Document, Object, ObjectId, dictionary};

use crate::{PdfResult, operation::PdfOperation};

pub struct PdfMergeOperation {
    pub docs: Vec<Document>,
}

impl PdfOperation<Document> for PdfMergeOperation {
    fn execute(mut self, doc: &mut Document) -> PdfResult<Document> {
        let mut docs = vec![doc];
        docs.extend(self.docs.iter_mut());
        Ok(Self::merge_documents(docs))
    }
}

impl PdfMergeOperation {
    fn merge_documents(documents: Vec<&mut Document>) -> Document {
        let mut target_doc = Document::with_version("1.5");
        let root_pages_id = target_doc.new_object_id();
        let mut max_id = 1;
        let mut all_page_ids = Vec::new();

        for doc in documents {
            // 1. 重排 ID
            doc.renumber_objects_with(max_id);
            max_id = doc.max_id + 1;

            // 2. 获取页面 ID 列表
            let pages_map = doc.get_pages();
            let current_page_ids: Vec<ObjectId> = pages_map.values().cloned().collect();

            // 3. 处理所有对象
            for (object_id, object) in doc.objects.iter() {
                match object.type_name().unwrap_or(b"") {
                    b"Page" => {
                        if let Ok(page_dict) = object.as_dict() {
                            let mut new_page_dict = page_dict.clone();

                            // [核心修复]：解析继承，并且"解引用" (Dereference)
                            // 确保页面拿到的是真实的字典数据，而不是指向将被删除的父节点的引用
                            Self::resolve_inheritance_deep(&doc, &mut new_page_dict, b"Resources");
                            Self::resolve_inheritance_deep(&doc, &mut new_page_dict, b"MediaBox");
                            Self::resolve_inheritance_deep(&doc, &mut new_page_dict, b"CropBox");
                            Self::resolve_inheritance_deep(&doc, &mut new_page_dict, b"Rotate");

                            // 指向新的根节点
                            new_page_dict.set("Parent", root_pages_id);

                            target_doc
                                .objects
                                .insert(*object_id, Object::Dictionary(new_page_dict));
                        }
                    }
                    // 忽略旧结构
                    b"Catalog" | b"Pages" | b"Outlines" | b"Outline" => {
                        continue;
                    }
                    // 搬运其他所有资源
                    _ => {
                        target_doc.objects.insert(*object_id, object.clone());
                    }
                }
            }

            all_page_ids.extend(current_page_ids);
        }

        // 4. 重建 Pages
        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Count" => all_page_ids.len() as u32,
            "Kids" => all_page_ids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
        };
        target_doc
            .objects
            .insert(root_pages_id, Object::Dictionary(pages_dict));

        // 5. 重建 Catalog
        let catalog_id = target_doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => root_pages_id,
        });
        target_doc.trailer.set("Root", catalog_id);
        target_doc.max_id = max_id;

        target_doc.compress();
        target_doc
    }

    /// 深度继承解析函数
    /// 1. 向上查找 Parent 链条找到属性
    /// 2. 如果属性是引用 (Reference)，则去文档中找到真实对象并 Clone 下来
    /// 3. 如果属性是直接对象，直接 Clone
    fn resolve_inheritance_deep(doc: &Document, dictionary: &mut lopdf::Dictionary, key: &[u8]) {
        // 如果页面自己已经有这个属性了，直接返回
        if dictionary.has(key) {
            return;
        }

        let mut current_dict = dictionary.clone();

        // 向上回溯 Parent (限制 50 层防止死循环)
        for _ in 0..50 {
            if let Ok(Object::Reference(parent_id)) = current_dict.get(b"Parent") {
                // 获取父节点对象
                if let Ok(parent_obj) = doc.get_object(*parent_id) {
                    if let Ok(parent_dict) = parent_obj.as_dict() {
                        // 检查父节点是否有我们要的 Key (如 Resources)
                        if let Ok(inherited_obj) = parent_dict.get(key) {
                            // [关键点]：检查拿到的对象是直接数据还是引用？
                            match inherited_obj {
                                Object::Reference(ref_id) => {
                                    // 如果是引用，我们需要"解引用" (Dereference)
                                    // 因为引用的目标对象(父节点)可能在合并过程中被我们删除了
                                    if let Ok(actual_obj) = doc.get_object(*ref_id) {
                                        // 拿到真实数据，克隆一份塞给页面
                                        // 这样页面就拥有了独立的资源字典，不再依赖死链接
                                        dictionary.set(key.to_vec(), actual_obj.clone());
                                    } else {
                                        // 如果引用是坏的，保留引用（虽然可能还是坏的，但没办法）
                                        dictionary.set(key.to_vec(), inherited_obj.clone());
                                    }
                                }
                                _ => {
                                    // 如果是直接数据 (Dictionary/Array/Integer)，直接克隆
                                    dictionary.set(key.to_vec(), inherited_obj.clone());
                                }
                            }
                            return;
                        }

                        // 没找到，继续把当前父节点当作起点，找爷爷
                        current_dict = parent_dict.clone();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                // 没有 Parent 了
                break;
            }
        }
    }
}
