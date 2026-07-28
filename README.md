# 小鐘寺 · a tiny bell temple 🔔🔥

**終端裡的小鐘寺** —— walk into a little bell temple over SSH: strike the bell,
burn incense, pet the temple cat, wander down to the courtyard to see the old
tree and chat with whoever is on duty, and everyone else inside sees you move,
live.

```console
$ ssh xiaozhongsi.sh
```

```
  ☀️  午時   ● 2

++++++      ++++++
        🔥
*,              *,
*,      🐱      *,
*,              *,
*,      🌳      *,
*,    🧑🙋      *,
*,              *,
*,*,*,      *,*,*,

  🙋 在此 · 按空格搭話
```

Everyone connected at the same time shares one small temple and sees each other
move, live. Pick a face on your first visit and it is remembered by your public
key. Walk with the arrows — the view scrolls to follow you.

Stand beside or below the **bell 🔔** and press Space to strike it: everyone
inside hears it, a `\a` reaching every terminal. Walk out through the doorway
into the garden and stand below the **censer 🔥** to burn a stick of incense.
Bell and incense are counted separately, day by day. A **cat 🐱** wanders the
grounds — stand next to it and press Space to give it a pet (just for fun, not
counted); it holds still while you do.

The garden opens into a lower **courtyard** with an old **tree 🌳** (press Space
next to it for a line). Someone is usually on duty by Shanghai time — a **guard
🙋** through the day (out by the tree, in the hall in the afternoon), a **night
volunteer 💇** — stand next to them and press Space to talk; they come and go
with the hours, and nobody keeps watch at noon or deep in the night. The status
line marks the hour with the traditional twelve *shichen* (the name — 午時,
子時, … — always shown; the icon before it follows the weather), and a green
`●` with the number of players currently connected. Leave through the
courtyard's bottom gate or the openings on either side.

The status-line icon follows the **real weather** at the Big Bell Temple
(大钟寺) in Haidian, Beijing — the real-world namesake of this little temple.
Every 30 minutes the server itself polls [Open-Meteo](https://open-meteo.com/)
(no key, non-commercial) and swaps the icon: `☀️` clear, `⛅` partly cloudy,
`☁️` overcast, `🌧️` rain, `🌨️` snow, `⛈️` thunder, `🌫️` fog, `🌙` clear
night — day/night comes from the sun, not the clock. The shichen name stays.
The fetch runs off the game path (its own task, on a blocking thread, holding
no lock); if it can't reach the weather for an hour the icon falls back to
`⚡` — as if the whole planet got struck by lightning and took the API down.

Each day at 06:00 Shanghai time a **letter 📨** drops on a random patch between
the spawn point and the tree. It blocks your way; stand next to it and press
Space to read it — the text is set by the `XIAOZHONGSI_LETTER` environment
variable. Once whoever opened it rises, the letter is gone until the next day.

The scenery is ASCII two characters wide so it lines up with the two-cell emoji
standing on it. `curl xiaozhongsi.sh` just points you at SSH.

### 🎮 Controls

| Key | |
| --- | --- |
| `← →` then `Enter` | pick your avatar (first visit only) |
| arrows, `WASD` or `hjkl` | walk (the view follows you) |
| `Space` beside/below the bell 🔔 | strike the bell — heard by all |
| `Space` below the censer 🔥 | burn a stick of incense |
| `Space` next to the cat 🐱 | pet it (just for fun) |
| `Space` next to the tree 🌳 | look at the old tree |
| `Space` next to whoever's on duty 🙋/💇 | have a word |
| `Space` next to the daily letter 📨 | read it (then it's gone) |
| any key | rise again |
| walk out a courtyard edge or gate | leave the temple |
| `q` / `Ctrl-C` | leave immediately |

Everything happens inside the temple. A trailing command is ignored — you still
just connect in — but a connection with no terminal attached gets a nudge to use
one, since the temple is interactive. Visitors without an SSH key can connect
but cannot be told apart, so they are met with `ssh-keygen` instructions instead
of being let in.

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
| `XIAOZHONGSI_FAKE_MIN` | — | debug: pin the perceived Shanghai minute-of-day (0–1439), e.g. `480` = 08:00, to test the time-based NPCs |
| `XIAOZHONGSI_LETTER` | — | text of the daily 06:00 letter; unset means no letter |

Rings are counted per public-key fingerprint. Nothing on disk holds the
fingerprint itself: both the log and the remembered avatars are keyed by
`sha256(salt + fingerprint)` truncated to 16 hex chars.

### 📄 License

[MIT](LICENSE)
