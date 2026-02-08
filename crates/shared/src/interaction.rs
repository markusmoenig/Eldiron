use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum InteractionType {
    Tell,
    Sell,
    Reply,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Interaction {
    pub interaction_type: InteractionType,
    pub from: Uuid,
    pub from_name: String,
    pub to: Uuid,
    pub reply_function: Option<String>,
    pub item_id: Option<Uuid>,
    pub value: TheValue,
}

impl Interaction {
    pub fn tell(from: Uuid, from_name: String, to: Uuid, text: String) -> Self {
        Self {
            interaction_type: InteractionType::Tell,
            from,
            from_name,
            to,
            reply_function: None,
            item_id: None,
            value: TheValue::Text(text),
        }
    }
}
