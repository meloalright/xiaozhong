//! curl xiaozhongsi.sh 是介绍页，不撞钟；撞钟只走 ssh。
//! 按 UA 分流：命令行给纯文本，浏览器给一张极简暗色落地页。

/// 直接拿 🙏🏻 当图标，SVG 里塞个 emoji 交给系统字体渲染
const FAVICON: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\">\
<text x=\"50\" y=\"52\" font-size=\"76\" text-anchor=\"middle\" dominant-baseline=\"central\">🔔</text></svg>";

fn is_terminal(user_agent: &str) -> bool {
    let ua = user_agent.to_ascii_lowercase();
    ["curl", "wget", "httpie", "fetch", "powershell"]
        .iter()
        .any(|c| ua.contains(c))
        || user_agent.is_empty()
}

/// 返回 (content_type, body)
pub fn route(path: &str, user_agent: &str) -> (u16, &'static str, String) {
    match path {
        "/" => {
            if is_terminal(user_agent) {
                (200, "text/plain; charset=utf-8", TEXT_HOME.to_string())
            } else {
                (200, "text/html; charset=utf-8", html_home())
            }
        }
        "/favicon.svg" | "/favicon.ico" => {
            (200, "image/svg+xml; charset=utf-8", FAVICON.to_string())
        }
        "/healthz" => (
            200,
            "application/json; charset=utf-8",
            "{\"ok\":true}".to_string(),
        ),
        _ => (404, "text/plain; charset=utf-8", "此路無廟\n".to_string()),
    }
}

const TEXT_HOME: &str = "終端裡的小鐘寺 · ssh xiaozhongsi.sh 進寺燒香撞鐘撸貓 🐱🔥🔔\n";

fn html_home() -> String {
    r####"<!doctype html>
<html lang="zh-Hant">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>終端裡的小鐘寺</title>
<meta name="description" content="終端裡的小鐘寺 · ssh xiaozhongsi.sh 進寺燒一炷香撞一記鐘撸一把貓 🐱🔥🔔">
<link rel="icon" href="/favicon.svg">
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; }
  body {
    margin: 0; min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 120% at 50% 0%, #2a0f0a 0%, #140807 60%);
    color: #f3e2c0; font: 15px/1.7 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    padding: 2rem;
  }
  main { width: 100%; max-width: 640px; }
  h1 { font-size: 2.25rem; letter-spacing: .08em; margin: 0 0 1.25rem;
       color: #e8b923; }
  .lead { color: #b08a6a; margin: 0 0 .25rem; font-size: .9375rem; }
  .cmd {
    display: block; margin: .5rem 0 1.5rem; padding: .8rem 1rem;
    background: #241210; border: 1px solid #4a2a22; border-radius: 8px;
    color: #f3e2c0; overflow-x: auto; white-space: pre-wrap; word-break: break-all;
  }
  .cmd b { color: #74c0c8; font-weight: normal; }
  .cmd a { color: #74c0c8; }
  /* 提示符只做装饰，选中复制时不带上它 */
  .prompt { color: #8a7a63; user-select: none; -webkit-user-select: none; }
  a { color: #e8b923; }
  /* GitHub 按钮固定在页面右上角 */
  .corner { position: fixed; top: 1rem; right: 1rem; }
  /* buttons.js 加载前先藏住 Star 文本，避免闪一下；
     加载后原 <a> 被替换成 iframe（不带此 class）自然显示 */
  .github-button { visibility: hidden; }
  /* 透明无背景的复制按钮，缓慢闪烁提示可点；点后变对勾 */
  .copy {
    display: inline-flex; align-items: center; vertical-align: middle;
    margin-left: .6rem; padding: 0; border: 0; background: none;
    color: #74c0c8; cursor: pointer; -webkit-appearance: none; appearance: none;
    -webkit-animation: blink 1.8s ease-in-out infinite;
    animation: blink 1.8s ease-in-out infinite;
  }
  .copy:hover { color: #a7dfe4; }
  .copy svg { width: 1.05em; height: 1.05em; display: block; }
  /* 复制成功：停止闪烁、转绿、显示对勾 */
  .copy.done { color: #7bbf6a; -webkit-animation: none; animation: none; }
  /* 命令下方那行「已复制」小字，默认透明，复制后淡入 */
  .copied {
    height: 1.1em; margin: -1.1rem 0 1.5rem; font-size: .8125rem;
    color: #7bbf6a; opacity: 0; transition: opacity .2s;
    user-select: none; -webkit-user-select: none;
  }
  .copied.show { opacity: 1; }
  @-webkit-keyframes blink { 0%,100% { opacity: 1; } 50% { opacity: .3; } }
  @keyframes blink { 0%,100% { opacity: 1; } 50% { opacity: .3; } }
</style>
</head>
<body>
  <!-- GitHub 官方 star 按钮，固定右上角，异步加载 -->
  <div class="corner">
    <a class="github-button" href="https://github.com/meloalright/xiaozhongsi"
       data-icon="octicon-star" data-show-count="true"
       aria-label="Star meloalright/xiaozhongsi on GitHub">Star</a>
    <noscript><a href="https://github.com/meloalright/xiaozhongsi">GitHub</a></noscript>
  </div>
  <script async defer src="https://buttons.github.io/buttons.js"></script>
<main>
  <h1>終端裡的小鐘寺 🔔</h1>
  <p class="lead">在終端運行如下命令 · 進寺燒香撞鐘撸貓</p>
  <code class="cmd"><span class="prompt">$ </span><span id="line"><b>ssh</b> xiaozhongsi.sh</span><button id="copy" class="copy" type="button" aria-label="複製指令"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg></button></code>
  <p class="copied" id="copied" aria-live="polite">已複製到剪貼簿</p>
</main>
<script>
(function(){
  var COPY='<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';
  var CHECK='<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"></path></svg>';
  var btn=document.getElementById('copy'), msg=document.getElementById('copied'), t;
  function flash(){
    btn.innerHTML=CHECK; btn.classList.add('done'); msg.classList.add('show');
    clearTimeout(t);
    t=setTimeout(function(){ btn.innerHTML=COPY; btn.classList.remove('done'); msg.classList.remove('show'); }, 1800);
  }
  btn.addEventListener('click', function(){
    var text='ssh xiaozhongsi.sh';
    if(navigator.clipboard&&navigator.clipboard.writeText){
      navigator.clipboard.writeText(text).then(flash, fallback);
    } else { fallback(); }
    function fallback(){
      var ta=document.createElement('textarea'); ta.value=text; ta.setAttribute('readonly','');
      ta.style.position='absolute'; ta.style.left='-9999px'; document.body.appendChild(ta);
      ta.select(); try{ document.execCommand('copy'); }catch(e){} document.body.removeChild(ta); flash();
    }
  });
})();
</script>
</body>
</html>
"####
    .to_string()
}
