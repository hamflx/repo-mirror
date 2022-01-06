use std::collections::HashSet;
use std::time::Duration;
use std::{env, fs, path::Path};
use std::{str, thread};

use anyhow::{anyhow, Result};
use env_logger::Env;
use git2::{
    build::RepoBuilder, Cred, CredentialType, FetchOptions, PushOptions, RemoteCallbacks,
    Repository,
};
use log::{info, trace, warn};
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
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let (mirrors_dir, repos) = read_sync_repos();
    let known_hosts = KnownHosts::load().unwrap_or_else(|_| KnownHosts::new());
    let (mut fetch_opts, mut push_opts, mut builder) = new_git_network_opts(known_hosts);

    loop {
        if let Err(err) = do_sync(
            &repos,
            mirrors_dir.as_str(),
            &mut builder,
            &mut fetch_opts,
            &mut push_opts,
        ) {
            warn!("An error occurred: {}", err);
        }

        info!("waiting for next tick");
        thread::sleep(Duration::from_secs(30 * 60));
    }
}

fn read_sync_repos() -> (String, Vec<SyncRepository>) {
    let temp_dir = std::env::temp_dir().join("repo_mirror");
    let mirrors_dir = temp_dir.to_str().unwrap().to_owned();
    fs::create_dir_all(&mirrors_dir).unwrap();
    let repos: Vec<SyncRepository> = serde_json::from_str(
        str::from_utf8(&fs::read("repos.json").expect("无法读取 repos.json"))
            .expect("文件内容不是有效的 utf8 格式"),
    )
    .expect("解析 json 格式失败");
    if repos.is_empty() {
        panic!("未找到有效的仓库配置项");
    }
    (mirrors_dir, repos)
}

fn new_git_network_opts<'cb>(
    mut known_hosts: KnownHosts,
) -> (FetchOptions<'cb>, PushOptions<'cb>, RepoBuilder<'cb>) {
    let mut clone_callbacks = RemoteCallbacks::new();
    let mut clone_opts = FetchOptions::new();
    clone_callbacks.credentials(get_credentials);
    clone_callbacks.certificate_check(move |cert, host| {
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

    (fetch_opts, push_opts, builder)
}

fn do_sync(
    repos: &Vec<SyncRepository>,
    mirrors_dir: &str,
    builder: &mut RepoBuilder,
    fetch_opts: &mut FetchOptions,
    push_opts: &mut PushOptions,
) -> Result<()> {
    for sync_repo in repos {
        let repo_name = sync_repo
            .source
            .split('/')
            .last()
            .ok_or_else(|| anyhow!("仓库地址应为非空字符串"))?;
        info!("syncing {}", repo_name);

        let repo_dir_path_buf = Path::new(mirrors_dir).join(repo_name);
        let repo_dir_path = repo_dir_path_buf.as_path();
        let repo = if repo_dir_path.exists() {
            Repository::open(repo_dir_path)?
        } else {
            (*builder).clone(sync_repo.source.as_str(), repo_dir_path)?
        };
        let mut remote_origin = repo.find_remote("origin")?;
        let mut remote_mirror = repo
            .find_remote("mirror")
            .or_else(|_| repo.remote("mirror", sync_repo.mirror.as_str()))?;

        remote_origin.fetch(&["+refs/heads/*:refs/heads/*"], Some(fetch_opts), None)?;

        let origin_refs = remote_origin.list()?;
        let origin_heads: Vec<_> = origin_refs
            .iter()
            .filter(|s| s.name().starts_with("refs/heads/"))
            .map(|r| r.name())
            .collect();
        let push_refspecs = origin_heads
            .iter()
            .map(|s| format!("+{}:{}", s, s))
            .collect::<Vec<_>>();

        trace!("Pushing refs `{:?}`", push_refspecs);
        remote_mirror.push(push_refspecs.as_slice(), Some(push_opts))?;

        repo.remote_delete("mirror")?;
    }

    Ok(())
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
