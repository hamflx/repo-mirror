# repo-mirror

## 基础用法

先运行一次程序，生成信任的主机文件配置 `known_hosts.json`，然后再次运行该程序，将 `known_hosts.json` 传入即可，Windows 系统中使用 `powershell` 运行如下命令：

```powershell
docker build -t hamflx/repo-mirror .
docker run -it --rm -v $env:USERPROFILE\.ssh:/root/.ssh:ro hamflx/repo-mirror -t -p -s >known_hosts.json
docker run -it --rm -v $env:USERPROFILE\.ssh:/root/.ssh:ro -v $PWD\known_hosts.json:/app/known_hosts.json -p 5000:5000 hamflx/repo-mirror --server
```
