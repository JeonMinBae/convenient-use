use serde::{Deserialize, Serialize};

// mods: "Ctrl" | "Alt" | "Shift" | "Super"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyCombo {
    pub mods: Vec<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RemapRule {
    Simple { from: String, to: KeyCombo },
    TapHold { key: String, tap: KeyCombo, hold: KeyCombo },
    Layer { trigger: String, tap: Option<KeyCombo>, mappings: Vec<LayerMapping> },
    // trigger 는 단축키 문자열
    Macro { trigger: String, steps: Vec<KeyCombo> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerMapping {
    pub from: String,
    pub to: KeyCombo,
}
