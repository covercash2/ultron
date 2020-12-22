const ITEM_EMOJIS: &[&str] = &[
    "none",
];

pub struct Item {
    id: u64,
    /// typ is short for type
    typ: ItemType
}

pub enum ItemType {
    MemberCard,
    BeggarCup,
}
