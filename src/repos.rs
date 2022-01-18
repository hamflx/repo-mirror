use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Object)]
pub struct SyncRepository {
    pub source: String,
    pub mirror: String,
}

pub fn read_sync_repos() -> (String, Vec<SyncRepository>) {
    let temp_dir = std::env::temp_dir().join("repo_mirror");
    let mirrors_dir = temp_dir.to_str().unwrap().to_owned();
    std::fs::create_dir_all(&mirrors_dir).unwrap();
    let repos: Vec<SyncRepository> = serde_json::from_str(
        std::str::from_utf8(&std::fs::read("repos.json").expect("无法读取 repos.json"))
            .expect("文件内容不是有效的 utf8 格式"),
    )
    .expect("解析 json 格式失败");
    (mirrors_dir, repos)
}

pub fn write_sync_repos(repos: &Vec<SyncRepository>) -> Result<()> {
    std::fs::write("repos.json", serde_json::to_string(repos)?)?;
    Ok(())
}
