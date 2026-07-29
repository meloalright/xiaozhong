# 小鐘寺 · a tiny bell temple 🔔🔥

**終端裡的小鐘寺** —— walk into a little bell temple over SSH: strike the bell,
burn incense, pet the temple cat, wander down to the courtyard to see the old
tree, chat with whoever is on duty, shove the shopping cart around, and watch
the status bar track the real weather and clock at the Big Bell Temple in
Beijing — while everyone else inside sees you move, live.

```console
$ ssh xiaozhongsi.sh
```

```
  ☀️  14:23   ● 2

++++++      ++++++
        🔥
*,              *,
*,      🐱      *,
*,              *,
*,      🌳      *,
*,    🧑🙋      *,
*,              *,
*,*,*,      *,*,*,

  🙋 · 按空格搭話
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
volunteer 💇** — stand next to them and press Space to talk. If a letter is set
for the day (the `XIAOZHONGSI_LETTER` environment variable), the on-duty staff
lead with it first, so a full chat takes two presses: the first line is the
day's letter, the second is their own greeting. They come and go with the
hours, and nobody keeps watch at noon or deep in the night. The status
line shows the current Shanghai time as `HH:MM` (the icon before it follows the
weather) and a green `●` with the number of players currently connected. Leave
through the courtyard's bottom gate or the openings on either side.

Some days a **shopping cart 🛒** turns up on a random patch between the spawn
point and the tree. It has a body of its own — it won't overlap the cat,
people, the tree or a wall. Press Space beside it and it tells you it
can be pushed: stand on one side and press the arrow toward it to shove both
yourself and the cart that way. Shove it into anything solid — a wall, the
censer, the tree, the cat, another person — and it doesn't just stop; it skids
off at random perpendicular to the hit. It is not allowed into the main hall
(an invisible admin wall bounces it back at the doorway), and if you manage to
shove it out of the temple it reappears at a random spot back in the courtyard.

The status-line icon follows the **real weather** at the Big Bell Temple
(大钟寺) in Haidian, Beijing — the real-world namesake of this little temple.
Every 30 minutes the server itself polls [Open-Meteo](https://open-meteo.com/)
(no key, non-commercial) and swaps the icon: `☀️` clear, `⛅` partly cloudy,
`☁️` overcast, `🌧️` rain, `🌨️` snow, `⛈️` thunder, `🌫️` fog, `🌙` clear
night — day/night comes from the sun, not the clock. The `HH:MM` time stays.
The fetch runs off the game path (its own task, on a blocking thread, holding
no lock); if it can't reach the weather for an hour the icon falls back to
`⚡` — as if the whole planet got struck by lightning and took the API down.

The scenery is ASCII two characters wide so it lines up with the two-cell emoji
standing on it. `curl xiaozhongsi.sh` just points you at SSH.

### 🎮 Controls

| Key | |
| --- | --- |
| `← →` then `Enter` | pick your avatar (first visit only) |
| arrows or `WASD` | walk (the view follows you) |
| `Space` beside/below the bell 🔔 | strike the bell — heard by all |
| `Space` below the censer 🔥 | burn a stick of incense |
| `Space` next to the cat 🐱 | pet it (just for fun) |
| `Space` next to the tree 🌳 | look at the old tree |
| `Space` next to whoever's on duty 🙋/💇 | have a word |
| `Space` next to the cart 🛒 | learn you can push it |
| arrow toward the cart 🛒 | push it (and yourself) that way; it bounces off walls |
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
| `XIAOZHONGSI_MAX_SESSIONS` | `32` | people allowed inside at once; beyond it a new visitor is met with a "temple is full, come back later" line and disconnected |
| `XIAOZHONGSI_HOST_KEY` | `host_key` | OpenSSH host key path (generated on first run if absent) |
| `XIAOZHONGSI_HOST_KEY_PEM` | — | host key as inline PEM; overrides the file, handy for stateless containers |
| `XIAOZHONGSI_LETTER` | — | today's letter; the on-duty staff say it first when talked to. Unset means no letter |
| `XIAOZHONGSI_FAKE_MIN` | — | debug: pin the perceived Shanghai minute-of-day (0–1439), e.g. `480` = 08:00, to test the time-based NPCs. Also turns off the live weather poll and the daily cart so demos stay deterministic |
| `XIAOZHONGSI_FAKE_WEATHER` | — | debug: `is_day,code,cloud,rain` (e.g. `1,3,90,0` = overcast day) run through the same mapping to pin the status-bar weather icon |
| `XIAOZHONGSI_FAKE_CART` | — | debug: pin the shopping cart at `r,c` instead of the random daily spot |
| `XIAOZHONGSI_FAKE_CAT` | — | debug: pin the cat at `r,c` and stop it wandering |

Rings are counted per public-key fingerprint. Nothing on disk holds the
fingerprint itself: both the log and the remembered avatars are keyed by
`sha256(salt + fingerprint)` truncated to 16 hex chars.

### 📄 License

[MIT](LICENSE)
