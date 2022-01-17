use poem::{endpoint::StaticFilesEndpoint, listener::TcpListener, Route, Server};
use poem_openapi::{payload::Json, OpenApi, OpenApiService};

use crate::repos::{read_sync_repos, SyncRepository};

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/repos", method = "get")]
    async fn index(&self) -> Json<Vec<SyncRepository>> {
        let (_, repos) = read_sync_repos();
        Json(repos)
    }
}

pub struct RepoMirrorConfigServer {}

impl RepoMirrorConfigServer {
    pub fn new() -> RepoMirrorConfigServer {
        RepoMirrorConfigServer {}
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let api_service =
            OpenApiService::new(Api, "Hello World", "1.0").server("http://localhost:5000/api");
        let ui = api_service.swagger_ui();

        Server::new(TcpListener::bind("127.0.0.1:5000"))
            .run(
                Route::new()
                    .nest("/api", api_service)
                    .nest("/swagger", ui)
                    .nest(
                        "/",
                        StaticFilesEndpoint::new("ui/build").index_file("index.html"),
                    ),
            )
            .await?;

        Ok(())
    }
}
