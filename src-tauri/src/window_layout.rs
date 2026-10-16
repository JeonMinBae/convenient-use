use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayoutCommand {
    LeftHalf, RightHalf, TopHalf, BottomHalf,
    TopLeft, TopRight, BottomLeft, BottomRight,
    FirstThird, CenterThird, LastThird, FirstTwoThirds, LastTwoThirds,
    Maximize, Center, NextDisplay,
}
