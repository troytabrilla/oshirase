use aggregator::sources::AniListAPI;

#[tokio::main]
async fn main() {
    let json = AniListAPI::fetch_lists().await.unwrap();
    println!("{:#?}", json);
}
