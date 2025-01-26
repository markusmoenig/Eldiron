use rusterix::{Light, ValueContainer};
use theframework::prelude::*;

#[derive(Clone)]
pub enum EffectWrapper {
    PointLight(ValueContainer),
}

use EffectWrapper::*;

impl EffectWrapper {
    pub fn name(&self) -> String {
        match self {
            PointLight(_) => "PointLight".into(),
        }
    }
    pub fn icon(&self) -> String {
        match self {
            PointLight(_) => "light_on".into(),
        }
    }

    /// Create the light ui for an Rusterix Light source
    pub fn create_light_ui(light: &Light) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();

        #[allow(clippy::single_match)]
        match light {
            rusterix::Light::PointLight {
                color,
                intensity,
                start_distance,
                end_distance,
                ..
            } => {
                let item = TheNodeUIItem::ColorPicker(
                    "lightColor".into(),
                    "".into(),
                    "Set the color of the light".into(),
                    TheColor::from(*color),
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightIntensity".into(),
                    "Intensity".into(),
                    "Set the intensity of the light.".into(),
                    *intensity,
                    0.0..=4.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightStartDistance".into(),
                    "Fade Start".into(),
                    "Set the intensity of the light.".into(),
                    *start_distance,
                    0.0..=10.0,
                    false,
                );
                nodeui.add_item(item);

                let item = TheNodeUIItem::FloatEditSlider(
                    "lightEndDistance".into(),
                    "Fade End".into(),
                    "Set the intensity of the light.".into(),
                    *end_distance,
                    0.0..=10.0,
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
            PointLight(properties) => Some(Light::PointLight {
                position: Vec3::new(position.x, 0.0, position.y),
                color: [1.0, 1.0, 1.0],
                intensity: properties.get_float_default("intensity", 1.0),
                start_distance: 3.0,
                end_distance: 5.0,
                flicker: None,
            }),
        }
    }
}
