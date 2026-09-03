"""화면 확인용 파일(docs/screen-review.html)을 만든다.

앱의 styles.css·app.js를 **그대로** 쓰고 Tauri API만 흉내내므로, 여기서 보이는 것이 앱에서 보이는 것과 같다.
화면을 바꿀 때마다 이 스크립트를 다시 돌려 사용자에게 먼저 보여 주고 승인을 받는다 (CLAUDE.md 작업 규칙).

    python tools/make-review.py
"""
import base64
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
OUT = ROOT / "docs" / "screen-review.html"
# 파일 크기를 줄이려고 라틴 글꼴만 심는다. 한글은 시스템 글꼴로 대체된다.
FONTS = ["barlow-latin-400-normal", "barlow-latin-600-normal",
         "barlow-latin-700-normal", "barlow-condensed-latin-600-normal"]

USAGE = {
    "ok": {"source": "oauth", "status": "ok", "fetched_at": None,
           "five_hour": {"used_pct": 35, "resets_at": "2026-09-03T13:00:00Z"},
           "seven_day": {"used_pct": 9, "resets_at": "2026-09-09T22:00:00Z"},
           "model_window": {"label": "Fable", "used_pct": 14, "resets_at": "2026-09-09T22:00:00Z"}},
    "stale": {"source": "statusline", "status": "stale", "fetched_at": "2026-09-03T00:30:00Z",
              "five_hour": {"used_pct": 33, "resets_at": "2026-09-03T13:00:00Z"},
              "seven_day": {"used_pct": 8, "resets_at": "2026-09-09T22:00:00Z"},
              "model_window": None},
}

# (제목, 설명, [(라벨, 레이아웃, 상태, 배율), ...])
SECTIONS = [
    ("1 · 크기", "시안은 100% 기준입니다. 메뉴에서 100·125·150·175% 중 고를 수 있고 지금 기본은 125%입니다.",
     [("100% <b>(시안 원본)</b>", "2a", "ok", 1), ("125% <b>(현재 기본)</b>", "2a", "ok", 1.25),
      ("150%", "2a", "ok", 1.5)]),
    ("2 · 끊김 표시 — 두 안 중 골라 주십시오", "한도 영역만 회색이 되고 시스템 지표는 살아 있습니다. "
     "\"끊김\"은 빨간 굵은 글씨입니다. <b>A안</b>은 문구를 한 줄 아래로 내려 가운데에 두고, "
     "<b>B안</b>은 \"끊김\"을 윗줄로 올려 두 줄로 나눕니다.",
     [("정상 (비교용)", "2a", "ok", 1.25), ("<b>A안</b> · 한 줄 내려 가운데", "2a", "stale", 1.25, "v-a"),
      ("<b>B안</b> · 두 줄로 분리", "2a", "stale", 1.25, "v-b")]),
    ("3 · 모델별 한도", "시안의 \"OPUS\" 자리에 서버가 준 모델 이름을 넣습니다. 값이 없으면 그 칸을 숨깁니다(오른쪽).",
     [("Fable 있음", "2b", "ok", 1.25), ("없음", "2b", "stale", 1.25)]),
    ("4 · 나머지 레이아웃", "메뉴에서 고를 수 있는 일곱 가지 중 나머지입니다. 위 규칙이 모두 같게 적용됩니다.",
     [(l, l, "ok", 1.25) for l in ["2c", "2d", "2e", "2f", "2g"]]),
]

PAGE_CSS = """
body{margin:0;padding:28px 30px 60px;background:#191c21;height:auto;overflow:auto;
  font-family:"Barlow","Noto Sans KR","Malgun Gothic",sans-serif;color:#e6ebf2}
h1{font:600 20px/1.3 "Barlow","Noto Sans KR",sans-serif;margin:0 0 4px}
.sub{font:400 13px/1.6 "Barlow","Noto Sans KR",sans-serif;color:rgba(214,235,255,.55);margin:0 0 22px;max-width:70ch}
h2{font:600 13px/1 "Barlow Condensed",sans-serif;letter-spacing:.14em;text-transform:uppercase;
  color:rgba(214,235,255,.5);margin:32px 0 4px;padding-bottom:7px;border-bottom:1px solid rgba(148,188,227,.16)}
.h2note{font:400 12px/1.5 "Barlow","Noto Sans KR",sans-serif;color:rgba(214,235,255,.45);margin:0 0 16px;max-width:70ch}
.row{display:flex;flex-wrap:wrap;gap:26px 30px;align-items:flex-start}
.item{display:flex;flex-direction:column;gap:9px}
.cap{font:600 10.5px/1 "Barlow Condensed",sans-serif;letter-spacing:.12em;color:rgba(214,235,255,.42)}
.cap b{color:rgba(214,235,255,.75)}
.stage{padding:14px 16px;background:rgba(148,188,227,.045)}
.ctl{display:flex;gap:8px;margin:0 0 18px}
.ctl button{font:600 11px/1 "Barlow Condensed",sans-serif;letter-spacing:.1em;text-transform:uppercase;
  padding:7px 13px;background:transparent;color:rgba(214,235,255,.6);
  border:1px solid rgba(148,188,227,.28);cursor:pointer}
.ctl button[aria-pressed="true"]{background:#94bce3;color:#16181c;border-color:#94bce3}
/* 끊김 문구 두 안 — 승인된 쪽만 앱 CSS로 옮긴다 */
.v-a .note.rs{margin-top:16px}
.v-b .stale-tag{display:block;margin:0 0 2px}
.v-b .note.rs{margin-top:6px;line-height:1.35}
"""

RUNTIME = """
const OK=%OK%, STALE=%STALE%;
function paint(){
  document.querySelectorAll('.holder').forEach(h=>{
    state.usage = h.dataset.case==='stale' ? STALE : OK;
    state.ui.layout = h.dataset.lay;
    render();
    h.innerHTML = document.getElementById('root').innerHTML;
    const p = h.firstElementChild;
    if(document.documentElement.dataset.state==='stale') p.style.borderStyle='dashed';
    if(h.dataset.scale) p.style.zoom = h.dataset.scale;
  });
}
setTimeout(()=>{
  const emit=(n,p)=>(subs[n]||[]).forEach(f=>f({payload:p}));
  let t=0;
  const tick=()=>{t++;emit('sys:update',{cpu_pct:52+26*Math.sin(t/5),mem:{used_gb:19.6,total_gb:32},
    gpu:{name:'RTX 4090',util_pct:38+20*Math.sin(t/3),mem_used_gb:12.7,mem_total_gb:24}});};
  for(let i=0;i<40;i++)tick();
  paint(); setInterval(()=>{tick();paint();},1000);
  const b1=document.getElementById('b-dark'), b2=document.getElementById('b-light');
  const setTheme=(t)=>{document.documentElement.dataset.theme=t;
    document.body.style.background = t==='dark'?'#191c21':'#e9e9ea';
    document.body.style.color = t==='dark'?'#e6ebf2':'#1d1f20';
    document.querySelectorAll('.sub,.cap,h2,.h2note').forEach(e=>e.style.color = t==='dark'?'':'rgba(29,31,32,.55)');
    b1.setAttribute('aria-pressed', t==='dark'); b2.setAttribute('aria-pressed', t!=='dark'); paint();};
  b1.onclick=()=>setTheme('dark'); b2.onclick=()=>setTheme('light');
},60);
"""


def inlined_css() -> str:
    lines = []
    for line in (SRC / "styles.css").read_text(encoding="utf-8").splitlines():
        m = re.search(r"url\(fonts/([a-z0-9-]+)\.woff2\)", line)
        if m:
            name = m.group(1)
            if name not in FONTS:
                continue
            data = base64.b64encode((SRC / "fonts" / f"{name}.woff2").read_bytes()).decode()
            line = line.replace(f"url(fonts/{name}.woff2)", f"url(data:font/woff2;base64,{data})")
        lines.append(line)
    return "\n".join(lines)


def main() -> None:
    body = []
    for title, note, items in SECTIONS:
        body.append(f"<h2>{title}</h2>")
        body.append(f'<p class="h2note">{note}</p>')
        cells = ""
        for it in items:
            cap, lay, case, scale = it[:4]
            variant = it[4] if len(it) > 4 else ""
            cells += (f'<div class="item"><span class="cap">{cap}</span><div class="stage">'
                      f'<div class="holder {variant}" data-lay="{lay}" data-case="{case}"'
                      f' data-scale="{scale}"></div></div></div>')
        body.append(f'<div class="row">{cells}</div>')

    page = f"""<!doctype html>
<html lang="ko" data-theme="dark" data-state="ok" data-pulse="on">
<head><meta charset="utf-8"><title>Claude Usage 화면 확인</title>
<style>{inlined_css()}
{PAGE_CSS}</style></head>
<body>
<h1>Claude Usage — 지금 앱에 들어간 화면</h1>
<p class="sub">설치하지 않고 여기서 확인하실 수 있습니다. 앱과 같은 코드로 그립니다.
값은 예시입니다(5시간 35%, 주간 9%, Fable 14%).</p>
<div class="ctl"><button id="b-dark" aria-pressed="true">다크</button>
<button id="b-light" aria-pressed="false">라이트</button></div>
{"".join(body)}
<div id="root" style="position:absolute;left:-9999px"></div>
<script>const subs={{}};window.__TAURI__={{event:{{listen:(n,f)=>{{(subs[n]=subs[n]||[]).push(f);
 return Promise.resolve(()=>{{}});}}}},
 core:{{invoke:(c)=>c==="get_settings"?Promise.resolve({{layout:"2a",theme:"dark",alarm:"pulse",demo:false,scale:1}})
 :Promise.resolve()}}}};</script>
<script>{(SRC / "app.js").read_text(encoding="utf-8")}</script>
<script>{RUNTIME.replace("%OK%", json.dumps(USAGE["ok"])).replace("%STALE%", json.dumps(USAGE["stale"]))}</script>
</body></html>"""
    OUT.write_text(page, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} · {OUT.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
