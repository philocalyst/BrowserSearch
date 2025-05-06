// src/alfred.rs
use crate::search::SearchResult;
use crate::utils;
use serde::Serialize;
use std::error::Error;

/// Represents an Alfred Script Filter item
#[derive(Serialize, Debug)]
pub struct AlfredItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mods: Option<Mods>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quicklookurl: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Icon {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub path: String,
}

#[derive(Serialize, Debug)]
pub struct Mods {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<ModifierAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<ModifierAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctrl: Option<ModifierAction>,
}

#[derive(Serialize, Debug)]
pub struct ModifierAction {
    pub valid: bool,
    pub arg: String,
    pub subtitle: String,
}

#[derive(Serialize, Debug)]
pub struct Text {
    pub copy: String,
    pub largetype: String,
}

#[derive(Serialize, Debug)]
pub struct AlfredResponse {
    pub items: Vec<AlfredItem>,
}

/// Output search results to Alfred
pub fn output_results(items: &[SearchResult]) -> Result<(), Box<dyn Error>> {
    let alfred_items: Vec<AlfredItem> = items.iter().map(|result| result.into()).collect();

    let response = AlfredResponse {
        items: alfred_items,
    };

    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}

/// Convert a SearchResult to an AlfredItem
impl From<&SearchResult> for AlfredItem {
    fn from(result: &SearchResult) -> Self {
        let show_favicon = utils::get_env_bool("show_favicon");

        let favicon = if show_favicon {
            result.favicon.as_ref().map(|path| Icon {
                r#type: Some("fileicon".to_string()),
                path: path.clone(),
            })
        } else {
            None
        };

        AlfredItem {
            uid: Some(result.url.clone()),
            title: result.title.clone(),
            subtitle: Some(result.subtitle.clone()),
            arg: Some(result.url.clone()),
            icon: favicon,
            valid: Some(true),
            mods: Some(Mods {
                alt: Some(ModifierAction {
                    valid: true,
                    arg: result.url.clone(),
                    subtitle: result.url.clone(),
                }),
                cmd: Some(ModifierAction {
                    valid: true,
                    arg: result.url.clone(),
                    subtitle: "Other Actions...".to_string(),
                }),
                ctrl: None,
            }),
            text: None,
            quicklookurl: None,
        }
    }
}
