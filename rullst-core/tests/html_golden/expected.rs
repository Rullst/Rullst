//! The bytes each case of `html_golden.rs` writes, by case name, recorded
//! from the 13.0.0-rc.1 expansion.

pub const EXPECTED: &[(&str, &str)] = &[
    ("text/empty", ""),
    ("text/single-literal", "plain"),
    ("text/single-unquoted", "Hello there"),
    ("text/single-block", "Ana"),
    (
        "text/literal-is-written-as-is",
        "<p>9 > 8 & 'x' \"q\" <b> &amp;</p>",
    ),
    (
        "text/unquoted-sentence",
        "<p>Welcome back, Ana! You have 3 new messages (and 2 drafts).</p>",
    ),
    (
        "text/unquoted-edges",
        "<ul><li>Write tests</li><li>Ship it</li></ul>",
    ),
    ("text/groups", "<p>[draft] (a, b) (3) [Ana] (Ana)</p>"),
    ("text/apostrophes", "<p>Ana's turn, don't stop (Ana's)</p>"),
    (
        "text/numbers-and-punctuation",
        "<p>At 12:30 on day 5 - no, 1.5 days later; keep calm: 100% done & ready?</p>",
    ),
    ("text/number-literals", "<p>0x1F 1e3 2.50 7_000 1u8</p>"),
    ("text/entities", "<p>Tom &amp; Jerry &copy; 2026 &lt;3</p>"),
    (
        "text/keywords",
        "<p>if you match for else, true or false.</p>",
    ),
    ("text/non-ascii", "<p>Olá, São Paulo! Ça va?</p>"),
    (
        "text/pre-keeps-spaces",
        "<pre>  exact   spaces\n\tand a tab  </pre>",
    ),
    (
        "text/emoji-literal",
        "<span>🏠 Control Center</span><span>フレームワーク</span>",
    ),
    ("text/mixed-literals", "<p>Say hello now, (quoted)</p>"),
    (
        "text/blocks-and-literals",
        "<p>Hello, Ana!</p><p>123</p><p>a2c then quoted text</p>",
    ),
    ("text/comments-in-text", "<p>one three, onethree</p>"),
    (
        "text/control-edges",
        "<p>Status: finished today</p>All done",
    ),
    ("text/comments", "<div>x</div>"),
    (
        "text/doctype-upper",
        "<!DOCTYPE html><html><body></body></html>",
    ),
    (
        "text/doctype-lower",
        "<!doctype HTML><html lang=\"pt-BR\"></html>",
    ),
    ("text/doctype-after-comment", "<!DOCTYPE html><html></html>"),
    (
        "elements/siblings",
        "<span>a</span><span>b</span>cAna<b></b>",
    ),
    (
        "elements/deep",
        "<html><head><title>T</title></head><body><main><section><div><p><b><i>deep</i></b></p></div></section></main></body></html>",
    ),
    (
        "elements/self-closing-non-void",
        "<div></div><span class=\"x\"></span><use></use>",
    ),
    (
        "elements/void",
        "<area /><base /><br /><col /><embed /><hr /><img /><input /><link /><meta /><param /><source /><track /><wbr />",
    ),
    (
        "elements/void-written-open-close",
        "<br /><hr class=\"x\" />",
    ),
    ("elements/void-with-children", "<br>x</br>"),
    (
        "elements/uppercase-tags",
        "<DIV><IMG src=\"a.png\"></IMG><Br></Br></DIV>",
    ),
    (
        "elements/svg",
        "<svg aria-hidden=\"true\" viewBox=\"0 0 24 24\" stroke-width=\"2\"><path stroke-linecap=\"round\" d=\"M12 9v2m0 4h.01\"></path><use href=\"#i\"></use><use xlink:href=\"#icon\"></use></svg>",
    ),
    (
        "attributes/static",
        "<meta name=\"htmx-config\" content=\"{&quot;includeIndicatorStyles&quot;:false}\" title=\"a &amp; <b> 'q'\" />",
    ),
    (
        "attributes/bare",
        "<input required disabled hx-boost data-x type=\"checkbox\" checked />",
    ),
    (
        "attributes/alpine-and-htmx-names",
        "<div x-data=\"{ open: false }\" @click.outside=\"open = false\" :class=\"{ 'open': open }\"><button x-on:click.prevent=\"open = !open\" x-bind:disabled=\"busy\">Menu</button><form hx-post=\"/save\" hx-on::after-request=\"this.reset()\" @submit.prevent.stop=\"go()\"><input x-model.number=\"age\" x-transition:enter.duration.500ms required :value=\"age\" /></form></div>",
    ),
    (
        "attributes/colon-after-space",
        "<input required :value=\"v\" disabled:x=\"y\" /><p :type=\"t\" @click.self=\"s\"></p>",
    ),
    (
        "attributes/static-handlers",
        "<button onclick=\"go()\" hx-on-click=\"go()\" onclick>Go</button>",
    ),
    (
        "attributes/dynamic-strings",
        "<p title=\"a&quot;b&lt;c&gt;&amp;d&#x27;e\" data-owned=\"&lt;owned &amp; &quot;quoted&quot;&gt;\" data-ref=\"&lt;owned &amp; &quot;quoted&quot;&gt;\" data-cow=\"c&lt;o&gt;w\" data-refref=\"a&quot;b&lt;c&gt;&amp;d&#x27;e\" data-empty=\"\"></p>",
    ),
    (
        "attributes/dynamic-numbers-and-bools",
        "<input value=\"42\" min=\"-7\" max=\"18446744073709551615\" step=\"0.5\" data-f=\"-0\" aria-busy=\"true\" aria-hidden=\"false\" />",
    ),
    (
        "attributes/dynamic-markup-is-escaped",
        "<p title=\"&lt;b title=&quot;t&quot;&gt;x &amp; y&lt;/b&gt;\" data-raw=\"<i>raw</i>\"></p>",
    ),
    (
        "attributes/dynamic-options",
        "<p title=\"&lt;some&gt;\" data-none=\"\" data-ref=\"&lt;some&gt;\" data-num=\"5\"></p>",
    ),
    (
        "attributes/dynamic-expressions",
        "<p class=\"b\" title=\"1-&lt;2&gt;\" data-len=\"10\"></p>",
    ),
    (
        "attributes/order-of-every-kind",
        "<input type=\"text\" required name=\"a&quot;b&lt;c&gt;&amp;d&#x27;e\" checked title=\"&lt;some&gt;\" data-x=\"y\" />",
    ),
    (
        "urls/every-url-attribute",
        "<form action=\"#\" data=\"#\" formaction=\"#\"><a href=\"#\" cite=\"#\" xlink:href=\"#\">a</a><img src=\"#\" longdesc=\"#\" usemap=\"#\" /><video poster=\"#\"></video><body background=\"#\"></body><object codebase=\"#\"></object><menuitem icon=\"#\"></menuitem><html manifest=\"#\"></html></form>",
    ),
    ("urls/names-in-any-case", "<a HREF=\"#\" Src=\"#\">a</a>"),
    (
        "urls/other-attributes-keep-the-value",
        "<a title=\"javascript:alert(1)\" data-href=\"javascript:alert(1)\" hrefs=\"javascript:alert(1)\">a</a>",
    ),
    (
        "urls/data-on-media-only",
        "<div><img src=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\" /><IMG src=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\"></IMG><image href=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\"></image><audio src=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\"></audio><video src=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\" poster=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\"></video><source src=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\" /><track src=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\" /><a href=\"#\">a</a><iframe src=\"#\"></iframe></div>",
    ),
    (
        "urls/schemes",
        "<p><a href=\"#\">1</a><a href=\"#\">2</a><a href=\"#\">3</a><a href=\"#\">4</a><a href=\"javascriptx:x\">5</a><a href=\"jav ascript:x\">6</a><a href=\"&amp;#106;avascript:x\">7</a><a href=\"mailto:a@b.c\">8</a><a href=\"/path?a=1&amp;b=&quot;2&quot;\">9</a><a href=\"https://example.com/a?b=&lt;c&gt;\">10</a><a href=\"#top\">11</a><a href=\"\">12</a><a href=\":x\">13</a><a href=\"data\">14</a></p>",
    ),
    (
        "urls/value-types",
        "<p><a href=\"5\">n</a><a href=\"#\">m</a><a href=\"#\">r</a><a href=\"#\">s</a><a href=\"\">none</a><a href=\"#\">owned</a></p>",
    ),
    (
        "conditional/bools",
        "<input type=\"checkbox\" checked required />",
    ),
    (
        "conditional/options",
        "<p title=\"&lt;t&gt; &amp; &quot;q&quot;\" data-ref=\"&lt;t&gt; &amp; &quot;q&quot;\" data-num=\"-3\" data-markup=\"&lt;b&gt;m&lt;/b&gt;\"></p>",
    ),
    (
        "conditional/urls",
        "<p><a href=\"#\" cite=\"/path?a=1\">a</a><img src=\"data:text/html,&lt;b&gt;x&lt;/b&gt;\" /><a href=\"#\">b</a><a>c</a><a title=\"javascript:alert(1)\">d</a></p>",
    ),
    (
        "values/strings",
        "<p>plain text</p><p>&lt;script&gt;alert(&quot;x&quot; &amp; &#x27;y&#x27;)&lt;/script&gt;</p><p>a &lt; b &amp;&amp; c &gt; d</p><p>a &lt; b &amp;&amp; c &gt; d</p><p>&lt;borrowed&gt;</p><p>&quot;owned&quot;</p><p>plain text</p><p></p>",
    ),
    (
        "values/only-specials",
        "<p>&lt;&lt;&gt;&gt;&amp;&amp;&quot;&quot;&#x27;&#x27;</p><p>&amp;</p><p>&#x27;</p>",
    ),
    (
        "values/multibyte-around-specials",
        "<p>é&lt;ü&gt;ñ&amp;ç&quot;ã&#x27;õ</p><p>フ&lt;レ&gt;</p><p>🏠&amp;🛠\u{fe0f}</p>",
    ),
    (
        "values/specials-at-the-edges",
        "<p>&lt;a</p><p>a&gt;</p><p>&lt;</p><p>a&amp;b&amp;c</p>",
    ),
    (
        "values/fortune-messages",
        "<p>A computer scientist is someone who fixes things that aren&#x27;t broken.</p><p>&lt;script&gt;alert(&quot;This should not be displayed in a browser alert box.&quot;);&lt;/script&gt;</p><p>Emacs is a nice operating system, but I prefer UNIX. — Tom Christaensen</p>",
    ),
    (
        "values/integers",
        "<p>-128-32768-42-92233720368547758081701411834604692317316873037158841057279223372036854775807</p><p>2556553571844674407370955161534028236692093846346337460743176821145500</p>",
    ),
    (
        "values/floats",
        "<p>1.50.30000000000000004-0NaNinf-inf</p><p>10000000000000000000000.00000013402823500000000000000000000000000000000.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000022250738585072014325000000000</p>",
    ),
    ("values/bools", "<p>truefalse</p>"),
    (
        "values/expressions",
        "<p>3</p><p>ANA</p><p>&lt;5&gt;</p><p>6</p><p>three</p><p>42</p>",
    ),
    (
        "values/options",
        "<p>&lt;a&gt;</p><p></p><p>5</p><p></p><p>x</p><p></p><p>&amp;</p><p><b>b</b></p><p></p>",
    ),
    (
        "values/markup-and-raw",
        "<p><i><already> escaped</i></p><p><i><already> escaped</i></p><p><em>raw & ready</em></p><p><u>u</u></p><p><s>trusted</s></p>",
    ),
    (
        "fragments/collected",
        "<ul><li>Ana<span class=\"badge\">new</span></li><li>&lt;Bob&gt;<span class=\"badge\">new</span></li><li>Cé<span class=\"badge\">new</span></li></ul>",
    ),
    (
        "fragments/nested-calls",
        "<div class=\"dashboard\"><header><h2>Welcome, Ana</h2><span class=\"badge\">Admin</span></header><ul><li title=\"Ana\">Ana</li><li title=\"&lt;Bob&gt;\">&lt;Bob&gt;</li><li title=\"Cé\">Cé</li></ul><span class=\"badge\">&lt;x&gt;</span></div>",
    ),
    ("if/true", "<p>yes</p>"),
    ("if/false", "<div></div>"),
    ("if/let", "<b>&lt;ana&gt;</b>guest"),
    ("if/chain-0", "<p>zero</p>"),
    ("if/chain-1", "one"),
    ("if/chain-3", "<b>3</b>"),
    ("if/chain-9", "many"),
    ("if/beside-text", "<p>a b c  e</p>"),
    (
        "for/slice",
        "<ul><li>a</li><li>&lt;b&gt;</li><li>c</li></ul>",
    ),
    ("for/empty", "<ul></ul>"),
    ("for/range-and-text", "<p>0,1,2,3,</p>"),
    (
        "for/enumerate",
        "<ol><li data-i=\"0\">0: a</li><li data-i=\"1\">1: &lt;b&gt;</li><li data-i=\"2\">2: c</li></ol>",
    ),
    (
        "for/patterns",
        "<dl><dt>x</dt><dd>1</dd><dt>y</dt><dd>2</dd></dl>",
    ),
    ("for/owned", "<p><span>o1</span><span>o&amp;2</span></p>"),
    (
        "for/nested",
        "<table><tr><td>1</td><td>2</td></tr><tr><td>3</td><td>4</td></tr></table>",
    ),
    (
        "for/nested-control",
        "<ul><li class=\"even\">a</li><li>&lt;b&gt;</li><li class=\"even\">c</li></ul>",
    ),
    (
        "for/nested-html-call",
        "<div><i>a</i> <i>&lt;b&gt;</i> <i>c</i> </div>",
    ),
    ("match/empty", "<div><p>Nothing</p></div>"),
    ("match/guard", "<div>anonymous</div>"),
    ("match/or-pattern", "<div><b>&lt;two&gt;</b>!</div>"),
    ("match/binding", "<div>t x3</div>"),
    ("match/at-pattern", "<div><i>12</i></div>"),
    ("match/empty-arm", "<p></p>"),
    (
        "exit/continue-in-text",
        "<ul><b>before</b><li>n=1</li><i>after</i><b>before</b><b>before</b><li>n=3</li><i>after</i><b>before</b><li>n=4</li><i>after</i></ul>",
    ),
    (
        "exit/break-in-text",
        "<ul><li>keep</li><hr /><li>skip</li><hr /></ul>",
    ),
    (
        "exit/continue-in-attribute",
        "<p><a href=\"/w\" title=\"keep\">keep</a><a href=\"/w\" title=\"stop\">stop</a><a href=\"/w\" title=\"after\">after</a></p>",
    ),
    ("exit/continue-in-conditions", "<p>[keep+][[stop</p>"),
    (
        "exit/nested-loops",
        "<div><p><span>1</span><span>3</span></p><hr /><p><span>4</span></p><hr /></div>",
    ),
    (
        "exit/inner-iterable",
        "<div><p>ke</p><p>st</p><p>af</p></div>",
    ),
    (
        "exit/own-loop-inside-a-block",
        "<p>2keep2skip2stop2after</p>",
    ),
    (
        "exit/continue-deep-in-an-element",
        "<ul><li><span>keep</span><em></em></li>;<li><span>stop</span><em></em></li>;<li><span>after</span><em></em></li>;</ul>",
    ),
    (
        "pages/benchmarks-fortunes",
        "<!DOCTYPE html><html><head><title>Fortunes</title></head><body><table><tr><th>id</th><th>message</th></tr><tr><td>11</td><td>&lt;script&gt;alert(&quot;This should not be displayed in a browser alert box.&quot;);&lt;/script&gt;</td></tr><tr><td>2</td><td>A computer scientist is someone who fixes things that aren&#x27;t broken.</td></tr><tr><td>0</td><td>Additional fortune added at request time.</td></tr><tr><td>3</td><td>After enough decimal places, nobody gives a damn.</td></tr><tr><td>1</td><td>fortune: No such file or directory</td></tr><tr><td>12</td><td>フレームワークのベンチマーク</td></tr></table></body></html>",
    ),
    (
        "pages/benchmarks-fortunes-empty",
        "<!DOCTYPE html><html><head><title>Fortunes</title></head><body><table><tr><th>id</th><th>message</th></tr></table></body></html>",
    ),
    (
        "pages/benchmarks-form",
        "<!DOCTYPE html><html><head><title>Form</title></head><body><form method=\"post\" action=\"/form\"><input type=\"hidden\" name=\"_token\" value=\"tok&amp;en&quot;&lt;1&gt;\" /><input type=\"text\" name=\"name\" /><input type=\"number\" name=\"randomNumber\" /><button type=\"submit\">Send</button></form></body></html><!DOCTYPE html><html><head><title>Form</title></head><body><h1>Hello, &lt;Ana &amp; Bob&gt;</h1><p>-17</p></body></html>",
    ),
    (
        "pages/studio-error",
        "<div class=\"flex-grow flex flex-col items-center justify-center p-8 max-w-2xl mx-auto text-left\"><div class=\"w-full bg-slate-900 border border-amber-500/30 p-6 rounded-2xl space-y-4 shadow-xl\"><div class=\"flex items-center gap-3 text-amber-400 font-bold text-base\"><svg aria-hidden=\"true\" class=\"h-6 w-6 flex-shrink-0\" fill=\"none\" viewBox=\"0 0 24 24\" stroke=\"currentColor\" stroke-width=\"2\"><path stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z\"></path></svg><span>Database Connection Offline</span></div><div class=\"p-3 bg-slate-950 border border-slate-800 rounded-xl font-mono text-xs text-rose-400 overflow-x-auto\">error returned from database: &lt;pool timed out&gt; &amp; &quot;retry&quot;</div><ul class=\"text-xs text-slate-300 space-y-1.5 list-disc list-inside\"><li>Verify credentials in your project's .env file (DATABASE_URL).</li></ul></div></div>",
    ),
    (
        "pages/studio-shell",
        "<html lang=\"en\" class=\"min-h-full bg-slate-950\"><head><meta charset=\"UTF-8\" /><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" /><title>Rullst Studio Control Center</title><link href=\"/studio/assets/studio.css\" rel=\"stylesheet\" /></head><body class=\"min-h-screen text-slate-100 flex flex-col antialiased selection:bg-sky-500/30 selection:text-sky-200\"><nav class=\"order-3 w-full min-w-0 lg:order-none lg:w-auto flex items-center gap-1.5\"><a href=\"/studio/radar\" class=\"px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white\"><span>📡 Radar & Telemetry</span></a></nav><div class=\"flex-grow flex min-w-0\"><main id=\"studio-content\" class=\"w-full min-w-0 flex-grow flex flex-col bg-slate-950\"><section>trusted</section></main></div></body></html>",
    ),
    (
        "pages/ui-badge",
        "<span class=\"inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium border-emerald-300 bg-emerald-50\">Paid &amp; &lt;due&gt;</span>",
    ),
    (
        "pages/tutorial",
        "<div x-data=\"{ open: false }\" @click.outside=\"open = false\"><button @click=\"open = !open\" :aria-expanded=\"open\">Menu (2)</button><form hx-post=\"/notes\" hx-on::after-request=\"this.reset()\">Write a note, then press <kbd>Enter</kbd>.</form></div><ul id=\"user-list\" class=\"divide-y divide-slate-700\"><li class=\"py-2 text-slate-200\">Ana</li><li class=\"py-2 text-slate-200\">&lt;Bob&gt;</li></ul><p><em>flagged</em></p>",
    ),
    (
        "pages/field-valid",
        "<div class=\"space-y-1\"><label for=\"email\" class=\"text-sm font-medium\">email</label><input id=\"email\" name=\"email\" value=\"a@b.c\" required /></div>",
    ),
    (
        "pages/field-invalid",
        "<div class=\"space-y-1\"><label for=\"name\" class=\"text-sm font-medium\">name</label><input id=\"name\" name=\"name\" aria-invalid aria-describedby=\"name-error\" /><p id=\"name-error\" class=\"text-red-600\">Name is &lt;required&gt;</p></div>",
    ),
];
