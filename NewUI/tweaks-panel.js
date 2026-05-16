const __TWEAKS_STYLE=`
  .twk-panel{position:fixed;right:16px;bottom:16px;z-index:2147483646;width:280px;
    max-height:calc(100vh - 32px);display:flex;flex-direction:column;
    transform:scale(var(--dc-inv-zoom,1));transform-origin:bottom right;
    background:rgba(250,249,247,.78);color:#29261b;
    -webkit-backdrop-filter:blur(24px) saturate(160%);backdrop-filter:blur(24px) saturate(160%);
    border:.5px solid rgba(255,255,255,.6);border-radius:14px;
    box-shadow:0 1px 0 rgba(255,255,255,.5) inset,0 12px 40px rgba(0,0,0,.18);
    font:11.5px/1.4 ui-sans-serif,system-ui,-apple-system,sans-serif;overflow:hidden}
  .twk-hd{display:flex;align-items:center;justify-content:space-between;
    padding:10px 8px 10px 14px;cursor:move;user-select:none}
  .twk-hd b{font-size:12px;font-weight:600;letter-spacing:.01em}
  .twk-x{appearance:none;border:0;background:transparent;color:rgba(41,38,27,.55);
    width:22px;height:22px;border-radius:6px;cursor:default;font-size:13px;line-height:1}
  .twk-x:hover{background:rgba(0,0,0,.06);color:#29261b}
  .twk-body{padding:2px 14px 14px;display:flex;flex-direction:column;gap:10px;
    overflow-y:auto;overflow-x:hidden;min-height:0;
    scrollbar-width:thin;scrollbar-color:rgba(0,0,0,.15) transparent}
  .twk-body::-webkit-scrollbar{width:8px}
  .twk-body::-webkit-scrollbar-track{background:transparent;margin:2px}
  .twk-body::-webkit-scrollbar-thumb{background:rgba(0,0,0,.15);border-radius:4px;
    border:2px solid transparent;background-clip:content-box}
  .twk-body::-webkit-scrollbar-thumb:hover{background:rgba(0,0,0,.25);
    border:2px solid transparent;background-clip:content-box}
  .twk-row{display:flex;flex-direction:column;gap:5px}
  .twk-row-h{flex-direction:row;align-items:center;justify-content:space-between;gap:10px}
  .twk-lbl{display:flex;justify-content:space-between;align-items:baseline;
    color:rgba(41,38,27,.72)}
  .twk-lbl>span:first-child{font-weight:500}
  .twk-val{color:rgba(41,38,27,.5);font-variant-numeric:tabular-nums}

  .twk-sect{font-size:10px;font-weight:600;letter-spacing:.06em;text-transform:uppercase;
    color:rgba(41,38,27,.45);padding:10px 0 0}
  .twk-sect:first-child{padding-top:0}

  .twk-field{appearance:none;box-sizing:border-box;width:100%;min-width:0;height:26px;padding:0 8px;
    border:.5px solid rgba(0,0,0,.1);border-radius:7px;
    background:rgba(255,255,255,.6);color:inherit;font:inherit;outline:none}
  .twk-field:focus{border-color:rgba(0,0,0,.25);background:rgba(255,255,255,.85)}
  select.twk-field{padding-right:22px;
    background-image:url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'><path fill='rgba(0,0,0,.5)' d='M0 0h10L5 6z'/></svg>");
    background-repeat:no-repeat;background-position:right 8px center}

  .twk-slider{appearance:none;-webkit-appearance:none;width:100%;height:4px;margin:6px 0;
    border-radius:999px;background:rgba(0,0,0,.12);outline:none}
  .twk-slider::-webkit-slider-thumb{-webkit-appearance:none;appearance:none;
    width:14px;height:14px;border-radius:50%;background:#fff;
    border:.5px solid rgba(0,0,0,.12);box-shadow:0 1px 3px rgba(0,0,0,.2);cursor:default}
  .twk-slider::-moz-range-thumb{width:14px;height:14px;border-radius:50%;
    background:#fff;border:.5px solid rgba(0,0,0,.12);box-shadow:0 1px 3px rgba(0,0,0,.2);cursor:default}

  .twk-seg{position:relative;display:flex;padding:2px;border-radius:8px;
    background:rgba(0,0,0,.06);user-select:none}
  .twk-seg-thumb{position:absolute;top:2px;bottom:2px;border-radius:6px;
    background:rgba(255,255,255,.9);box-shadow:0 1px 2px rgba(0,0,0,.12);
    transition:left .15s cubic-bezier(.3,.7,.4,1),width .15s}
  .twk-seg.dragging .twk-seg-thumb{transition:none}
  .twk-seg button{appearance:none;position:relative;z-index:1;flex:1;border:0;
    background:transparent;color:inherit;font:inherit;font-weight:500;min-height:22px;
    border-radius:6px;cursor:default;padding:4px 6px;line-height:1.2;
    overflow-wrap:anywhere}

  .twk-toggle{position:relative;width:32px;height:18px;border:0;border-radius:999px;
    background:rgba(0,0,0,.15);transition:background .15s;cursor:default;padding:0}
  .twk-toggle[data-on="1"]{background:#34c759}
  .twk-toggle i{position:absolute;top:2px;left:2px;width:14px;height:14px;border-radius:50%;
    background:#fff;box-shadow:0 1px 2px rgba(0,0,0,.25);transition:transform .15s}
  .twk-toggle[data-on="1"] i{transform:translateX(14px)}

  .twk-num{display:flex;align-items:center;box-sizing:border-box;min-width:0;height:26px;padding:0 0 0 8px;
    border:.5px solid rgba(0,0,0,.1);border-radius:7px;background:rgba(255,255,255,.6)}
  .twk-num-lbl{font-weight:500;color:rgba(41,38,27,.6);cursor:ew-resize;
    user-select:none;padding-right:8px}
  .twk-num input{flex:1;min-width:0;height:100%;border:0;background:transparent;
    font:inherit;font-variant-numeric:tabular-nums;text-align:right;padding:0 8px 0 0;
    outline:none;color:inherit;-moz-appearance:textfield}
  .twk-num input::-webkit-inner-spin-button,.twk-num input::-webkit-outer-spin-button{
    -webkit-appearance:none;margin:0}
  .twk-num-unit{padding-right:8px;color:rgba(41,38,27,.45)}

  .twk-btn{appearance:none;height:26px;padding:0 12px;border:0;border-radius:7px;
    background:rgba(0,0,0,.78);color:#fff;font:inherit;font-weight:500;cursor:default}
  .twk-btn:hover{background:rgba(0,0,0,.88)}
  .twk-btn.secondary{background:rgba(0,0,0,.06);color:inherit}
  .twk-btn.secondary:hover{background:rgba(0,0,0,.1)}

  .twk-swatch{appearance:none;-webkit-appearance:none;width:56px;height:22px;
    border:.5px solid rgba(0,0,0,.1);border-radius:6px;padding:0;cursor:default;
    background:transparent;flex-shrink:0}
  .twk-swatch::-webkit-color-swatch-wrapper{padding:0}
  .twk-swatch::-webkit-color-swatch{border:0;border-radius:5.5px}
  .twk-swatch::-moz-color-swatch{border:0;border-radius:5.5px}

  .twk-chips{display:flex;gap:6px}
  .twk-chip{position:relative;appearance:none;flex:1;min-width:0;height:46px;
    padding:0;border:0;border-radius:6px;overflow:hidden;cursor:default;
    box-shadow:0 0 0 .5px rgba(0,0,0,.12),0 1px 2px rgba(0,0,0,.06);
    transition:transform .12s cubic-bezier(.3,.7,.4,1),box-shadow .12s}
  .twk-chip:hover{transform:translateY(-1px);
    box-shadow:0 0 0 .5px rgba(0,0,0,.18),0 4px 10px rgba(0,0,0,.12)}
  .twk-chip[data-on="1"]{box-shadow:0 0 0 1.5px rgba(0,0,0,.85),
    0 2px 6px rgba(0,0,0,.15)}
  .twk-chip>span{position:absolute;top:0;bottom:0;right:0;width:34%;
    display:flex;flex-direction:column;box-shadow:-1px 0 0 rgba(0,0,0,.1)}
  .twk-chip>span>i{flex:1;box-shadow:0 -1px 0 rgba(0,0,0,.1)}
  .twk-chip>span>i:first-child{box-shadow:none}
  .twk-chip svg{position:absolute;top:6px;left:6px;width:13px;height:13px;
    filter:drop-shadow(0 1px 1px rgba(0,0,0,.3))}
`;function useTweaks(i){const[t,r]=React.useState(i),a=React.useCallback((n,d)=>{const o=typeof n=="object"&&n!==null?n:{[n]:d};r(c=>({...c,...o})),window.parent.postMessage({type:"__edit_mode_set_keys",edits:o},"*"),window.dispatchEvent(new CustomEvent("tweakchange",{detail:o}))},[]);return[t,a]}function TweaksPanel({title:i="Tweaks",noDeckControls:t=!1,children:r}){const[a,n]=React.useState(!1),d=React.useRef(null),o=React.useMemo(()=>typeof document<"u"&&!!document.querySelector("deck-stage"),[]),[c,h]=React.useState(()=>o&&!!document.querySelector("deck-stage")?._railEnabled);React.useEffect(()=>{if(!o||c)return;const e=l=>{l.data&&l.data.type==="__omelette_rail_enabled"&&h(!0)};return window.addEventListener("message",e),()=>window.removeEventListener("message",e)},[o,c]);const[k,b]=React.useState(()=>{try{return localStorage.getItem("deck-stage.railVisible")!=="0"}catch{return!0}}),x=e=>{b(e),window.postMessage({type:"__deck_rail_visible",on:e},"*")},p=React.useRef({x:16,y:16}),u=16,g=React.useCallback(()=>{const e=d.current;if(!e)return;const l=e.offsetWidth,w=e.offsetHeight,m=Math.max(u,window.innerWidth-l-u),f=Math.max(u,window.innerHeight-w-u);p.current={x:Math.min(m,Math.max(u,p.current.x)),y:Math.min(f,Math.max(u,p.current.y))},e.style.right=p.current.x+"px",e.style.bottom=p.current.y+"px"},[]);React.useEffect(()=>{if(!a)return;if(g(),typeof ResizeObserver>"u")return window.addEventListener("resize",g),()=>window.removeEventListener("resize",g);const e=new ResizeObserver(g);return e.observe(document.documentElement),()=>e.disconnect()},[a,g]),React.useEffect(()=>{const e=l=>{const w=l?.data?.type;w==="__activate_edit_mode"?n(!0):w==="__deactivate_edit_mode"&&n(!1)};return window.addEventListener("message",e),window.parent.postMessage({type:"__edit_mode_available"},"*"),()=>window.removeEventListener("message",e)},[]);const v=()=>{n(!1),window.parent.postMessage({type:"__edit_mode_dismissed"},"*")},s=e=>{const l=d.current;if(!l)return;const w=l.getBoundingClientRect(),m=e.clientX,f=e.clientY,N=window.innerWidth-w.right,S=window.innerHeight-w.bottom,y=_=>{p.current={x:N-(_.clientX-m),y:S-(_.clientY-f)},g()},R=()=>{window.removeEventListener("mousemove",y),window.removeEventListener("mouseup",R)};window.addEventListener("mousemove",y),window.addEventListener("mouseup",R)};return a?React.createElement(React.Fragment,null,React.createElement("style",null,__TWEAKS_STYLE),React.createElement("div",{ref:d,className:"twk-panel","data-noncommentable":"",style:{right:p.current.x,bottom:p.current.y}},React.createElement("div",{className:"twk-hd",onMouseDown:s},React.createElement("b",null,i),React.createElement("button",{className:"twk-x","aria-label":"Close tweaks",onMouseDown:e=>e.stopPropagation(),onClick:v},"\u2715")),React.createElement("div",{className:"twk-body"},r,o&&c&&!t&&React.createElement(TweakSection,{label:"Deck"},React.createElement(TweakToggle,{label:"Thumbnail rail",value:k,onChange:x}))))):null}function TweakSection({label:i,children:t}){return React.createElement(React.Fragment,null,React.createElement("div",{className:"twk-sect"},i),t)}function TweakRow({label:i,value:t,children:r,inline:a=!1}){return React.createElement("div",{className:a?"twk-row twk-row-h":"twk-row"},React.createElement("div",{className:"twk-lbl"},React.createElement("span",null,i),t!=null&&React.createElement("span",{className:"twk-val"},t)),r)}function TweakSlider({label:i,value:t,min:r=0,max:a=100,step:n=1,unit:d="",onChange:o}){return React.createElement(TweakRow,{label:i,value:`${t}${d}`},React.createElement("input",{type:"range",className:"twk-slider",min:r,max:a,step:n,value:t,onChange:c=>o(Number(c.target.value))}))}function TweakToggle({label:i,value:t,onChange:r}){return React.createElement("div",{className:"twk-row twk-row-h"},React.createElement("div",{className:"twk-lbl"},React.createElement("span",null,i)),React.createElement("button",{type:"button",className:"twk-toggle","data-on":t?"1":"0",role:"switch","aria-checked":!!t,onClick:()=>r(!t)},React.createElement("i",null)))}function TweakRadio({label:i,value:t,options:r,onChange:a}){const n=React.useRef(null),[d,o]=React.useState(!1),c=React.useRef(t);c.current=t;const h=s=>String(typeof s=="object"?s.label:s).length;if(!(r.reduce((s,e)=>Math.max(s,h(e)),0)<=({2:16,3:10}[r.length]??0))){const s=e=>{const l=r.find(w=>String(typeof w=="object"?w.value:w)===e);return l===void 0?e:typeof l=="object"?l.value:l};return React.createElement(TweakSelect,{label:i,value:t,options:r,onChange:e=>a(s(e))})}const x=r.map(s=>typeof s=="object"?s:{value:s,label:s}),p=Math.max(0,x.findIndex(s=>s.value===t)),u=x.length,g=s=>{const e=n.current.getBoundingClientRect(),l=e.width-4,w=Math.floor((s-e.left-2)/l*u);return x[Math.max(0,Math.min(u-1,w))].value};return React.createElement(TweakRow,{label:i},React.createElement("div",{ref:n,role:"radiogroup",onPointerDown:s=>{o(!0);const e=g(s.clientX);e!==c.current&&a(e);const l=m=>{if(!n.current)return;const f=g(m.clientX);f!==c.current&&a(f)},w=()=>{o(!1),window.removeEventListener("pointermove",l),window.removeEventListener("pointerup",w)};window.addEventListener("pointermove",l),window.addEventListener("pointerup",w)},className:d?"twk-seg dragging":"twk-seg"},React.createElement("div",{className:"twk-seg-thumb",style:{left:`calc(2px + ${p} * (100% - 4px) / ${u})`,width:`calc((100% - 4px) / ${u})`}}),x.map(s=>React.createElement("button",{key:s.value,type:"button",role:"radio","aria-checked":s.value===t},s.label))))}function TweakSelect({label:i,value:t,options:r,onChange:a}){return React.createElement(TweakRow,{label:i},React.createElement("select",{className:"twk-field",value:t,onChange:n=>a(n.target.value)},r.map(n=>{const d=typeof n=="object"?n.value:n,o=typeof n=="object"?n.label:n;return React.createElement("option",{key:d,value:d},o)})))}function TweakText({label:i,value:t,placeholder:r,onChange:a}){return React.createElement(TweakRow,{label:i},React.createElement("input",{className:"twk-field",type:"text",value:t,placeholder:r,onChange:n=>a(n.target.value)}))}function TweakNumber({label:i,value:t,min:r,max:a,step:n=1,unit:d="",onChange:o}){const c=b=>r!=null&&b<r?r:a!=null&&b>a?a:b,h=React.useRef({x:0,val:0});return React.createElement("div",{className:"twk-num"},React.createElement("span",{className:"twk-num-lbl",onPointerDown:b=>{b.preventDefault(),h.current={x:b.clientX,val:t};const x=(String(n).split(".")[1]||"").length,p=g=>{const v=g.clientX-h.current.x,s=h.current.val+v*n,e=Math.round(s/n)*n;o(c(Number(e.toFixed(x))))},u=()=>{window.removeEventListener("pointermove",p),window.removeEventListener("pointerup",u)};window.addEventListener("pointermove",p),window.addEventListener("pointerup",u)}},i),React.createElement("input",{type:"number",value:t,min:r,max:a,step:n,onChange:b=>o(c(Number(b.target.value)))}),d&&React.createElement("span",{className:"twk-num-unit"},d))}function __twkIsLight(i){const t=String(i).replace("#",""),r=t.length===3?t.replace(/./g,c=>c+c):t.padEnd(6,"0"),a=parseInt(r.slice(0,6),16);if(Number.isNaN(a))return!0;const n=a>>16&255,d=a>>8&255,o=a&255;return n*299+d*587+o*114>148e3}const __TwkCheck=({light:i})=>React.createElement("svg",{viewBox:"0 0 14 14","aria-hidden":"true"},React.createElement("path",{d:"M3 7.2 5.8 10 11 4.2",fill:"none",strokeWidth:"2.2",strokeLinecap:"round",strokeLinejoin:"round",stroke:i?"rgba(0,0,0,.78)":"#fff"}));function TweakColor({label:i,value:t,options:r,onChange:a}){if(!r||!r.length)return React.createElement("div",{className:"twk-row twk-row-h"},React.createElement("div",{className:"twk-lbl"},React.createElement("span",null,i)),React.createElement("input",{type:"color",className:"twk-swatch",value:t,onChange:o=>a(o.target.value)}));const n=o=>String(JSON.stringify(o)).toLowerCase(),d=n(t);return React.createElement(TweakRow,{label:i},React.createElement("div",{className:"twk-chips",role:"radiogroup"},r.map((o,c)=>{const h=Array.isArray(o)?o:[o],[k,...b]=h,x=b.slice(0,4),p=n(o)===d;return React.createElement("button",{key:c,type:"button",className:"twk-chip",role:"radio","aria-checked":p,"data-on":p?"1":"0","aria-label":h.join(", "),title:h.join(" \xB7 "),style:{background:k},onClick:()=>a(o)},x.length>0&&React.createElement("span",null,x.map((u,g)=>React.createElement("i",{key:g,style:{background:u}}))),p&&React.createElement(__TwkCheck,{light:__twkIsLight(k)}))})))}function TweakButton({label:i,onClick:t,secondary:r=!1}){return React.createElement("button",{type:"button",className:r?"twk-btn secondary":"twk-btn",onClick:t},i)}Object.assign(window,{useTweaks,TweaksPanel,TweakSection,TweakRow,TweakSlider,TweakToggle,TweakRadio,TweakSelect,TweakText,TweakNumber,TweakColor,TweakButton});
