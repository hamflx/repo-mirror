use std::collections::HashSet;
use std::str;
use std::{env, fs, path::Path};

use anyhow::Result;
use git2::{
    build::RepoBuilder, Cred, CredentialType, FetchOptions, PushOptions, RemoteCallbacks,
    Repository,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SyncRepository {
    pub source: String,
    pub mirror: String,
}

#[derive(Serialize, Deserialize)]
struct KnownHosts {
    pub hosts: HashSet<String>,
}

fn main() {
    let mirrors_dir = r#"D:\tmp\mirrors"#;
    fs::create_dir_all(mirrors_dir).unwrap();

    let repos: Vec<SyncRepository> = serde_json::from_str(
        str::from_utf8(&fs::read("repos.json").expect("无法读取 repos.json"))
            .expect("文件内容不是有效的 utf8 格式"),
    )
    .expect("解析 json 格式失败");
    if repos.is_empty() {
        panic!("未找到有效的仓库配置项");
    }

    let mut known_hosts = KnownHosts::load().unwrap_or_else(|_| KnownHosts::new());

    let mut clone_callbacks = RemoteCallbacks::new();
    let mut clone_opts = FetchOptions::new();
    clone_callbacks.credentials(get_credentials);
    clone_callbacks.certificate_check(|cert, host| {
        let host_key = base64::encode(cert.as_hostkey().unwrap().hash_sha256().unwrap());
        if known_hosts.check(&host_key) {
            return true;
        }

        println!(
            "Host {} key is: {}",
            host,
            base64::encode(cert.as_hostkey().unwrap().hash_sha256().unwrap()),
        );
        println!("Do you trust?");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() == "y" {
            known_hosts.push(host_key).unwrap();
            return true;
        }
        false
    });
    clone_opts.remote_callbacks(clone_callbacks);

    let mut fetch_callbacks = RemoteCallbacks::new();
    let mut fetch_opts = FetchOptions::new();
    fetch_callbacks.credentials(get_credentials);
    fetch_opts.remote_callbacks(fetch_callbacks);

    let mut push_callbacks = RemoteCallbacks::new();
    let mut push_opts = PushOptions::new();
    push_callbacks.credentials(get_credentials);
    push_opts.remote_callbacks(push_callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(clone_opts);
    builder.bare(true);

    for sync_repo in &repos {
        let repo_name = sync_repo.source.split('/').last().unwrap();
        let repo_dir_path_buf = Path::new(mirrors_dir).join(repo_name);
        let repo_dir_path = repo_dir_path_buf.as_path();
        let repo = if repo_dir_path.exists() {
            Repository::open(repo_dir_path).unwrap()
        } else {
            builder
                .clone(sync_repo.source.as_str(), repo_dir_path)
                .unwrap()
        };
        let mut remote_origin = repo.find_remote("origin").unwrap();
        let mut remote_mirror = repo
            .find_remote("mirror")
            .or_else(|_| repo.remote("mirror", sync_repo.mirror.as_str()))
            .unwrap();

        remote_origin
            .fetch(&["+refs/heads/*:refs/heads/*"], Some(&mut fetch_opts), None)
            .unwrap();

        let origin_refs = remote_origin.list().unwrap();
        let origin_heads: Vec<_> = origin_refs
            .iter()
            .filter(|s| s.name().starts_with("refs/heads/"))
            .map(|r| r.name())
            .collect();
        let push_refspecs = origin_heads
            .iter()
            .map(|s| format!("{}:{}", s, s))
            .collect::<Vec<_>>();

        remote_mirror
            .push(push_refspecs.as_slice(), Some(&mut push_opts))
            .unwrap();

        repo.remote_delete("mirror").unwrap();
    }
}

fn get_credentials(
    _url: &str,
    username_from_url: Option<&str>,
    _allowed_types: CredentialType,
) -> Result<Cred, git2::Error> {
    Cred::ssh_key(
        username_from_url.unwrap(),
        None,
        Path::new(&format!(
            "{}/.ssh/id_rsa",
            env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap()
        )),
        None,
    )
}

impl KnownHosts {
    pub fn load() -> Result<Self> {
        Ok(serde_json::from_str(str::from_utf8(&fs::read(
            "known_hosts.json",
        )?)?)?)
    }

    pub fn new() -> Self {
        KnownHosts {
            hosts: HashSet::new(),
        }
    }

    pub fn check(&self, key: &String) -> bool {
        self.hosts.contains(key)
    }

    pub fn push(&mut self, key: String) -> Result<()> {
        self.hosts.insert(key);
        let content = serde_json::to_string(self)?;
        Ok(fs::write("known_hosts.json", content)?)
    }
}
