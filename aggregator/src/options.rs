#[derive(Clone)]
pub struct ExtractOptions {
    pub user_id: Option<u64>,
    pub mongodb_client: Option<mongodb::Client>,
}

pub struct RunOptions {
    pub user_id: Option<u64>,
}
