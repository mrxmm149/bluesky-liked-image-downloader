use std::error::Error;
use std::io::{stdin, Read, Write};
use std::path::Path;

use atrium_api::app::bsky::feed::defs;
use atrium_xrpc_client::reqwest::ReqwestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let config_path = args.get(1).cloned().unwrap_or("config.toml".into());
    let config: Config = toml::from_str(&std::fs::read_to_string(config_path)?)?;

    let agent = atrium_api::agent::AtpAgent::new(
        ReqwestClient::new(config.service_uri),
        atrium_api::agent::store::MemorySessionStore::default(),
    );
    agent.login(config.login_id, config.login_pw).await?;

    let params = atrium_api::app::bsky::feed::get_actor_likes::Parameters {
        actor: config.target_user,
        cursor: None,
        limit: None,
    };
    let likes = agent.api.app.bsky.feed.get_actor_likes(params).await?.feed;

    futures::future::join_all(
        likes
            .iter()
            .map(|f| download_image(f, &config.download_directory)),
    )
    .await;

    pause();

    Ok(())
}

async fn download_image(
    f: &defs::FeedViewPost,
    download_directory: &String,
) -> Result<(), Box<dyn Error>> {
    let post = &f.post;
    // filename: kaedeko111.bsky.social/20240207_205601_bsky_3kkt7kjkqod2b_01.jpg
    let author_handle = post.author.handle.clone();

    // Technically we should follow https://www.docs.bsky.app/docs/advanced-guides/timestamps#sortat
    let creation_time =
        chrono::DateTime::parse_from_rfc3339(&post.indexed_at)?.format("%Y%m%d_%H%M%S");

    let post_id = post.uri.rsplit_once('/').unwrap().1;

    let mut urls: Vec<String> = Vec::new();
    if let Some(view) = &post.embed {
        if let defs::PostViewEmbedEnum::AppBskyEmbedImagesView(image) = view {
            urls = image.images.iter().map(|i| i.fullsize.clone()).collect();
        }
    }

    for (ii, url) in urls.iter().enumerate() {
        let dir = Path::new(download_directory).join(&author_handle);
        std::fs::create_dir_all(&dir)?;

        let ext = url.rsplit_once('@').unwrap().1;
        let file_name = format!("{}_bsky_{}_{:02}.{}", creation_time, post_id, ii + 1, ext);
        let path = dir.join(file_name);
        download_file(&url, path.to_str().unwrap()).await?;
    }

    Ok(())
}

async fn download_file(url: &str, file_name: &str) -> Result<(), Box<dyn Error>> {
    let content = reqwest::get(url).await?.bytes().await?;
    let mut file = std::fs::File::create(file_name)?;
    file.write_all(&content)?;
    println!("Downloaded {}", file_name);
    Ok(())
}

fn pause() {
    // let mut stdout = stdout();
    // stdout.write(b"Press Enter to continue...").unwrap();
    // stdout.flush().unwrap();
    println!("Press Enter to continue...");
    stdin().read(&mut [0]).unwrap();
}

#[derive(serde::Deserialize)]
struct Config {
    service_uri: String,
    login_id: String,
    login_pw: String,
    target_user: String,
    download_directory: String,
}
