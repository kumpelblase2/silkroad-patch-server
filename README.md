# skrillax-patch-server

A patch server for Silkroad Online, serving the patches in a way a normal iSro client would expect. It does _not_, 
however, emulate the gateway server in any capacity and thus is only useful in two cases:
- for private servers using (somewhat recent) iSro clients
- for local client testing of the iSro client, injecting updates through a proxy

This server is very minimal and should probably not be directly exposed to the internet, but instead should run behind a
cache and/or reverse proxy. Given that the Silkroad Client seems to require the use of port 80, it will also run on that
port by default, but can also be adjusted to run on any other port using `--port`/`-p` options.

The patch server will use a folder that contains a directory for each server version. This doesn't _need_ to match the 
client versions, but it still helps to stay organized. Inside each version directory, there should be directories for 
the individual pk2 containers, such as `media` or `data`. Files that don't belong into a container, like 
`sro_client.exe`, can be placed directly into the version directory. Individual files that belong into a container 
should have a matching patch inside the container directory inside the version directory. So a file called 
`unity_server.txt` that is inside the `Media.pk2` with the path `server_dep/silkroad/event/` should thus be placed in 
`media/server_dep/silkroad/event/unity_server.txt` inside the version directory.

An example patch directory, with multiple versions, is shown below.

```
patch-files/
├─ 595/
│  ├─ media/
│  │  ├─ server_dep/
│  │  │  ├─ silkroad/
│  │  │  │  ├─ event/
│  │  │  │  │  ├─ unity_server.txt
│  ├─ sro_client.exe
├─ 596/
│  ├─ ...
```

To create a new patch, simply create a new version directory inside the main patch directory and create the structure as
described above. The server will automatically look for new patches and include them.

__Please note__  
Because of how patching in Silkroad works, only the _latest_ version of a file will be transferred to the clients. If a 
client is on version 1.500 and the latest patch is 1.502, both patch 1.501 and 1.502 contain a file at 
`media/server_dep/silkroad/event/unity_server.txt`, the client will only get the file from the `1.502` patch. More on 
why this happens in the explanation below.

## How Silkroad Patching Works (for iSro)

To understand how patching works, we start with opening the launcher. This launcher then connects to the gateway server 
and asks for things like news, but also sends in its local version. Upon receiving the version from the launcher, the 
gateway server will then check if this version against its list. It may return with a response saying the client might 
be too old or that everything is up ot date. If the client is not too old, but also isn't completely up to date, it will
tell the launcher that it needs to update to a specific version - the current version. Additionally, the server will say
where the files can be downloaded from and which files need to be updated. Since the server responds with the current 
version, the launcher will download every file that was updated in all the patches that it missed. But if a file was 
updated multiple times in several patches, it only loads the _latest_ version of the file. The launcher thus completely 
replaces the files instead of, for example, only applying changes. The gateway server gave the launcher a http server 
URLs that will then use to download each file. One thing to note here is that the launcher _only_ accepts URLs without a
port and plain http as a protocol. Once all files have been downloaded and packed into their containers, the launcher 
will adjust its local version and restart.

## How Files are Transferred

The patching process technically is just a simple HTTP download of the updates files, but it does not download the file 
directly. It is instead first compressed using LZMA and then sent off to the client, which will decompress it before 
applying it to the local files. Interestingly, it uses the compression preset of `6` but with a version large dictionary
size (`33554432`). Apart from that, it is standard LZMA.