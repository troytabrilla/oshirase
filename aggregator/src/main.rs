use aggregator::sources::AniListAPI;

#[tokio::main]
async fn main() {
    let lists = AniListAPI::aggregate().await.unwrap();
    println!("{:#?}", lists);
}
