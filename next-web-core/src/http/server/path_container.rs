use std::{collections::BTreeMap, sync::Arc};

pub type MultiValueMap = BTreeMap<String, Vec<String>>;

pub trait PathContainer
where
    Self: Send + Sync,
{
    fn value(&self) -> &str;

    fn elements(&self) -> &[Arc<dyn Element>];
}

#[allow(unused)]
pub mod path_container_helper {
    use std::sync::Arc;

    use crate::http::server::{DefaultPathContainer, PathContainer, PathOptions};

    pub fn sub_path(
        container: &Arc<dyn PathContainer>,
        start_index: usize,
        end_index: usize,
    ) -> Arc<dyn PathContainer> {
        DefaultPathContainer::sub_path(container, start_index, end_index)
    }

    pub fn sub_path_with_index(
        container: &Arc<dyn PathContainer>,
        index: usize,
    ) -> Arc<dyn PathContainer> {
        sub_path(container, index, container.elements().len())
    }

    pub fn parse_path(path: &str) -> Arc<dyn PathContainer> {
        DefaultPathContainer::create_from_url_path(path, &PathOptions::HTTP_PATH)
    }

    pub fn parse_path_with_options(path: &str, options: PathOptions) -> Arc<dyn PathContainer> {
        DefaultPathContainer::create_from_url_path(path, &options)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathOptions {
    /// path separator
    separator: char,
    /// Whether to decode and parse the path segment (including matrix parameters)
    decode_and_parse_segments: bool,
}

impl PathOptions {
    /// HTTP 路径模式：'/' 分隔，解码段并解析矩阵参数
    pub const HTTP_PATH: Self = Self {
        separator: '/',
        decode_and_parse_segments: true,
    };

    /// 消息路由模式：'.' 分隔，不解码段
    pub const MESSAGE_ROUTE: Self = Self {
        separator: '.',
        decode_and_parse_segments: false,
    };

    pub fn new(separator: char, decode_and_parse_segments: bool) -> Self {
        Self {
            separator,
            decode_and_parse_segments,
        }
    }

    pub fn separator(&self) -> char {
        self.separator
    }

    pub fn should_decode_and_parse_segments(&self) -> bool {
        self.decode_and_parse_segments
    }
}

pub trait Element
where
    Self: Send + Sync,
{
    fn value(&self) -> &str;
}

pub trait PathSegment: Element {
    fn value_to_match(&self) -> &str;

    fn value_to_match_as_chars(&self) -> Vec<char>;

    fn parameters(&self) -> &MultiValueMap;
}

#[allow(unused)]
pub trait Separator: Element {}
