pub mod defs {
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MsgUpdate {
        user: String,
        path: String,
        date: String,
        cycle: u16,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MsgStatus {
        date: String,
        cycle: u16,
    }
}
