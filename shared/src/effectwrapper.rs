use rusterix::{Light, LightType};
use theframework::prelude::*;

#[derive(Clone)]
pub enum EffectWrapper {
    RusterixLight(Light),
}

use EffectWrapper::*;

impl EffectWrapper {
    pub fn name(&self) -> String {
        match self {
            RusterixLight(light) => match light.light_type {
                LightType::Area => "Area Light".into(),
                LightType::Daylight => "Daylight".into(),
                _ => "Point Light".into(),
            },
        }
    }
    pub fn icon(&self) -> String {
        match self {
            RusterixLight(_) => "light_on".into(),
        }
    }

    /// Create the light ui for an Rusterix Light source
    pub fn create_light_ui(light: &Light) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();

        #[allow(clippy::single_match)]
        match light.light_type {
            LightType::Point => {
                let item = TheNodeUIItem::ColorPicker(
                    "lightColor".into(),
                    "".into(),
                    "Set the color of the light".into(),
                    TheColor::from(light.get_color()),
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightIntensity".into(),
                    "Intensity".into(),
                    "Set the intensity of the light.".into(),
                    light.get_intensity(),
                    0.0..=4.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightStartDistance".into(),
                    "Fade Start".into(),
                    "Set the distance the light starts to fade.".into(),
                    light.get_start_distance(),
                    0.0..=100.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightEndDistance".into(),
                    "Fade End".into(),
                    "Set the distance the light fade ends.".into(),
                    light.get_end_distance(),
                    0.0..=100.0,
                    false,
                );
                nodeui.add_item(item);
                /*
                let item = TheNodeUIItem::Selector(
                    "lightType".into(),
                    "Type".into(),
                    "Select the type of light.".into(),
                    vec!["Yes".to_string(), "No".to_string()],
                    properties.get_int_default("light_type", 0),
                );
                nodeui.add_item(item);*/
            }
            LightType::Area => {
                let item = TheNodeUIItem::ColorPicker(
                    "lightColor".into(),
                    "".into(),
                    "Set the color of the light".into(),
                    TheColor::from(light.get_color()),
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightIntensity".into(),
                    "Intensity".into(),
                    "Set the intensity of the light.".into(),
                    light.get_intensity(),
                    0.0..=4.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightStartDistance".into(),
                    "Fade Start".into(),
                    "Set the distance the light starts to fade.".into(),
                    light.get_start_distance(),
                    0.0..=100.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightEndDistance".into(),
                    "Fade End".into(),
                    "Set the distance the light fade ends.".into(),
                    light.get_end_distance(),
                    0.0..=100.0,
                    false,
                );
                nodeui.add_item(item);
                /*
                let item = TheNodeUIItem::Selector(
                    "lightType".into(),
                    "Type".into(),
                    "Select the type of light.".into(),
                    vec!["Yes".to_string(), "No".to_string()],
                    properties.get_int_default("light_type", 0),
                );
                nodeui.add_item(item);*/
            }
            _ => {}
        }

        nodeui
    }

    pub fn to_light(&self, position: Vec2<f32>) -> Option<Light> {
        match self {
            RusterixLight(light) => {
                let mut l = light.clone();
                l.set_position(Vec3::new(position.x, 0.0, position.y));
                Some(l)
            }
        }
    }
}
