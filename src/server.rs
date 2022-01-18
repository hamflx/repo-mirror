use anyhow::anyhow;
use poem::{endpoint::StaticFilesEndpoint, listener::TcpListener, Route, Server};
use poem_openapi::{param::Path, payload::Json, Object, OpenApi, OpenApiService};
use serde_json::Value;

use crate::repos::{read_sync_repos, write_sync_repos, SyncRepository};

pub trait PropertySetter<T> {
    fn set_property(&mut self, name: &str, value: T, old: T) -> anyhow::Result<()>;
}

impl PropertySetter<String> for SyncRepository {
    fn set_property(&mut self, name: &str, value: String, old: String) -> anyhow::Result<()> {
        match name {
            "source" => {
                if self.source != old {
                    return Err(anyhow!("Old value mismatch {} != {}", self.source, old));
                }
                self.source = value;
            }
            "mirror" => {
                if self.mirror != old {
                    return Err(anyhow!("Old value mismatch {} != {}", self.mirror, old));
                }
                self.mirror = value;
            }
            _ => return Err(anyhow!("Invalid field name")),
        };
        Ok(())
    }
}

#[derive(Object)]
struct RequestUpdateValue {
    pub value: Value,
    pub old: Value,
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/repos", method = "get")]
    async fn index(&self) -> Json<Vec<SyncRepository>> {
        let (_, repos) = read_sync_repos();
        Json(repos)
    }

    #[oai(path = "/repo/:index/:field", method = "post")]
    async fn put(
        &self,
        index: Path<usize>,
        field: Path<String>,
        payload: Json<RequestUpdateValue>,
    ) -> Json<bool> {
        let (_, mut repos) = read_sync_repos();
        match &payload.value {
            Value::Null => todo!(),
            Value::Bool(_) => todo!(),
            Value::Number(_) => todo!(),
            Value::String(str_value) => {
                if let Value::String(old_str_value) = &payload.old {
                    repos[*index]
                        .set_property(
                            (*field).as_str(),
                            str_value.to_owned(),
                            old_str_value.to_owned(),
                        )
                        .unwrap();
                    write_sync_repos(&repos).unwrap();
                } else {
                    panic!("Invalid old value type");
                }
            }
            Value::Array(_) => todo!(),
            Value::Object(_) => todo!(),
        };

        Json(true)
    }

    #[oai(path = "/repo/:index", method = "delete")]
    async fn delete(&self, index: Path<usize>) {
        let (_, mut repos) = read_sync_repos();
        repos.remove(*index);
        write_sync_repos(&repos).unwrap();
    }

    #[oai(path = "/repo", method = "post")]
    async fn post(&self) {
        let (_, mut repos) = read_sync_repos();
        repos.push(SyncRepository {
            source: String::new(),
            mirror: String::new(),
        });
        write_sync_repos(&repos).unwrap();
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
