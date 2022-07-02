

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Property {
    pub name                    : String,
    pub value                   : PropertyValue,
}

impl Property {

    pub fn new_int(name: String, value: i32) -> Self {

        Self {
            name,
            value               : PropertyValue::Int(value),
        }
    }

    pub fn new_string(name: String, value: String) -> Self {

        Self {
            name,
            value               : PropertyValue::String(value.clone()),
        }
    }

    pub fn new_bool(name: String, value: bool) -> Self {

        Self {
            name,
            value               : PropertyValue::Bool(value),
        }
    }

    pub fn as_int(&self) -> Option<i32> {
        match &self.value {
            PropertyValue::Int(value) => Some(*value),
            PropertyValue::Float(_value) => None,
            PropertyValue::String(_value) => None,
            PropertyValue::Bool(_value) => None,
        }
    }

    pub fn as_float(&self) -> Option<f32> {
        match &self.value {
            PropertyValue::Int(_value) => None,
            PropertyValue::Float(value) => Some(*value),
            PropertyValue::String(_value) => None,
            PropertyValue::Bool(_value) => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match &self.value {
            PropertyValue::Int(_value) => None,
            PropertyValue::Float(_value) => None,
            PropertyValue::String(value) => Some(value.clone()),
            PropertyValue::Bool(_value) => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            PropertyValue::Int(_value) => None,
            PropertyValue::Float(_value) => None,
            PropertyValue::String(_value) => None,
            PropertyValue::Bool(value) => Some(*value),
        }
    }

    pub fn to_string(&self) -> String {
        let mut string = self.name.clone();
        string += " = ";

        let value_string = match &self.value {
            PropertyValue::Int(value) => value.to_string(),
            PropertyValue::Float(value) =>  value.to_string(),
            PropertyValue::String(value) => "\"".to_string() + (value.clone() + "\"").as_str(),
            PropertyValue::Bool(value) => value.to_string(),
        };
        string += value_string.as_str();
        return string;
    }

}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum PropertyValue {
    Int(i32),
    Float(f32),
    String(String),
    Bool(bool)
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct PropertySink {
    pub properties              : Vec<Property>,
}

impl PropertySink {

    pub fn new() -> Self {
        Self {
            properties          : vec![],
        }
    }

    /// Returns true if a property by the given name exists in the sink
    pub fn contains(&self, name: &str) -> bool {

        for p in &self.properties {
            if p.name == *name {
                return true;
            }
        }
        false
    }

    /// Get a clone of the given property name, if any
    pub fn get(&self, name: &String) -> Option<PropertyValue> {

        for p in &self.properties {
            if p.name == *name {
                return Some(p.value.clone());
            }
        }
        None
    }

    /// Append a new property to the sink
    pub fn push(&mut self, property: Property) {
        self.properties.push(property);
    }

    /// Convert the sink to a string
    pub fn to_string(&self) -> String {
        let mut string = "".to_string();

        for p in & self.properties {
            string += (p.to_string() + "\n").as_str();
        }

        string
    }

}
