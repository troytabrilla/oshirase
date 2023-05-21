#[derive(Clone)]
pub struct ExtractOptions {
    pub mongodb_client: Option<mongodb::Client>,
}
