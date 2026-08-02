# 小鐘寺 · a tiny bell temple 🔔🔥

**終端裡的小鐘寺** —— walk into a little bell temple over SSH: strike the bell,
burn incense and hear the day's letter, pet the temple cat, wander down to the
courtyard to see the old tree, pray beside other pilgrims, shove the shopping
cart around, and watch the status bar track the real weather and clock at the
Big Bell Temple in Beijing — while everyone else inside sees you move, live.
Inspired by [mpiorowski/late-sh](https://github.com/mpiorowski/late-sh).

```console
$ ssh xiaozhongsi.sh
```

```
☀️  14:23  寺中 2 人

        🔔
++      🧑      ++
++              ++
++++++      ++++++
        🔥
*,      🐱      *,
*,      🌳      *,
*,          👧  *,
*,*,*,      *,*,*,

  🔔 你是今天第 3 位撞鐘
```

Everyone connected at the same time shares one small temple and sees each other
move, live. Pick a face on your first visit and it is remembered by your public
key. Walk with the arrows — the view scrolls to follow you.

Stand beside or below the **bell 🔔** and press Space to strike it: everyone
inside hears it, a `\a` reaching every terminal. Once it fades, press a key and
you murmur the day's **letter** (the `XIAOZHONGSI_LETTER` environment variable)
to yourself as `{you} 「…」`; press once more to rise. Walk out through the
doorway into the garden and stand below the **censer 🔥** to burn a stick of
incense. Bell and incense are counted separately, day by day. A **cat 🐱**
wanders the grounds — stand next to it and press Space to give it a pet (just
for fun, not counted); it holds still while you do.

The garden opens into a lower **courtyard** with an old **tree 🌳** (press Space
next to it for a line). Stand next to **another player** and press Space to
**pray** — you alone turn into a sparkle `✨` and see the line `你進行了祈禱 🙏`
until you press a key to rise; the other player is never disturbed. Deep in the
night (00:00–02:00) a lone **ghost 👻** drifts to the right of the bell — talk to
it and it only sighs `......`. The status line shows the current Shanghai time as
`HH:MM` (the icon before it follows the weather) and `寺中 N 人` — how many
players are connected. Leave through the courtyard's bottom gate or the openings
on either side.

Some days a **shopping cart 🛒** turns up on a random patch between the spawn
point and the tree. It has a body of its own — it won't overlap people, the
tree or a wall. Press Space beside it and it tells you it can be pushed: stand
on one side and press the arrow toward it to shove both yourself and the cart
that way. Shove it into anything solid — a wall, the censer, the tree, another
person — and it doesn't just stop; it skids off perpendicular to the hit,
preferring whichever side points **further from an exit** so it stays inside (and
if the only open way to skid is where the **cat 🐱** is standing, it shoves the
cat aside as it goes). Shove it into the cat head-on and the cat is pushed along
by the same rules: it slides ahead if there's room, skids off perpendicular
(away from the nearest exit) when it hits a wall / person / tree / censer / the
hall's admin wall, and — like the cart — gets sent back inside if it's shoved off
the map. The cart is not allowed into
the main hall (an invisible admin wall bounces it back at the doorway, with a
faint gray note reminding you the courtyard cart stays outdoors). Where a
cart or cat that's shoved out of the temple reappears depends on which exit it
went through: out one of the **openings beside the censer 🔥** and it lands two
cells below the **tree 🌳** (or, if that cell is taken — say the cat is on it —
a cell right next to it); out the **gate below the tree** and it lands on a
random free cell around the censer's flanks, three cells out to its left or right.

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
| any key after the bell | murmur the day's letter as `{you} 「…」`, then a key to rise |
| `Space` below the censer 🔥 | burn a stick of incense |
| `Space` next to the cat 🐱 | pet it (just for fun) |
| `Space` next to the tree 🌳 | look at the old tree |
| `Space` next to another player | pray 🙏 — you alone become ✨ until you rise |
| `Space` next to the night ghost 👻 | it only sighs `......` |
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
| `XIAOZHONGSI_MAX_HANDSHAKES` | `128` | a flood shield: the most SSH handshakes allowed in flight at once. Keep it above `XIAOZHONGSI_MAX_SESSIONS` (see below) |
| `XIAOZHONGSI_HOST_KEY` | `host_key` | OpenSSH host key path (generated on first run if absent) |
| `XIAOZHONGSI_HOST_KEY_PEM` | — | host key as inline PEM; overrides the file, handy for stateless containers |
| `XIAOZHONGSI_LETTER` | — | today's letter; right after you strike the bell, pressing a key makes you murmur it to yourself as `{you} 「…」` before another key lets you rise. Unset means striking the bell just rises |
| `XIAOZHONGSI_FAKE_MIN` | — | debug: pin the perceived Shanghai minute-of-day (0–1439), e.g. `480` = 08:00, to test the time-based NPCs. Also turns off the live weather poll and the daily cart so demos stay deterministic |
| `XIAOZHONGSI_FAKE_WEATHER` | — | debug: `is_day,code,cloud,rain` (e.g. `1,3,90,0` = overcast day) run through the same mapping to pin the status-bar weather icon |
| `XIAOZHONGSI_FAKE_CART` | — | debug: pin the shopping cart at `r,c` instead of the random daily spot |
| `XIAOZHONGSI_FAKE_CAT` | — | debug: pin the cat at `r,c` and stop it wandering |

Rings are counted per public-key fingerprint. Nothing on disk holds the
fingerprint itself: both the log and the remembered avatars are keyed by
`sha256(salt + fingerprint)` truncated to 16 hex chars.

### 🌊 Full temple, and a flood shield

Being "full" happens in two tiers. A visitor's slot is claimed only once they
actually enter the hall, *after* the SSH handshake — so when the hall already
holds `XIAOZHONGSI_MAX_SESSIONS` people, the next one still completes a full
handshake and is then met with the polite `寺中擁擠 · 香客已滿 · 稍後再來 🔔`
line before disconnecting.

That handshake is a real curve25519 key exchange, so a burst of connections
costs CPU even when everyone is turned away. `XIAOZHONGSI_MAX_HANDSHAKES`
(default 128, released the instant a handshake finishes) caps how many run at
once. Under a flood the excess connections are dropped *before* the key
exchange — no crypto spent — after a cheap plaintext banner
(`寺中擁擠 · 稍後再來 🔔`) written ahead of the SSH version string. openssh logs
such pre-version lines at debug level, so a plain client only sees
`Connection closed by remote host`; the banner shows up under `ssh -v` and to
raw/library clients. Delivering a visible message here would require finishing
the handshake — exactly the cost the shield exists to avoid.

### 🩺 Probes

Two GitHub Actions workflows keep an eye on the live temple. Each connects over
real SSH once an hour with a fixed probe key — so the stats count them as one
returning pilgrim — and first checks the host-key fingerprint hasn't changed:

- **bell-probe** (`.github/workflows/probe.yml`, at `:01`) walks to the bell,
  strikes it, hears the day's letter, and leaves.
- **incense-probe** (`.github/workflows/incense.yml`, at `:02`) walks to the
  censer, burns a stick of incense, and leaves.

They are separate workflows, staggered a minute apart, so one can go red without
masking the other.

### 📄 License

[MIT](LICENSE)
