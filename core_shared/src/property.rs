use crate::prelude::*;
use colors_transform::{Rgb, Color, AlphaColor};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
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

    pub fn new_intx(name: String, value: Vec<i32>) -> Self {
        Self {
            name,
            value               : PropertyValue::IntX(value),
        }
    }

    pub fn new_float(name: String, value: f32) -> Self {
        Self {
            name,
            value               : PropertyValue::Float(value),
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

    pub fn new_color(name: String, value: String) -> Self {
        Self {
            name,
            value               : PropertyValue::Color(value.clone()),
        }
    }

    pub fn as_int(&self) -> Option<i32> {
        match &self.value {
            PropertyValue::Int(value) => Some(*value),
            PropertyValue::IntX(_value) => None,
            PropertyValue::Float(_value) => None,
            PropertyValue::String(_value) => None,
            PropertyValue::Bool(_value) => None,
            PropertyValue::Color(_value) => None,
        }
    }

    pub fn as_float(&self) -> Option<f32> {
        match &self.value {
            PropertyValue::Int(_value) => None,
            PropertyValue::IntX(_value) => None,
            PropertyValue::Float(value) => Some(*value),
            PropertyValue::String(_value) => None,
            PropertyValue::Bool(_value) => None,
            PropertyValue::Color(_value) => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match &self.value {
            PropertyValue::Int(_value) => None,
            PropertyValue::IntX(_value) => None,
            PropertyValue::Float(_value) => None,
            PropertyValue::String(value) => {
                Some(value.clone())
            }
            PropertyValue::Bool(_value) => None,
            PropertyValue::Color(_value) => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            PropertyValue::Int(_value) => None,
            PropertyValue::IntX(_value) => None,
            PropertyValue::Float(_value) => None,
            PropertyValue::String(_value) => None,
            PropertyValue::Bool(value) => Some(*value),
            PropertyValue::Color(_value) => None,
        }
    }

    pub fn as_color(&self) -> Option<String> {
        match &self.value {
            PropertyValue::Int(_value) => None,
            PropertyValue::IntX(_value) => None,
            PropertyValue::Float(_value) => None,
            PropertyValue::String(_value) => None,
            PropertyValue::Bool(_value) => None,
            PropertyValue::Color(value) => Some(value.clone()),
        }
    }

    pub fn to_float(&self) -> f32 {
        match &self.value {
            PropertyValue::Int(value) => *value as f32,
            PropertyValue::IntX(value) => value[0] as f32,
            PropertyValue::Float(value) => *value,
            _ => { 0.0 }
        }
    }

    pub fn to_rgb(&self) -> Option<[u8; 4]> {
        match &self.value {
            PropertyValue::Int(_value) => None,
            PropertyValue::IntX(_value) => None,
            PropertyValue::Float(_value) => None,
            PropertyValue::String(_value) => None,
            PropertyValue::Bool(_value) => None,
            PropertyValue::Color(value) => {
                if let Some(rgb) = Rgb::from_hex_str(value).ok() {
                    return Some([rgb.get_red() as u8, rgb.get_green() as u8, rgb.get_blue() as u8, rgb.get_alpha() as u8]);
                }
                None
            }
        }
    }

    pub fn to_string(&self) -> String {
        let mut string = self.name.clone();
        string += " = ";

        let value_string = match &self.value {
            PropertyValue::Int(value) => value.to_string(),
            PropertyValue::IntX(value) => {
                let mut string = "".to_string();
                for i in 0..value.len() {
                    string += value[i].to_string().as_str();
                    if i < value.len() - 1  {
                        string += ", ";
                    }
                }
                string
            }
            PropertyValue::Float(value) =>  value.to_string(),
            PropertyValue::String(value) => "\"".to_string() + (value.clone() + "\"").as_str(),
            PropertyValue::Bool(value) => value.to_string(),
            PropertyValue::Color(value) => value.to_string(),
        };
        string += value_string.as_str();
        return string;
    }

}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum PropertyValue {
    Int(i32),
    IntX(Vec<i32>),
    Float(f32),
    String(String),
    Bool(bool),
    Color(String)
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PropertySink {
    pub properties              : Vec<Property>,

    pub error                   : Option<(usize, String)>
}

impl PropertySink {

    pub fn new() -> Self {
        Self {
            properties          : vec![],

            error               : None
        }
    }

    /// Loads the properties of the given source string.
    pub fn load_from_string(&mut self, source: String) -> bool {
        let mut lines = source.lines();

        self.error = None;
        self.properties = vec![];

        let mut line_counter = 1_usize;

        while let Some(line) = lines.next() {

            let mut split_comment = line.split("//");

            if let Some(left_of_comment) = split_comment.next() {
                if left_of_comment.is_empty() == false {

                    let mut split_equal = left_of_comment.split("=");

                    if let Some(mut left) = split_equal.next() {
                        if let Some(mut right) = split_equal.next() {

                            left = left.trim();
                            right = right.trim();

                            if left.is_empty() == false && right.is_empty() == false && split_equal.next().is_none() {
                                //println!("{} = {}", left, right);

                                if right == "false" || right == "true" {
                                    if right == "false" {
                                        self.properties.push(Property::new_bool(left.to_string(), false));
                                    } else {
                                        self.properties.push(Property::new_bool(left.to_string(), true));
                                    }
                                } else
                                // String ?
                                if right.starts_with("\"") && right.ends_with("\"") {
                                    let mut chars = right.chars();
                                    chars.next();
                                    chars.next_back();
                                    self.properties.push(Property::new_string(left.to_string(),  chars.as_str().to_string()));
                                } else
                                if right.starts_with("#") && Rgb::from_hex_str(right).is_ok() {
                                    self.properties.push(Property::new_color(left.to_string(), right.to_string()));
                                } else
                                // Int2 ?
                                if let Some(value) = self.string_to_int2(right.to_string()) {
                                    self.properties.push(Property::new_intx(left.to_string(), value));
                                } else
                                // Int ?
                                if let Some(value) = right.parse::<i32>().ok() {
                                    self.properties.push(Property::new_int(left.to_string(), value));
                                } else
                                // Float ?
                                if let Some(value) = right.parse::<f32>().ok() {
                                    self.properties.push(Property::new_float(left.to_string(), value));
                                } else{
                                    self.error = Some((line_counter, "Unknown Type".to_string()));
                                    return false;
                                }
                            } else {
                                self.error = Some((line_counter, "Syntax Error".to_string()));
                                return false;
                            }
                        }  else {
                            self.error = Some((line_counter, "Syntax Error".to_string()));
                            return false;
                        }
                    } else {
                        self.error = Some((line_counter, "Syntax Error".to_string()));
                        return false;
                    }
                }
            }

            line_counter += 1;
        }

        //println!("{:?}", self.properties);
        true
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
    pub fn get(&self, name: &str) -> Option<Property> {

        for p in &self.properties {
            if p.name == *name {
                return Some(p.clone());
            }
        }
        None
    }

    /// Append a new property to the sink
    pub fn push(&mut self, property: Property) {
        self.properties.push(property);
    }

    /// Convert the sink to a string
    pub fn to_string(&self, descriptions: FxHashMap<String, Vec<String>>) -> String {
        let mut string = "".to_string();

        for p in & self.properties {
            if let Some(desc) = descriptions.get(&p.name) {
                for s in desc {
                    let add = "// ".to_string() + s.as_str() + "\n";
                    string += add.as_str();
                }
            }
            string += (p.to_string() + "\n").as_str();
            //string += "\n";
        }

        string
    }

    /// Splits integer numbers separated by "," into an array
    pub fn string_to_int2(&self, string: String) -> Option<Vec<i32>> {
        if string.matches(",").count() >= 1 {
            let split = string.split(",");
            let vec: Vec<&str> = split.collect();

            if vec.len() >= 1 {
                let mut array = vec![];

                for i in 0..vec.len() {

                    if let Some(v) = vec[i].trim().parse::<i32>().ok() {
                        array.push(v);
                    } else {
                        return None;
                    }
                }
                return Some(array);
            }
        }
        None
    }

}
