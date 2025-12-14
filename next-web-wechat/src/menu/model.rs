use std::{borrow::Cow, collections::HashMap};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct CreateMenuRequest {
    button: Vec<ItemBody>,
}

impl CreateMenuRequest {
    pub fn new() -> Self {
        Self {
            button: Vec::with_capacity(3),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Click,
    View,
    ScancodePush,
    ScancodeWaitmsg,
    PicSysphoto,
    PicPhotoOrAlbum,
    PicWeixin,
    LocationSelect,
    MediaId,
    ArticleId,
    ArticleViewLimited,
    // .....
}

#[derive(Clone, Debug, Serialize)]
pub struct ItemBody {
    name: Cow<'static, str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<ItemType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<Cow<'static, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    media_id: Option<Cow<'static, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    article_id: Option<Cow<'static, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    sub_button: Option<Vec<ItemBody>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryMenuRespnose {
    is_menu_open: u64,
    selfmenu_info: SelfmenuInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SelfmenuInfo {
    button: Vec<HashMap<Cow<'static, str>, Cow<'static, str>>>,
}
