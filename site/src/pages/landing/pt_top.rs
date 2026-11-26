//! The Brazilian Portuguese landing page (`/pt-br/`): the document, navigation, first screen, Performance and Security.

use rullst::html;
use rullst::html::Markup;

use super::{art, pt_bottom as bottom};
use crate::pages::{Context, INSTALL_VERSION, Lang, blocks};

/// The Brazilian Portuguese landing, with the benchmark blocks `proof` and `summary` (or their pending text).
pub fn page(cx: &Context, proof: Markup, summary: Markup) -> Markup {
    html! {
        <!doctype html>
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="description" content="O Rullst é uma suíte de frameworks Rust full-stack, open source: aplicações web sobre Axum e SQLx com os starters Blank, API e SaaS, mais pacotes opcionais para IA, pagamentos, e-mail, mensageria, dispositivos IoT, plataformas de aprendizagem e privacidade. A v13 está em pré-lançamento (13.0.0-rc.1, o primeiro release candidate): veja o desempenho medido, as proteções padrão e o que ele inclui, cada um com a sua fonte." />
                <meta name="theme-color" content="#070b15" />
                <meta name="referrer" content="no-referrer" />
                <meta http-equiv="Content-Security-Policy" content="default-src 'self'; img-src 'self'; style-src 'self'; script-src 'self'; connect-src 'none'; font-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'" />
                <meta property="og:title" content="Rullst — Inteligente, consciente da segurança e feito para uma produtividade sem esforço" />
                <meta property="og:description" content="Um framework web full-stack para Rust sobre Axum e SQLx: desempenho medido, proteções padrão e o que ele inclui, cada um com a sua fonte." />
                <meta property="og:url" content="https://rullst.win/pt-br/" />
                <meta property="og:image" content="https://rullst.win/Rullst/Rullst.png" />
                <meta property="og:type" content="website" />
                <meta property="og:locale" content="pt_BR" />
                <meta property="og:locale:alternate" content="en_US" />
                <meta name="twitter:card" content="summary_large_image" />
                <meta name="twitter:title" content="Rullst — Inteligente, consciente da segurança e feito para uma produtividade sem esforço" />
                <meta name="twitter:description" content="Um framework web full-stack para Rust sobre Axum e SQLx, com os starters Blank, API e SaaS. Conheça a v13, o código-fonte e as evidências." />
                <meta name="twitter:image" content="https://rullst.win/Rullst/Rullst.png" />
                <title>"Rullst — Inteligente, consciente da segurança e feito para uma produtividade sem esforço"</title>
                {blocks::head_links("https://rullst.win/pt-br/", "https://rullst.win/", "https://rullst.win/pt-br/", &cx.assets.css)}
                <script src={cx.assets.js.as_str()} defer></script>
            </head>
            <body data-ui-copied="Comando copiado." data-ui-copied-button="Copiado" data-ui-copy-failed="Área de transferência indisponível. Selecione e copie o comando exibido." data-ui-copy-failed-button="Selecione o texto" data-ui-replay-intro="Repetir introdução" data-ui-stop-intro="Parar introdução" data-ui-say-hello="Dizer olá ↗" data-ui-stop-waving="Parar de acenar">
                <a class="skip-link" href="#main">"Pular para o conteúdo"</a>" "
                <div class="ambient-scene" aria-hidden="true"><span></span><span></span><span></span></div>
                <aside class="release-banner" aria-label="Status de lançamento do Rullst"><span class="status-dot" aria-hidden="true"></span>" "<strong>"Pré-lançamento do Rullst v13"</strong>" "<span>"13.0.0-rc.1 é o primeiro release candidate, depois do congelamento de funcionalidades da 13.0 na 13.0.0-alpha.6; depois vem a 13.0.0, então instale a versão exata. A v12 chegou ao fim da vida útil em 16 de outubro de 2026."</span>" "<a href="/Rullst/book/compatibility-policy.html#source-branches">"Status de lançamento ↗"</a></aside>
                {nav()}
                <main id="main">
                    {hero(proof)}
                    {bottom::live()}
                    <div class="foundation-strip shell" aria-label="Fundamentos do framework" data-reveal><span>"CONSTRUÍDO EM TERRENO CONHECIDO"</span>" "<strong>"Axum"</strong><strong>"Tokio"</strong><strong>"Tower"</strong><strong>"SQLx"</strong>" "<a href="/Rullst/book/axum-sqlx-migration.html">"Mantenha suas saídas de emergência ↗"</a></div>
                    {performance(summary)}
                    {security()}
                    {bottom::included()}
                    {bottom::integrations()}
                    {bottom::examples()}
                    {bottom::academy()}
                    {bottom::community()}
                    {bottom::release()}
                </main>
                {bottom::footer(cx)}
                <p class="sr-only" role="status" aria-live="polite" data-copy-status></p>
            </body>
        </html>
    }
}

/// The primary navigation and the language switcher.
fn nav() -> Markup {
    html! {
        <nav class="site-nav" aria-label="Navegação principal">
            <div class="shell nav-inner">
                <a class="brand" href="https://rullst.win/pt-br/" aria-label="Página inicial do Rullst"><img src="/Rullst/Rullst.png" alt="" width="40" height="40" />" "<span>"rullst"<span class="brand-period">"."</span></span></a>" "
                {blocks::lang_switch(Lang::PtBr, "/", "/pt-br/", true)}" "
                <button class="nav-toggle" type="button" aria-label="Abrir ou fechar a navegação" aria-controls="navigation" aria-expanded="false" data-nav-toggle>"☰"</button>" "
                <div class="nav-links" id="navigation" data-navigation><a href="#performance">"Desempenho"</a>" "<a href="/pt-br/comparar/">"Comparar"</a>" "<a href="#security">"Segurança"</a>" "<a href="#included">"Incluído"</a>" "<a href="#examples">"Exemplos"</a>" "<a class="nav-academy" href="#academy">"Academy "<span aria-hidden="true">"↗"</span></a>" "<a href="/Rullst/book/">"Documentação"</a>" "<a href="#community">"Comunidade"</a>" "<a class="nav-source" href="https://github.com/Rullst/Rullst">"Código-fonte ↗"</a></div>
            </div>
        </nav>
    }
}

/// The first screen: what Rullst is, the install command, the concept illustration and three proof points.
fn hero(proof: Markup) -> Markup {
    html! {
        <header class="hero shell" data-reveal>
            <div class="hero-content">
                <p class="eyebrow"><span class="short-line" aria-hidden="true"></span>" A suíte de frameworks Rust open source"</p>
                <h1 class="hero-wordmark">"Rullst"<span class="wordmark-period">"."</span></h1>
                <p class="hero-lede"><strong lang="en">"Because With Rullst, We Rule!"</strong>" Uma suíte de frameworks Rust full-stack: aplicações web sobre "<strong>"Axum"</strong>" e "<strong>"SQLx"</strong>" com os starters "<strong>"Blank"</strong>", "<strong>"API"</strong>" e "<strong>"SaaS"</strong>", mais pacotes opcionais para "<a href="/Rullst/book/crates/ai.html">"IA"</a>", "<a href="/Rullst/book/crates/capital.html">"pagamentos"</a>", "<a href="/Rullst/book/crates/mail.html">"e-mail"</a>", "<a href="/Rullst/book/crates/messaging.html">"mensageria"</a>", "<a href="/Rullst/book/crates/iot.html">"dispositivos IoT"</a>", "<a href="/Rullst/book/labs-runner-contract.html">"plataformas de aprendizagem"</a>" e "<a href="https://github.com/Rullst/Rullst/tree/main/rullst-privacy">"privacidade"</a>"."</p>
                <p class="hero-slogan">"Inteligente. Consciente da segurança."<br />"Feito para uma "<span>"produtividade sem esforço."</span></p>
                <div class="hero-actions"><a class="button button-primary" href="/Rullst/book/start-here.html">"Comece a construir "<span aria-hidden="true">"↗"</span></a>" "<a class="button button-secondary" href="/pt-br/comparar/">"Compare com Loco e Topcoat "<span aria-hidden="true">"↗"</span></a></div>
                <div class="hero-tags" aria-label="Fatos do projeto"><span>"20 pacotes, uma versão"</span><span>"HTML no servidor + htmx"</span><span>"Licença MIT"</span></div>
                <p class="hero-note">"A documentação, os tutoriais e os painéis de benchmark estão em inglês."</p>" "
                <a class="hero-academy-link" href="#academy"><span aria-hidden="true">"✦"</span>" Um novo lugar para aprender. Conheça a Rullst Academy "<span aria-hidden="true">"↓"</span></a>
            </div>
            <div class="workbench">
                <div class="workbench-orbit" aria-hidden="true"></div>
                {art::signal_map()}
                <div class="workbench-label"><span class="status-dot" aria-hidden="true"></span>" SUA IDEIA. UM KIT DE FERRAMENTAS CONECTADO."<button type="button" class="motion-control" data-intro-toggle aria-pressed="false" hidden>"Repetir introdução"</button></div>
                <div class="engine" aria-label="Ilustração conceitual: conecte os dados, a identidade e a IA da sua aplicação com o Rullst. Não é telemetria ao vivo.">
                    <div class="engine-halo" aria-hidden="true"></div>
                    {art::engine_orbits()}
                    <div class="engine-core" aria-hidden="true">
                        <div class="engine-emblem"><span>"R"</span><span class="engine-corner">"↗"</span></div>
                        <strong>"Sua próxima ideia"</strong>
                        <span>"MOVIDA A RULLST"</span>
                    </div>
                    <div class="engine-module module-data">
                        <span class="module-symbol" aria-hidden="true">"{ }"</span>
                        <div><strong>"Seus dados"</strong><span>"Modelos → consultas"</span></div>
                    </div>
                    <div class="engine-module module-identity">
                        <span class="module-symbol" aria-hidden="true">"⌘"</span>
                        <div><strong>"Seus limites"</strong><span>"Identidade + políticas"</span></div>
                    </div>
                    <div class="engine-module module-ai">
                        <span class="module-symbol" aria-hidden="true">"✦"</span>
                        <div><strong>"Sua inteligência"</strong><span>"IA local + na nuvem"</span></div>
                    </div>" "
                    <span class="engine-caption">"MAPA CONCEITUAL / NÃO É UM MONITOR AO VIVO"</span>
                </div>
            </div>
            <div class="hero-install">
                <p class="install-label"><span class="status-dot" aria-hidden="true"></span>" INSTALE O PRÉ-LANÇAMENTO ATUAL"</p>
                <div class="terminal" aria-label="Comandos de instalação do pré-lançamento atual">
                    <div class="terminal-bar"><span class="terminal-dots" aria-hidden="true"><span></span><span></span><span></span></span>" "<span>"~/sua-proxima-ideia"</span><span class="terminal-mode">"Rust 1.96+"</span></div>
                    <div class="terminal-body">
                        <p class="terminal-comment">"# A 13.0 é um pré-lançamento, então informe a versão"</p>
                        <div class="command-row"><code><span class="prompt">"$"</span>" "<span id="install-command">"cargo install cargo-rullst --version ="{INSTALL_VERSION}" --locked"</span></code><button class="copy-command" type="button" data-copy-command="install-command" aria-label="Copiar o comando de instalação da CLI">"Copiar"</button></div>
                        <p class="terminal-comment">"# Depois escolha o starter Blank, API ou SaaS"</p>
                        <div class="command-row"><code><span class="prompt">"$"</span>" "<span id="new-command">"cargo rullst new my_app"</span></code><button class="copy-command" type="button" data-copy-command="new-command" aria-label="Copiar o comando de criação do projeto">"Copiar"</button></div>
                        <details class="workflow-details">
                            <summary>"Veja o que vem depois "<span aria-hidden="true">"↗"</span></summary>
                            <div class="project-tree" aria-label="Exemplo da estrutura gerada pelo starter SaaS">
                                <div><span class="tree-branch">"├─"</span>" src/controllers/ "<span class="tree-note">"suas rotas"</span></div>
                                <div><span class="tree-branch">"├─"</span>" src/models/ "<span class="tree-note">"seus dados"</span></div>
                                <div><span class="tree-branch">"├─"</span>" src/security_tests.rs "<span class="tree-note">"sua base, testada"</span></div>
                                <div><span class="tree-branch">"└─"</span>" Rullst.toml "<span class="tree-note">"suas escolhas"</span></div>
                            </div>
                            <p><code><span class="prompt">"$"</span>" cd my_app"<br /><span class="prompt">"$"</span>" cargo rullst dev"</code></p>
                            <div class="dev-loop"><span>"EDITAR"</span><b aria-hidden="true">"→"</b><span>"COMPILAR"</span><b aria-hidden="true">"→"</b><span>"REINICIAR"</span><b aria-hidden="true">"→"</b><span>"ATUALIZAR"</span></div>
                            <p class="terminal-comment terminal-caption">"Ilustração do fluxo · estrutura do starter SaaS · a primeira compilação compila as dependências e pode levar alguns minutos"</p>
                        </details>
                    </div>
                </div>
            </div>
            <ul class="proof-list" aria-label="Três fatos, cada um com a sua fonte">
                <li class="proof-card proof-security">
                    <span class="proof-index">"LIGADAS EM TODA APLICAÇÃO GERADA"</span>" "
                    <p class="proof-title">"Camadas de segurança ligadas por padrão"</p>
                    <p class="proof-figure">"Cabeçalhos seguros com um nonce de CSP por requisição, proteção contra CSRF e um WAF, montados pelo "<code>"Server"</code>" em todos os starters; o "<code>"src/security_tests.rs"</code>" de cada projeto verifica essas camadas. Sem "<code>"RULLST_ENV=production"</code>" (ou staging), a aplicação roda como desenvolvimento, em que o WAF só registra."</p>" "
                    <a href="#security">"Segurança "<span aria-hidden="true">"↓"</span></a>
                </li>
                <li class="proof-card proof-memory">
                    <span class="proof-index">"MEDIDO CONTRA O AXUM"</span>" "
                    <p class="proof-title">"Memória e velocidade, medidas"</p>
                    <div class="proof-data" data-landing-proof>
                        {proof}
                    </div>" "
                    <a href="#performance">"Desempenho "<span aria-hidden="true">"↓"</span></a>
                </li>
                <li class="proof-card proof-verification">
                    <span class="proof-index">"VERIFICADO NA CI"</span>" "
                    <p class="proof-title">"Verificação automatizada"</p>
                    <p class="proof-figure">"42 alvos de fuzzing, testes de mutação, Miri, Kani, sanitizers, OWASP ZAP, CodeQL e auditoria de dependências, em 36 workflows de CI."</p>" "
                    <a href="#verification">"Verificação "<span aria-hidden="true">"↓"</span></a>
                </li>
            </ul>
        </header>
    }
}

/// 01 / Performance: the Rullst/Benchmarks summary and who maintains that benchmark.
fn performance(summary: Markup) -> Markup {
    html! {
        <section id="performance" class="shell section performance-section" aria-labelledby="performance-heading" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"01 / Desempenho, medido"</p>
                <h2 id="performance-heading">"Medido contra o Axum."<br /><span class="accent-orange">"Resultados fortes e fracos, lado a lado."</span></h2>
                <p>"As pontuações abaixo são as do Rullst como o "<code>"cargo rullst new"</code>" o gera, com o middleware de segurança ligado. A comparação entre frameworks do "<a href="/Benchmarks/">"Rullst/Benchmarks"</a>" (em inglês) mede cada nova versão do Rullst contra o Axum (1,00×) e outros frameworks em máquinas hospedadas pelo GitHub, e este resumo é refeito todos os dias a partir dos últimos resultados; a página de resultados diz se eles passam pela porta de publicação do próprio benchmark."</p>
            </div>
            <div class="landing-explain">
                <h3>"O que os números significam para uma aplicação real"</h3>
                <p>"O Axum é a referência: um framework web Rust pequeno e rápido, sem camada de banco de dados, sem sessões e sem middleware de segurança. Uma pontuação de 1,00× é o mesmo resultado do Axum e 0,50× é a metade dele; em memória e inicialização a razão é invertida, então mais é sempre melhor. O Rullst é o Axum mais um ORM, templates "<code>"html!"</code>", sessões e as camadas de segurança que uma aplicação gerada liga, então algum custo frente ao Axum puro é esperado. A questão é onde ele aparece."</p>
                <ul>
                    <li><strong>"Por onde começar: "</strong>"memória e inicialização, que importam quando você paga por uma máquina pequena ou reinicia com frequência. Como o Rullst se saiu nas duas nos resultados atuais, e a memória dele com usuários autenticados (um número que não entra em nenhuma pontuação), está na lista em palavras simples abaixo, refeita a cada rodada."</li>
                    <li><strong>"O preço: "</strong>"a vazão. O WAF, a verificação CSRF, os cabeçalhos seguros e o log de acesso estão ligados em toda aplicação gerada, e cada requisição paga por eles. Por isso o benchmark tem uma segunda linha do Rullst, sem essas camadas: a diferença entre as duas linhas mede o que elas custam."</li>
                    <li><strong>"No que a 13.1 trabalha: "</strong>"requisições por segundo em páginas triviais, renderização de listas longas com "<code>"html!"</code>" e gravações que abrem uma transação por linha, onde rodadas anteriores mediram o Rullst atrás. O trabalho de desempenho da 13.1 precisa mostrar números de antes e depois e não pode enfraquecer nenhuma camada de segurança: "<a href="https://github.com/Rullst/Rullst/issues/832">"issue #832"</a>"."</li>
                </ul>
            </div>
            <p class="landing-disclosure"><strong>"Quem mediu."</strong>" O mantenedor do Rullst também mantém esse benchmark, escreveu todas as implementações dele e projetou o método. O README dele lista o que ele não mede: logins, requisições grandes, TLS, um banco de dados em outra máquina e escala além de 2 vCPUs, entre outros."</p>
            <div class="landing-bench" data-landing-benchmarks>
                {summary}
            </div>
            <a class="compare-callout" href="/pt-br/comparar/"><span class="compare-callout-index">"LADO A LADO"</span><strong>"Rullst, Loco e Topcoat comparados"</strong><span>"O que cada um escolheu, a vantagem e o custo de cada escolha, com a versão e a data em que cada fato foi verificado "<span aria-hidden="true">"↗"</span></span></a>
            <div class="performance-links"><a href="/pt-br/comparar/">"Rullst, Loco e Topcoat comparados, com a tabela completa "<span aria-hidden="true">"↗"</span></a>" "<a href="/Benchmarks/">"Resultados completos, método e limitações (em inglês) "<span aria-hidden="true">"↗"</span></a>" "<a href="/Rullst/benches/">"Histórico de microbenchmarks no "<code>"main"</code>" (verificação de regressões, não um ranking) "<span aria-hidden="true">"↗"</span></a></div>
        </section>
    }
}

/// 02 / Security: on by default, opt-in and verified in CI.
fn security() -> Markup {
    html! {
        <section id="security" class="shell section security-section" aria-labelledby="security-heading" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"02 / Segurança, por padrão e por teste"</p>
                <h2 id="security-heading">"Mais proteções ligadas por padrão."<br /><span class="accent-green">"Mais verificação automatizada."</span></h2>
                <p>"Com o que uma aplicação gerada começa, o que você acrescenta quando precisa e o que a CI executa contra o código do próprio framework. Nada disso prova que uma aplicação é segura: as regras de autorização, a configuração dos provedores e o deploy continuam sendo seus."</p>
            </div>
            <div class="security-grid">
                <article class="security-column security-default">
                    <span class="security-index">"01 / LIGADO POR PADRÃO"</span>" "
                    <h3>"Em toda aplicação gerada"</h3>
                    <ul>
                        <li><b>"Cabeçalhos seguros"</b>" com um nonce de CSP por requisição, em todos os ambientes."</li>
                        <li><b>"Proteção contra CSRF"</b>" (um token double-submit) nas rotas de escrita, em todos os ambientes."</li>
                        <li>"Um "<b>"WAF"</b>" que recusa requisições suspeitas em staging e produção e só as registra em desenvolvimento. O Dockerfile, o Compose e os arquivos de Kubernetes gerados definem "<code>"RULLST_ENV=production"</code>"."</li>
                        <li><b>"Testes de segurança"</b>": o "<code>"src/security_tests.rs"</code>" de cada projeto verifica os cabeçalhos, o CSRF e o WAF com a base de produção, via "<code>"cargo test"</code>"."</li>
                        <li><b>"Verificações de produção"</b>": o starter API não inicia sem "<code>"API_TOKEN"</code>", e o código de cobrança gerado recusa uma chave de pagamento vazia em produção. Na 13.0.0-rc.1 (ainda não lançada), e-mail, IA e os starters Turso também recusam configurações vazias em staging e produção em vez de usar um mock offline, e a CI falha diante de um fallback não revisado que siga os padrões que ela conhece."</li>
                    </ul>" "
                    <a href="/Rullst/book/security-layers.html#what-generated-applications-already-have">"O que cada starter monta (em inglês) "<span aria-hidden="true">"↗"</span></a>
                </article>
                <article class="security-column security-opt-in">
                    <span class="security-index">"02 / OPCIONAL"</span>" "
                    <h3>"Quando a sua aplicação precisar"</h3>
                    <ul>
                        <li><b>"Passkeys"</b>" (WebAuthn) no "<code>"rullst-auth"</code>", com um armazenamento opcional e compartilhado de cerimônias no PostgreSQL."</li>
                        <li><b>"Login com OAuth2/OIDC"</b>": "<code>"cargo rullst make:oauth"</code>" para Google, GitHub, Microsoft ou Apple."</li>
                        <li><b>"Verificação em duas etapas"</b>" com TOTP e códigos de recuperação: "<code>"cargo rullst make:mfa"</code>"."</li>
                        <li>"Um "<b>"bloqueio de login"</b>" ("<code>"LoginGuard"</code>") e mais camadas no "<code>"rullst-security"</code>": caminhos honeypot, mascaramento de segredos nas respostas, uma proteção de origem para WebSocket e validação de JSON por schema."</li>
                    </ul>" "
                    <a href="/Rullst/book/security-layers.html#recommended-production-composition">"Pilha de produção recomendada (em inglês) "<span aria-hidden="true">"↗"</span></a>
                </article>
                <article id="verification" class="security-column security-verification">
                    <span class="security-index">"03 / VERIFICADO NA CI"</span>" "
                    <h3>"Verificações automatizadas do código do framework"</h3>
                    <ul>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml">"42 alvos de fuzzing"</a>" (libFuzzer), exigidos como evidência de lançamento."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/mutants-pr.yml">"Testes de mutação"</a>" das linhas alteradas de cada pull request (não é uma verificação obrigatória), além de "<a href="https://github.com/Rullst/Rullst/actions/workflows/mutants.yml">"campanhas manuais"</a>"."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/miri.yml">"Miri"</a>" e "<a href="https://github.com/Rullst/Rullst/actions/workflows/kani.yml">"Kani"</a>" em partes críticas selecionadas, semanalmente e a cada lançamento."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/sanitizers.yml">"ThreadSanitizer e AddressSanitizer"</a>", diariamente."</li>
                        <li>"Varreduras do "<a href="https://github.com/Rullst/Rullst/actions/workflows/dast-zap.yml">"OWASP ZAP"</a>" na API REST gerada, no starter SaaS e no exemplo LMS: bloqueantes, com sete regras explicadas relatadas como INFO."</li>
                        <li>"Análise do "<a href="https://github.com/Rullst/Rullst/actions/workflows/codeql.yml">"CodeQL"</a>" no código Rust e JavaScript."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/supply-chain.yml">"Auditoria de dependências"</a>": alertas do RustSec, licenças e origens a cada mudança e duas vezes por dia."</li>
                        <li><a href="https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml">"Nada de "<code>"unwrap"</code>", "<code>"expect"</code>" ou "<code>"panic!"</code></a>" no código de produção, proibidos pelo Clippy."</li>
                    </ul>" "
                    <a href="https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md">"Os 36 workflows e o escopo exato de cada um (em inglês) "<span aria-hidden="true">"↗"</span></a>
                </article>
            </div>
            <p class="security-compare" id="checked-security">"Comparando os padrões: o Loco ("<code>"loco-rs 1.3.1"</code>") documenta um middleware de cabeçalhos seguros desligado por padrão e nenhum middleware de CSRF; o Topcoat ("<code>"topcoat 0.12.1"</code>") rejeita por padrão requisições de outras origens que alteram estado e não documenta cabeçalhos seguros nem tokens de CSRF. Verificado em "<time datetime="2026-11-21">"21 de novembro de 2026"</time>"; "<a href="/pt-br/comparar/#topic-security">"veja lado a lado"</a>"."</p>
            <p class="security-review"><strong>"Revisão."</strong>" O Rullst tem um único mantenedor, e pull requests não precisam de um segundo revisor. A revisão assistida por IA e as verificações automatizadas não são uma revisão humana independente, e nenhuma auditoria externa foi concluída. "<a href="/Rullst/book/assurance-case.html">"Caso de garantia de segurança (em inglês) "<span aria-hidden="true">"↗"</span></a>" "<a href="/Rullst/book/external-audit-kit.html">"Kit de auditoria externa, para revisores (em inglês) "<span aria-hidden="true">"↗"</span></a>" "<a href="https://github.com/Rullst/Rullst/security/policy">"Reportar uma vulnerabilidade "<span aria-hidden="true">"↗"</span></a></p>
        </section>
    }
}
