use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PetKind {
    #[default]
    Droplet,
    Remy,
    Emile,
    Pikachu,
    Eevee,
    Bulbasaur,
    Charmander,
    Squirtle,
    Jigglypuff,
    Snoopy,
    Woodstock,
    Belle,
}

#[derive(Clone, Copy, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PetTheme {
    Droplet,
    Remy,
    Pokemon,
    Snoopy,
}

impl PetKind {
    pub fn theme(self) -> PetTheme {
        match self {
            Self::Droplet => PetTheme::Droplet,
            Self::Remy | Self::Emile => PetTheme::Remy,
            Self::Pikachu
            | Self::Eevee
            | Self::Bulbasaur
            | Self::Charmander
            | Self::Squirtle
            | Self::Jigglypuff => PetTheme::Pokemon,
            Self::Snoopy | Self::Woodstock | Self::Belle => PetTheme::Snoopy,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Droplet => "Droplet",
            Self::Remy => "Remy",
            Self::Emile => "Emile",
            Self::Pikachu => "Pikachu",
            Self::Eevee => "Eevee",
            Self::Bulbasaur => "Bulbasaur",
            Self::Charmander => "Charmander",
            Self::Squirtle => "Squirtle",
            Self::Jigglypuff => "Jigglypuff",
            Self::Snoopy => "Snoopy",
            Self::Woodstock => "Woodstock",
            Self::Belle => "Belle",
        }
    }
}

#[derive(Clone, Debug, Serialize, TS)]
pub struct PetOption {
    pub id: PetKind,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, TS)]
pub struct PetThemeView {
    pub id: PetTheme,
    pub name: String,
    pub pets: Vec<PetOption>,
}

pub fn catalog() -> Vec<PetThemeView> {
    use PetKind::*;
    [
        (PetTheme::Remy, "Remy", vec![Remy, Emile]),
        (
            PetTheme::Pokemon,
            "Pokémon",
            vec![Pikachu, Eevee, Bulbasaur, Charmander, Squirtle, Jigglypuff],
        ),
        (PetTheme::Snoopy, "Snoopy", vec![Snoopy, Woodstock, Belle]),
        (PetTheme::Droplet, "Droplet", vec![Droplet]),
    ]
    .into_iter()
    .map(|(id, name, pets)| PetThemeView {
        id,
        name: name.into(),
        pets: pets
            .into_iter()
            .map(|id| PetOption {
                id,
                name: id.name().into(),
            })
            .collect(),
    })
    .collect()
}
