mod repos;
mod server;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use std::{env, fs, path::Path};
use std::{str, thread};

use anyhow::{anyhow, Result};
use clap::Parser;
use git2::Remote;
use git2::{
    build::RepoBuilder, Cred, CredentialType, FetchOptions, PushOptions, RemoteCallbacks,
    Repository,
};
use serde::{Deserialize, Serialize};
use tracing::{info, trace, warn};

#[derive(Serialize, Deserialize)]
struct KnownHosts {
    pub hosts: HashMap<String, String>,
}

#[derive(Parser)]
struct Cli {
    #[clap(short, long)]
    trust: bool,

    #[clap(short, long)]
    print: bool,

    #[clap(short, long)]
    silence: bool,

    #[clap(long)]
    server: bool,

    #[clap(long)]
    only_server: bool,

    #[clap(long)]
    bare: bool,
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    let (mirrors_dir, repos) = repos::read_sync_repos();
    let known_hosts = KnownHosts::load().unwrap_or_else(|err| {
        info!("Failed to loading known_hosts.json, using empty. {}", err);
        KnownHosts::new()
    });
    let known_hosts_mutex = Mutex::new(known_hosts);
    let (mut fetch_opts, mut push_opts, mut builder) =
        new_git_network_opts(&known_hosts_mutex, args.trust, args.bare);

    if args.trust {
        if repos.is_empty() {
            panic!("未找到有效的仓库配置项");
        }
        let remotes = repos
            .iter()
            .fold(Vec::new(), |mut list, item| {
                let source_url = item.source.split(':').next().unwrap();
                list.push((source_url, item.source.as_str()));
                if let Some(mirror) = &item.mirror {
                    let mirror_url = mirror.split(':').next().unwrap();
                    list.push((mirror_url, mirror.as_str()));
                }
                list
            })
            .into_iter()
            .collect::<HashMap<_, _>>();
        for (_, remote_url) in remotes {
            let auth_cb = new_auth_callbacks(&known_hosts_mutex, true);
            let mut remote = Remote::create_detached(remote_url).unwrap();
            remote
                .connect_auth(git2::Direction::Fetch, Some(auth_cb), None)
                .unwrap();
        }
        if args.print {
            let kh = known_hosts_mutex.lock().unwrap();
            println!("{}", kh.serialize());
        }
        return;
    }

    if args.server {
        let server = tokio::task::spawn(async {
            let server = server::RepoMirrorConfigServer::new();
            server.run().await.unwrap();
        });
        if args.only_server {
            server.await.unwrap();
            return;
        }
    }

    loop {
        if repos.is_empty() {
            panic!("未找到有效的仓库配置项");
        }
        if let Err(err) = do_sync(
            &repos,
            mirrors_dir.as_str(),
            &mut builder,
            &mut fetch_opts,
            &mut push_opts,
            Duration::from_secs(60),
            !args.bare,
        ) {
            warn!("An error occurred: {}", err);
        }

        info!("Waiting for next tick");
        thread::sleep(Duration::from_secs(3 * 60 * 60));
    }
}

fn new_auth_callbacks(known_hosts: &Mutex<KnownHosts>, always_trust: bool) -> RemoteCallbacks {
    let mut clone_callbacks = RemoteCallbacks::new();
    clone_callbacks.credentials(get_credentials);
    clone_callbacks.certificate_check(move |cert, host| {
        let host_key = base64::encode(cert.as_hostkey().unwrap().hash_sha256().unwrap());
        let mut kh = known_hosts.lock().unwrap();
        if always_trust {
            kh.push(host.to_string(), host_key).unwrap();
            return true;
        }
        if kh.check(&host.to_string(), &host_key) {
            return true;
        }

        println!("Host {} key is: {}", host, host_key);
        println!("Do you trust?");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() == "y" {
            kh.push(host.to_string(), host_key).unwrap();
            return true;
        }
        false
    });
    clone_callbacks
}

fn new_git_network_opts(
    known_hosts: &Mutex<KnownHosts>,
    always_trust: bool,
    bare: bool,
) -> (FetchOptions, PushOptions, RepoBuilder) {
    let clone_callbacks = new_auth_callbacks(known_hosts, always_trust);
    let mut clone_opts = FetchOptions::new();
    clone_opts.remote_callbacks(clone_callbacks);

    let fetch_callbacks = new_auth_callbacks(known_hosts, always_trust);
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(fetch_callbacks);

    let mut push_callbacks = RemoteCallbacks::new();
    let mut push_opts = PushOptions::new();
    push_callbacks.credentials(get_credentials);
    push_opts.remote_callbacks(push_callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(clone_opts);
    builder.bare(bare);

    (fetch_opts, push_opts, builder)
}

fn do_sync(
    repos: &[repos::SyncRepository],
    mirrors_dir: &str,
    builder: &mut RepoBuilder,
    fetch_opts: &mut FetchOptions,
    push_opts: &mut PushOptions,
    duration: Duration,
    remove_trailing_git: bool,
) -> Result<()> {
    for sync_repo in repos {
        let repo_name = sync_repo
            .source
            .split('/')
            .last()
            .ok_or_else(|| anyhow!("仓库地址应为非空字符串"))?;
        let repo_name = if remove_trailing_git && repo_name.ends_with(".git") {
            repo_name.get(0..(repo_name.len() - 4)).unwrap_or(repo_name)
        } else {
            repo_name
        };
        info!("[{}] Start syncing ...", repo_name);

        let repo_dir_path_buf = Path::new(mirrors_dir).join(repo_name);
        let repo_dir_path = repo_dir_path_buf.as_path();
        let repo = if repo_dir_path.exists() {
            Repository::open(repo_dir_path)?
        } else {
            (*builder).clone(sync_repo.source.as_str(), repo_dir_path)?
        };
        let mut remote_origin = repo.find_remote("origin")?;

        trace!("Fetching refs latest for {}", repo_name);
        remote_origin.fetch(&["+refs/heads/*:refs/heads/*"], Some(fetch_opts), None)?;

        if let Some(mirror) = sync_repo.mirror.as_ref() {
            let origin_refs = remote_origin.list()?;
            let origin_heads: Vec<_> = origin_refs
                .iter()
                .filter(|s| s.name().starts_with("refs/heads/"))
                .map(|r| r.name())
                .collect();
            let mut remote_mirror = repo
                .find_remote("mirror")
                .or_else(|_| repo.remote("mirror", mirror))?;
            let push_refspecs = origin_heads
                .iter()
                .map(|s| format!("+{}:{}", s, s))
                .collect::<Vec<_>>();

            trace!("Pushing refs `{:?}`", push_refspecs);
            remote_mirror.push(push_refspecs.as_slice(), Some(push_opts))?;

            repo.remote_delete("mirror")?;
        }

        info!("[{}] Done.", repo_name);

        if !duration.is_zero() {
            thread::sleep(duration);
        }
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
            hosts: HashMap::new(),
        }
    }

    pub fn check(&self, host: &str, key: &str) -> bool {
        self.hosts.get(host).map(|s| s.as_str()) == Some(key)
    }

    pub fn push(&mut self, host: String, key: String) -> Result<()> {
        info!("Trust host {}:{}", host, key);
        self.hosts.insert(host, key);
        let content = serde_json::to_string(self)?;
        Ok(fs::write("known_hosts.json", content)?)
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
