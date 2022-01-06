# repo-mirror

## 基础用法

Windows:

```bash
docker build -t hamflx/repo-mirror .
docker run -it --rm -v $env:USERPROFILE\.ssh:/root/.ssh:ro hamflx/repo-mirror
```

Linux:

```bash
docker build -t hamflx/repo-mirror .
docker run -it --rm -v $HOME/.ssh:/root/.ssh:ro hamflx/repo-mirror
```
