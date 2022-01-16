use poem::{endpoint::StaticFilesEndpoint, listener::TcpListener, Route, Server};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
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
                        StaticFilesEndpoint::new("public").index_file("index.html"),
                    ),
            )
            .await?;

        Ok(())
    }
}
