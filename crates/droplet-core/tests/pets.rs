use droplet_core::{
    model::{Config, Preferences},
    pets::{self, PetKind, PetTheme},
    protocol::MainRequest,
    Core,
};

#[test]
fn existing_settings_keep_droplet_and_unknown_pets_are_rejected() {
    let config: Config = serde_json::from_str(
        r#"{"version":1,"connections":[],"petVisible":false,"reduceMotion":true}"#,
    )
    .unwrap();
    assert_eq!(config.pet, PetKind::Droplet);
    assert!(!config.pet_visible);
    assert!(config.reduce_motion);
    assert!(serde_json::from_str::<Preferences>(r#"{"pet":"unknown"}"#).is_err());
    assert!(serde_json::from_str::<Config>(r#"{"pet":"../../external.svg"}"#).is_err());
}

#[tokio::test]
async fn every_catalog_pet_survives_restart_and_reaches_the_pet_view() {
    let directory = tempfile::tempdir().unwrap();
    let data = directory.path().join("data");
    let core = Core::open(data.clone(), directory.path().into());
    let mut seen = std::collections::HashSet::new();
    for theme in pets::catalog() {
        for option in theme.pets {
            assert!(seen.insert(serde_json::to_string(&option.id).unwrap()));
            assert_eq!(option.id.theme(), theme.id);
            core.execute(
                MainRequest::Preferences {
                    preferences: Preferences {
                        pet: option.id,
                        pet_visible: false,
                        reduce_motion: true,
                        ..Preferences::default()
                    },
                },
                false,
            )
            .await
            .unwrap();
            let reopened = Core::open(data.clone(), directory.path().into());
            let pet = reopened.pet().unwrap();
            assert_eq!(pet.pet, option.id);
            assert_eq!(pet.name, option.name);
            assert_eq!(pet.theme, theme.id);
            assert!(pet.reduce_motion);
            let view = reopened.view("").unwrap();
            assert_eq!(view.preferences.pet, option.id);
            assert!(!view.preferences.pet_visible);
            assert!(!view.launch_locked);
        }
    }
    assert_eq!(seen.len(), 12);
    assert_eq!(PetKind::Eevee.theme(), PetTheme::Pokemon);
}
