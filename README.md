# 小鐘寺 · a tiny bell temple 🔔🔥

**終端裡的小鐘寺** —— walk into a little bell temple over SSH: strike the bell,
step out to the garden to burn a stick of incense, and everyone else inside
sees you move, live.

```console
$ ssh xiaozhongsi.sh
```

```
        🔔
++              ++
++      🧑      ++
++              ++
++++++      ++++++
        🔥
*,              *,
*,          👧  *,
*,              *,
*,*,*,      *,*,*,

  🔥 香爐在前 · 按空格燒香
  寺中此刻 2 人
```

Everyone connected at the same time shares one small temple and sees each other
move, live. Pick a face on your first visit and it is remembered by your public
key. Walk with the arrows — the view scrolls to follow you.

Stand beside or below the **bell 🔔** and press Space to strike it: everyone
inside hears it, a `\a` reaching every terminal. Walk out through the doorway
into the garden and stand below the **censer 🔥** to burn a stick of incense.
Bell and incense are counted separately, day by day. Leave through the garden's
flower gate at the bottom or the openings on either side.

The scenery is ASCII two characters wide so it lines up with the two-cell emoji
standing on it. `curl xiaozhongsi.sh` just points you at SSH.

### 🎮 Controls

| Key | |
| --- | --- |
| `← →` then `Enter` | pick your avatar (first visit only) |
| arrows, `WASD` or `hjkl` | walk (the view follows you) |
| `Space` beside/below the bell 🔔 | strike the bell — heard by all |
| `Space` below the censer 🔥 | burn a stick of incense |
| any key | rise again |
| walk out a garden edge or gate | leave the temple |
| `q` / `Ctrl-C` | leave immediately |

Ringing happens in the temple and nowhere else. A connection that carries a
command, or has no terminal attached, is turned away rather than counted — you
have to come in and strike the bell yourself. Visitors without an SSH key can
connect but cannot be told apart, so they are met with `ssh-keygen`
instructions instead of being let in.

### 🧪 Test

```shell
cargo test
```

### ▶️ Run

```shell
cargo run --release
```

Configuration is via environment variables:

| Variable | Default | Description |
| --- | --- | --- |
| `SSH_PORT` | `2222` | SSH listen port(s), comma-separated for multiple |
| `HTTP_PORT` | `8080` | HTTP listen port |
| `XIAOZHONGSI_DATA_DIR` | `data` | where the log and chosen avatars are written |
| `XIAOZHONGSI_SALT` | `xiaozhongsi` | hashing salt — change it in production |
| `XIAOZHONGSI_MAX_SESSIONS` | `128` | concurrent SSH sessions before new ones are refused |
| `XIAOZHONGSI_HOST_KEY` | `host_key` | OpenSSH host key path (generated on first run if absent) |
| `XIAOZHONGSI_HOST_KEY_PEM` | — | host key as inline PEM; overrides the file, handy for stateless containers |

Rings are counted per public-key fingerprint. Nothing on disk holds the
fingerprint itself: both the log and the remembered avatars are keyed by
`sha256(salt + fingerprint)` truncated to 16 hex chars.

### 📄 License

[MIT](LICENSE)
