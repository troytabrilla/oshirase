use aggregator::sources::AniListAPI;
// use aggregator::sources::Source;

#[tokio::main]
async fn main() {
    let api = AniListAPI::from("config/anilist_api.yaml");
    println!("{:#?}", api);
}
