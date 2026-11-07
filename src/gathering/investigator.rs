use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Investigator {
    Tony,
    Patrice,
    Jacqueline,
}
impl Investigator {
    pub const ALL: [Self; 3] = [Self::Tony, Self::Patrice, Self::Jacqueline];
    pub fn index(self) -> usize {
        match self {
            Self::Tony => 0,
            Self::Patrice => 1,
            Self::Jacqueline => 2,
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Tony => "tony",
            Self::Patrice => "patrice",
            Self::Jacqueline => "jacqueline",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Tony => "Tony Morgan",
            Self::Patrice => "Patrice Hathaway",
            Self::Jacqueline => "Jacqueline Fine",
        }
    }
}
