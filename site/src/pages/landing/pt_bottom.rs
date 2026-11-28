//! The Brazilian Portuguese landing page (`/pt-br/`): Included, Integrations, Examples, Academy, Community, the release notice and the footer.

use rullst::html;
use rullst::html::Markup;

use super::{Demo, art, demo_card_pt, included_card};
use crate::footprint;
use crate::pages::made_with;
use crate::pages::{Context, Lang};

/// The live applications made with Rullst, right after the first screen.
pub fn live() -> Markup {
    html! {
        <section id="live-applications" class="shell section live-section" aria-labelledby="made-with-heading" data-reveal>
            {made_with::card(Lang::PtBr)}
        </section>
    }
}

/// 03 / Included in Rullst.
pub fn included() -> Markup {
    html! {
        <section id="included" class="shell section included-section" aria-labelledby="included-heading" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"03 / Incluído no Rullst"</p>
                <h2 id="included-heading">"Menos para montar."<br /><span class="accent-orange">"Mais para inspecionar."</span></h2>
                <p>"Partes que vêm com o framework, cada uma documentada com aquilo contra o que foi testada e o que ela não faz. Vinte pacotes compartilham uma versão; os especializados são pacotes Extension opcionais."</p>
            </div>
            <p class="landing-checked" id="checked-included">"O que um cartão diz sobre o Loco e o Topcoat foi verificado em "<time datetime="2026-11-27">"27 de novembro de 2026"</time>" no Loco "<code>"loco-rs 1.3.1"</code>" e no Topcoat "<code>"topcoat 0.12.1"</code>", as versões mais recentes deles nessa data."</p>
            <div class="included-grid">
                {included_card("CLI · RECURSOS", html! { <code>"cargo rullst footprint"</code> }, html! { "Mede o tamanho do binário, a memória ociosa, o tempo de inicialização, o tempo de CPU por requisição e o peso das páginas da sua aplicação, diz como mediu cada um e pode reprovar a CI quando um orçamento cresce. O número de carbono é uma estimativa SCI a partir de dados que você informa, não uma medição." }, Some("O Loco e o Topcoat não documentam um comando assim."), ("/Rullst/book/footprint.html", "Guia do footprint (em inglês)"))}
                {included_card("CLI · ATUALIZAÇÕES", html! { <code>"cargo rullst upgrade"</code> }, html! { "O "<code>"--dry-run"</code>" lista cada mudança no código que uma versão mais nova do Rullst exige, em "<code>"arquivo:linha"</code>", com o código da regra e a linha do guia de migração; ao aplicar, ele faz backup dos arquivos, atualiza as versões e roda "<code>"cargo fix"</code>" e "<code>"cargo check"</code>". Ele não migra dados." }, Some("O Loco documenta um guia de atualização e nenhum comando de atualização; o Topcoat não documenta nenhum dos dois."), ("/Rullst/book/tutorials/36-assisted-framework-upgrades.html", "Atualizações assistidas (em inglês)"))}
                {included_card("CLI · TESTES", html! { "Testes desde o primeiro commit" }, html! { "Todo projeto que o "<code>"cargo rullst new"</code>" cria tem um teste de fumaça de cada página (status, cabeçalhos de segurança, higiene do HTML) e, a menos que você passe "<code>"--no-ci"</code>", um workflow de CI com Clippy, testes, "<code>"cargo audit"</code>" e uma varredura de segredos. O "<code>"cargo rullst add browser-tests"</code>" abre cada página no Chromium, no Firefox e no WebKit com o axe-core." }, None, ("/Rullst/book/testing-your-app.html", "Testando a sua aplicação (em inglês)"))}
                {included_card("DESENVOLVIMENTO · STUDIO", html! { "Uma sala de controle de desenvolvimento" }, html! { "O Studio roda como um servidor local de depuração em "<code>"127.0.0.1:5555"</code>": um inspetor de banco de dados com diagrama ER, filas, feature flags, traces de requisições e os eventos de segurança do processo em execução." }, Some("O Loco e o Topcoat não documentam um painel de desenvolvimento."), ("/Rullst/book/3-rullst-studio.html", "Guia do Studio (em inglês)"))}
                {included_card("ADMIN · NEXUS", html! { "Um admin gerado" }, html! { "O Nexus cria telas renderizadas no servidor de listagem, busca, criação, edição e ações em lote para os modelos do ORM que você registra, com políticas de campos e RBAC; builds de release exigem o login da sua aplicação ou Basic Auth sobre TLS." }, Some("O Loco e o Topcoat não documentam um admin gerado (o Topcoat tem uma demo de admin escrita à mão)."), ("/Rullst/book/4-rullst-nexus.html", "Guia do Nexus (em inglês)"))}
                {included_card("COBRANÇA · CAPITAL", html! { "Pagamentos com verificações que falham fechadas" }, html! { "Adaptadores para Stripe e InfinitePay, webhooks assinados com proteção contra repetição, cotas, direitos de acesso e faturas em unidades menores da moeda. O código de cobrança gerado recusa chaves de pagamento vazias em produção, e o gerado desde a 13.0.0-rc.1 também as recusa em staging." }, Some("O Loco e o Topcoat não documentam pagamentos."), ("/Rullst/book/crates/capital.html", "Guia de cobrança (em inglês)"))}
                {included_card("E-MAIL", html! { "E-mail transacional" }, html! { "Resend, AWS SES, SendPulse ou SMTP, enviados em segundo plano, com supressão de destinatários e inspeção opcional de anexos. Desde a 13.0.0-rc.1, a fachada "<code>"Mail"</code>" trata uma chave de provedor ausente em staging ou produção como um erro, e não como um envio que só pareceu ter dado certo." }, None, ("/Rullst/book/crates/mail.html", "Guia de e-mail (em inglês)"))}
                {included_card("IA", html! { "Um cliente de IA protegido" }, html! { "Gemini, OpenAI, Claude e endpoints compatíveis com OpenAI (presets DeepSeek e Ollama), com filtro de injeção de prompt, mascaramento de dados pessoais e fixtures offline para os testes." }, None, ("/Rullst/book/ai-provider-capabilities.html", "Limites dos provedores (em inglês)"))}
                {included_card("IOT · OTA ASSINADO", html! { "Atualizações de firmware verificáveis" }, html! { "Helpers de telemetria "<code>"no_std"</code>", codificadores MQTT 5 e CoAP e verificação de manifestos OTA assinados com Ed25519, com um contrato de contador anti-rollback, executados em um BBC micro:bit V2. Transportes, gravação e bootloaders continuam sendo da aplicação." }, Some("O Loco e o Topcoat não documentam suporte a IoT ou OTA."), ("/Rullst/book/crates/iot.html", "Guia de IoT (em inglês)"))}
                {included_card("LABS", html! { "Exercícios avaliados, com o seu runner" }, html! { "Contratos para tarefas de exercícios que um runner implantado à parte assume: leases, cancelamento, retenção e recibos assinados. O Labs nunca executa o código dos alunos." }, None, ("/Rullst/book/labs-runner-contract.html", "Contrato do runner (em inglês)"))}
                {included_card("PRIVACIDADE", html! { "Políticas de idade e consentimento" }, html! { "Políticas de idade proporcionais, atestações de limite assinadas e consentimento e revogação versionados, em SQLite ou PostgreSQL; o "<code>"make:age-gate"</code>" e o "<code>"make:privacy"</code>" integram isso a uma aplicação. Nenhum provedor real de verificação de idade está incluído." }, None, ("https://github.com/Rullst/Rullst/tree/main/rullst-privacy", "rullst-privacy"))}
                {included_card("SUPERVISÃO", html! { "Supervisão transparente de provas" }, html! { "Contratos de provas e de controle parental com confirmação explícita do que é coletado, observações tipadas do navegador e adaptadores de presença na câmera e de atividade de áudio. Nenhum modelo de mídia, correção automática ou controle do dispositivo inteiro está incluído." }, None, ("/Rullst/book/supervision.html", "Guia de supervisão (em inglês)"))}
            </div>
            <p class="section-note">"Os pacotes Core estão em toda aplicação; os pacotes Extension são opcionais e têm a mesma promessa de estabilidade da 13.x para o escopo documentado ("<a href="/Rullst/book/maturity.html">"níveis de maturidade"</a>", em inglês). A página "<a href="/Rullst/book/pt-BR/scope-and-limits.html">"Escopo e limites"</a>" lista o que o Rullst não faz e o que os testes dele não conseguem comprovar."</p>
        </section>
    }
}

/// 04 / Integrations, by evidence (the entries of README's Integrations table).
pub fn integrations() -> Markup {
    html! {
        <section id="integrations" class="shell section integrations-section" aria-labelledby="integrations-heading" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"04 / Integrações, por evidência"</p>
                <h2 id="integrations-heading">"Até onde cada integração"<br /><span class="accent-green">"foi testada."</span></h2>
                <p>"Todas as integrações embutidas, agrupadas pela evidência mais forte por trás de cada uma. Os nomes são texto simples: listar um serviço não implica parceria nem endosso."</p>
            </div>
            <div class="integration-levels">
                <article class="integration-level" data-integration-level="real-accounts">
                    <span class="integration-index">"01 / CONTAS REAIS"</span>" "
                    <h3>"Usadas com contas reais"</h3>
                    <ul>
                        <li><b>"Stripe"</b>" (compras reais no SaaS em produção do mantenedor, sem rodada de aceitação registrada e sem teste automatizado contra uma conta real)"</li>
                        <li><b>"InfinitePay"</b>" (um pagamento Pix real na rodada de aceitação da 13.0.0-alpha.5)"</li>
                        <li><b>"Bunny Stream"</b>", "<b>"Resend"</b>", e-mail via "<b>"SMTP"</b>", "<b>"Google Gemini"</b>", login com "<b>"Google"</b>" e login com "<b>"GitHub"</b>" (usados pelas aplicações em produção do mantenedor, sem teste automatizado com uma conta real)"</li>
                        <li><b>"PostgreSQL"</b>" e "<b>"SQLite"</b>" (em produção nas aplicações do mantenedor, além da CI)"</li>
                        <li>"O adaptador de IA compatível com OpenAI (validado com o "<b>"Groq"</b>", um provedor compatível com OpenAI, e não com a própria OpenAI)"</li>
                    </ul>" "
                    <a href="/Rullst/book/maturity.html#what-supported-promises">"O que &quot;suportado&quot; promete ↗"</a>
                </article>
                <article class="integration-level" data-integration-level="ci-containers">
                    <span class="integration-index">"02 / CONTÊINERES LOCAIS NA CI"</span>" "
                    <h3>"Testadas na CI contra servidores em contêineres locais"</h3>
                    <ul>
                        <li><b>"MySQL"</b>" e "<b>"MariaDB"</b></li>
                        <li><b>"Redis"</b>" (cache, filas, limites de taxa, estruturas de dados do ORM e mensageria com Redis Streams)"</li>
                        <li><b>"SurrealDB"</b></li>
                        <li><b>"Turso/libSQL"</b>" (a imagem "<code>"libsql-server"</code>", que é o "<code>"sqld"</code>")"</li>
                        <li>"Busca com "<b>"Meilisearch"</b></li>
                        <li><b>"pgvector"</b></li>
                        <li>"Armazenamento compatível com "<b>"S3"</b>" (um servidor "<b>"Garage"</b>")"</li>
                    </ul>" "
                    <a href="/Rullst/book/database-support.html#where-the-live-tests-run">"Onde rodam os testes com servidores reais ↗"</a>
                </article>
                <article class="integration-level" data-integration-level="offline">
                    <span class="integration-index">"03 / SOMENTE OFFLINE"</span>" "
                    <h3>"Somente offline: modo mock, fixtures locais ou um kit de conformidade"</h3>
                    <ul>
                        <li>"E-mail via "<b>"AWS SES"</b>" e "<b>"SendPulse"</b></li>
                        <li><b>"OpenAI"</b>" e "<b>"Anthropic Claude"</b>", além dos presets "<b>"DeepSeek"</b>" e "<b>"Ollama"</b>" do adaptador compatível com OpenAI"</li>
                        <li>"Login com "<b>"Microsoft"</b>", "<b>"Apple"</b>", "<b>"Auth0"</b>", "<b>"AWS Cognito"</b>", "<b>"Facebook"</b>", "<b>"X (Twitter)"</b>", "<b>"Discord"</b>", "<b>"LinkedIn"</b>" e "<b>"OpenID Connect"</b>" genérico"</li>
                        <li>"O adaptador "<b>"S3/R2"</b>" do "<code>"rullst-media"</code></li>
                    </ul>" "
                    <a href="/Rullst/book/provider-conformance.html">"Kits de conformidade de provedores ↗"</a>
                </article>
            </div>
            <p class="integration-help"><strong>"Ajude a subir uma integração de nível."</strong>" A issue de acompanhamento "<a href="https://github.com/Rullst/Rullst/issues/801">"#801"</a>" lista uma issue por provedor que nunca rodou com uma conta real. Se você tem uma, escolha uma issue, diga que vai assumi-la e relate o que aconteceu; uma execução bem-sucedida é registrada na documentação e nesta página com a data. Dúvidas e companhia são bem-vindas na "<a href="https://discord.com/invite/2ntKFtsSjw">"comunidade do Rullst no Discord"</a>"."</p>
            <p class="integration-recipes" data-integration-recipes><strong>"Receitas, não integrações."</strong>" MongoDB, DuckDB e Qdrant são receitas de aplicação: seu código usa diretamente os crates oficiais de cada um. Os adaptadores embutidos deles foram removidos na 13.0.0-alpha.5. Outros serviços também são código da aplicação: escreva um adaptador sobre a trait do provedor (pagamentos, e-mail, vídeo) ou siga uma receita. "<a href="/Rullst/book/recipes/index.html">"Ler as receitas ↗"</a></p>
            <p class="integration-production">"As aplicações em produção do mantenedor, Academy, capoeira e vene.win, rodam em domínios reais em uma única máquina Arm Oracle Ampere A1, atrás de um Caddy compartilhado com certificados Let's Encrypt. A CI testa o deploy gerado somente em x86_64; máquinas Arm não são testadas na CI. "<a href="/Rullst/book/pt-BR/scope-and-limits.html#contas-e-provedores-externos">"Limites de deploy "<span aria-hidden="true">"↗"</span></a></p>
        </section>
    }
}

/// 05 / The deployed examples.
pub fn examples() -> Markup {
    html! {
        <section id="examples" class="shell section examples-section" data-reveal>
            <div class="section-heading">
                <p class="eyebrow">"05 / Menos imaginar. Mais explorar."</p>
                <h2>"Não é só um framework."<br /><span class="gradient-text">"É uma suíte para dar vida às suas ideias."</span></h2>
                <p>"Explore quatro aplicações publicadas de forma independente no Azure Container Apps. Siga o código-fonte, descubra um ponto de partida e depois faça dele o seu."</p>
            </div>
            <p class="demo-startup-note"><strong>"Aviso sobre a inicialização das demos."</strong>" Estas aplicações podem escalar a zero quando ficam ociosas. A requisição seguinte provoca uma inicialização a frio (cold start), enquanto o Azure Container Apps inicia uma instância de contêiner e a aplicação. Isso pode aumentar temporariamente o tempo da primeira resposta. Aguarde um momento até o site carregar."</p>
            <div class="demo-grid">
                {demo_card_pt(&Demo { class: "demo-showcase", caption: "ROTAS / DADOS / IDEIAS", kicker: "01 / EXPLORE AS PEÇAS", title: "Rullst Showcase", text: "Um blog e uma vitrine de aplicações. Explore páginas renderizadas no servidor e demonstrações selecionadas de integrações.", href: "https://rullst-showcase.redpond-24d9228d.eastus.azurecontainerapps.io/", button: "Explorar o Showcase", art: art::demo_showcase() })}
                {demo_card_pt(&Demo { class: "demo-lms", caption: "AULAS / QUIZZES / PROGRESSO", kicker: "02 / ABRA ESPAÇO PARA APRENDER", title: "LMS Academy", text: "Conheça um exemplo de plataforma de aprendizado, com catálogo público de cursos e pontos de entrada para contas.", href: "https://rullst-lms.redpond-24d9228d.eastus.azurecontainerapps.io/", button: "Explorar o LMS", art: art::demo_lms() })}
                {demo_card_pt(&Demo { class: "demo-portfolio", caption: "PROJETOS / HABILIDADES / SUA HISTÓRIA", kicker: "03 / MOSTRE SEU TRABALHO", title: "Portfólio de desenvolvedor", text: "Um ponto de partida de portfólio para apresentar projetos, habilidades e experiência em uma aplicação web enxuta.", href: "https://rullst-portfolio.redpond-24d9228d.eastus.azurecontainerapps.io/", button: "Explorar o portfólio", art: art::demo_portfolio() })}
                {demo_card_pt(&Demo { class: "demo-saas", caption: "BLUEPRINT / PRODUTO / POSSIBILIDADES", kicker: "04 / COMECE SEU PRÓXIMO PRODUTO", title: "SaaS Blueprint", text: "Um blueprint de aplicação SaaS. Explore um ponto de partida para construir seu próprio produto de software como serviço.", href: "https://saas.rullst.win/", button: "Explorar o SaaS", art: art::demo_saas() })}
            </div>
            <div class="demo-footnote">
                <p>"Demos públicas, não certificações de produção. Cada demo é atualizada no próprio ritmo e pode rodar uma versão do Rullst anterior à 13.0.0-rc.1. O checkout do SaaS é real e cobra dinheiro de verdade; os fixtures de pagamento do Showcase não são um checkout real. A não ser em uma compra que você pretenda fazer, use dados de teste, não informações sensíveis. A disponibilidade é gerenciada à parte."</p>
                <a href="https://github.com/Rullst/examples">"Código-fonte e receitas de deploy ↗"</a>
            </div>
        </section>
    }
}

/// Rullst Academy.
pub fn academy() -> Markup {
    html! {
        <section id="academy" class="shell academy-section" aria-labelledby="academy-heading" data-reveal>
            <div class="academy-spotlight">
                <div class="academy-content">
                    <p class="academy-label"><span class="academy-badge">"Rullst Academy"</span>" SUA ESCOLA DE TECNOLOGIA GRATUITA"</p>
                    <h2 id="academy-heading">"Um mundo para descobrir."<br /><span>"Um passo de cada vez."</span></h2>
                    <p class="academy-copy">"Transforme a curiosidade em algo que você pode construir. Aprenda Rust, crie com Rullst e explore o GitHub com aulas curtas, desafios práticos e seus próprios projetos."</p>
                    <ul class="academy-subjects" aria-label="Explore os temas da Academy">
                        <li>"Rust &amp; Rullst"</li>
                        <li>"GitHub"</li>
                        <li>"IA &amp; cibersegurança"</li>
                        <li>"Robótica &amp; matemática"</li>
                    </ul>
                    <div class="academy-actions"><a class="button button-primary" href="https://academy.rullst.win/pt-BR">"Explore a Rullst Academy "<span aria-hidden="true">"↗"</span></a>" "<a class="academy-path-link" href="https://academy.rullst.win/pt-BR/paths">"Encontre sua trilha de aprendizado "<span aria-hidden="true">"↗"</span></a></div>
                    <p class="academy-note">"Português · English "<span aria-hidden="true">"/"</span>" Experimente a primeira aula de cada trilha sem criar uma conta."</p>
                </div>
                <div class="academy-companion">
                    <div class="academy-art" aria-hidden="true">
                        <div class="academy-orbit academy-orbit-outer"></div>
                        <div class="academy-orbit academy-orbit-inner"></div>" "
                        <span class="academy-spark academy-spark-one">"✦"</span>
                        <span class="academy-spark academy-spark-two">"+"</span>
                        <span class="academy-spark academy-spark-three">"✳"</span>" "
                        <span class="academy-art-label">"CURIOSIDADE, CONHEÇA SEU COMPANHEIRO."</span>" "
                        <div class="academy-crab">
                            // Rullst Academy's own mascot, adapted from its public homepage.
                            {art::academy_crab()}
                        </div>" "
                        <span class="academy-code-token">"&lt;olá, explorador /&gt;"</span>" "
                        <span class="academy-discovery"><span>"✓"</span>" Pequenos passos. Novas possibilidades."</span>
                    </div>
                    <div class="academy-companion-caption"><span>"Seu caranguejo robô. Pronto para explorar."</span><button class="academy-wave-button" type="button" data-academy-wave aria-pressed="false" hidden>"Dizer olá "<span aria-hidden="true">"↗"</span></button></div>
                </div>
            </div>
        </section>
    }
}

/// Community.
pub fn community() -> Markup {
    html! {
        <section id="community" class="shell section" data-reveal>
            <div class="community">
                <div>
                    <p class="eyebrow">"Open source. Convite aberto."</p>
                    <h2>"Seu próximo projeto."<br />"Nossa próxima "<span class="accent-green">"conquista."</span></h2>
                    <p>"Construa algo. Questione uma suposição. Traga uma reprodução. Melhore um tutorial. Você não precisa de um pull request enorme para fazer uma diferença de verdade."</p>
                    <div class="hero-actions"><a class="button button-primary" href="https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md">"Faça sua primeira contribuição ↗"</a><a class="button button-secondary" href="https://discord.com/invite/2ntKFtsSjw">"Conheça a comunidade ↗"</a></div>
                </div>
                <div class="community-mark" aria-hidden="true"><span>"{"</span><span>"você"<br />"+ rust"<br />"+ nós"</span><span>"}"</span></div>
            </div>
        </section>
    }
}

/// The release notice.
pub fn release() -> Markup {
    html! {
        <section class="shell release-card" aria-label="Antes do deploy" data-reveal>
            <h2>"Explore com ousadia."<br />"Faça deploy com responsabilidade."</h2>
            <div>
                <p>"A "<strong>"v13"</strong>" está em pré-lançamento: 13.0.0-rc.1, publicada em 26 de novembro de 2026, é o primeiro release candidate, depois do congelamento de funcionalidades da 13.0 na 13.0.0-alpha.6 (correções e documentação, e as mudanças que o mantenedor aprovou uma a uma depois do congelamento, duas delas incompatíveis, enquanto outras mudam o que um deploy da alpha.6 que funciona faz, como recusar configurações vazias em staging e produção; a "<a href="/Rullst/book/migration-v13.html#upgrading-an-alpha6-application-to-rc1">"lista de verificação da alpha.6 para a rc.1"</a>" (em inglês) mostra quais), e depois vem a 13.0.0. O Cargo nunca escolhe um pré-lançamento por conta própria, então informe a versão exata do "<a href="https://crates.io/crates/rullst">"crates.io"</a>" ("<code>"=13.0.0-rc.1"</code>") e use a tag imutável correspondente em vez do branch "<a href="https://github.com/Rullst/Rullst/tree/main">"main"</a>", que muda o tempo todo. A linha v12 chegou ao fim da vida útil em 16 de outubro de 2026 com a 12.3.0, e o "<a href="https://github.com/Rullst/Rullst/tree/v12">"branch v12"</a>" está congelado; migre as aplicações com o "<a href="/Rullst/book/migration-v13.html">"guia de adoção v12 → v13"</a>". O antigo "<a href="https://github.com/Rullst/Rullst/tree/v5">"branch v5"</a>" também não é mais mantido. A página "<a href="/Rullst/book/pt-BR/scope-and-limits.html">"Escopo e limites"</a>", traduzida para o português, lista o que o Rullst não faz por decisão de projeto e o que os testes dele não conseguem comprovar. Pré-lançamento e estável descrevem o ciclo de vida da API do framework, não uma aprovação automática da autorização, da configuração de provedores, da operação ou do deploy de uma aplicação."</p>
                <a href="https://github.com/Rullst/Rullst/blob/main/RELEASE_GUIDE.md">"Consulte o guia de lançamento ↗"</a>
            </div>
        </section>
    }
}

/// The footer, with the privacy summary and the dedication.
pub fn footer(cx: &Context) -> Markup {
    let weight = cx
        .weights
        .of(Lang::PtBr)
        .map(|weight| footprint::render(weight, Lang::PtBr));
    html! {
        <footer class="footer">
            <div class="shell">
                <div class="footer-main">
                    <div>
                        <a class="brand" href="https://rullst.win/pt-br/">"rullst"<span class="brand-period">"."</span></a>
                        <p lang="en">"Because With Rullst, We Rule!"</p>
                        <span class="footer-subtitle">"Uma suíte de frameworks open source."<br />"Construída de forma aberta, com espaço para você."</span>
                    </div>
                    <div class="footer-group">
                        <h2>"Construir"</h2>
                        <a href="https://academy.rullst.win/pt-BR">"Rullst Academy"</a>
                        <a href="/Rullst/book/">"Documentação"</a>
                        <a href="/Rullst/book/philosophy.html">"Nossa história"</a>
                        <a href="/Benchmarks/">"Benchmarks"</a>
                        <a href="/Rullst/benches/">"Microbenchmarks"</a>
                        <a href="https://github.com/Rullst/Rullst">"Código-fonte"</a>
                        <a href="https://github.com/Rullst/Rullst/blob/main/CONTRIBUTING.md">"Contribuir"</a>
                    </div>
                    <div class="footer-group">
                        <h2>"Contato"</h2>
                        <a href="mailto:officialrullst@gmail.com">"Fale com a equipe"</a>
                        <a href="https://github.com/Rullst/Rullst/security/policy">"Reportar uma vulnerabilidade"</a>
                        <a href="/pt-br/privacy.html">"Aviso de privacidade"</a>
                        <a href="/Rullst/book/compatibility-policy.html#source-branches">"Status de lançamento"</a>
                    </div>
                </div>
                <nav class="social-links" aria-label="Links sociais e da comunidade do Rullst"><a href="https://rullst.github.io">"Site ↗"</a>" "<a href="https://github.com/Rullst">"GitHub ↗"</a>" "<a href="https://rullst.blogspot.com">"Blogspot ↗"</a>" "<a href="https://www.bilibili.tv/en/space/1436672033">"BiliBili ↗"</a>" "<a href="https://bsky.app/profile/rullst.bsky.social">"Bluesky ↗"</a>" "<a href="https://daily.dev/squads/rullst">"Daily.dev ↗"</a>" "<a href="https://dev.to/venelouis">"Dev.to ↗"</a>" "<a href="https://discord.com/invite/2ntKFtsSjw">"Discord ↗"</a>" "<a href="https://rullst.hashnode.dev/">"Hashnode ↗"</a>" "<a href="https://www.instagram.com/officialrullst/">"Instagram ↗"</a>" "<a href="https://www.linkedin.com/company/rullst">"LinkedIn ↗"</a>" "<a href="https://www.reddit.com/r/rullst">"Reddit ↗"</a>" "<a href="https://substack.com/@rullst">"Substack ↗"</a>" "<a href="https://t.me/rullst">"Telegram ↗"</a>" "<a href="https://www.tiktok.com/@venelouis">"TikTok ↗"</a>" "<a href="https://www.youtube.com/@officialrullst">"YouTube ↗"</a>" "<a href="https://x.com/venelouis">"X ↗"</a></nav>
                <section id="privacy" class="privacy-notice" aria-labelledby="privacy-heading">
                    <h2 id="privacy-heading">"Pegada pequena. Aviso de privacidade claro."</h2>
                    <p>"Esta página não tem analytics, pixels de publicidade nem feeds de redes sociais incorporados. O código dela não define cookies nem usa o armazenamento do navegador. Os botões Copiar gravam na sua área de transferência apenas o comando exibido, e somente quando clicados."</p>
                    <details>
                        <summary>"Hospedagem, páginas vinculadas e seus direitos de privacidade"</summary>
                        <div>
                            <p>"O site é hospedado no GitHub Pages. As requisições chegam necessariamente ao serviço de hospedagem, que pode tratar endereços IP e informações técnicas da requisição conforme a "<a href="https://docs.github.com/pt/site-policy/privacy-policies/github-general-privacy-statement">"Declaração de Privacidade do GitHub"</a>". Não controlamos os logs de hospedagem do GitHub nem o tempo pelo qual são mantidos. As requisições a rullst.win chegam primeiro à Cloudflare, o serviço de entrega de conteúdo e de segurança na frente do GitHub Pages: ela recebe as mesmas informações conforme a "<a href="https://www.cloudflare.com/pt-br/privacypolicy/">"Política de Privacidade da Cloudflare"</a>" e, a menos que esses recursos estejam desligados para o domínio, pode acrescentar scripts próprios às páginas (ofuscação de endereços de e-mail, detecção de bots) e pede ao navegador que lhe envie relatórios de conexões que falharam."</p>
                            <p>"Os links sociais são links comuns, não conteúdo incorporado. Ao segui-los, você abre um serviço com práticas de privacidade próprias. A Rullst Academy, a documentação, os benchmarks e as demos Showcase, LMS, Portfolio e SaaS têm fluxos de dados separados. A documentação pode guardar preferências de exibição localmente e incluir recursos externos; as páginas de microbenchmarks em /Rullst/benches/ atualmente carregam o Chart.js a partir do jsDelivr. A descrição acima, de ausência de rastreamento, vale para esta página, não para todos os serviços vinculados."</p>
                            <p><strong>"Responsável por este site:"</strong>" Venelouis, Brasil. Para dúvidas ou pedidos de acesso, correção ou exclusão de dados pessoais, escreva para "<a href="mailto:officialrullst@gmail.com">"officialrullst@gmail.com"</a>". Se você enviar um e-mail para esse endereço, a equipe e o provedor de e-mail recebem seu endereço e sua mensagem para tratar do seu contato. O e-mail é gerenciado pelo Gmail; não inclua credenciais nem dados pessoais desnecessários. Issues públicas e posts em redes sociais não são canais privados de suporte. Seus direitos dependem da lei aplicável à sua situação."</p>
                            <p><a href="/pt-br/privacy.html">"Leia o aviso de privacidade completo"</a>" para conhecer finalidades, bases legais, retenção, tratamento internacional e direitos regionais, incluindo a LGPD e o GDPR quando aplicáveis. Este aviso descreve o site, e não o tratamento de dados em aplicações construídas com Rullst, e não afirma conformidade legal universal. Última atualização: 22 de novembro de 2026."</p>
                        </div>
                    </details>
                </section>
                {made_with::line(Lang::PtBr, weight)}
                <div class="footer-bottom"><span>"Feito com cuidado. Compartilhado abertamente."</span><a href="#main">"Voltar ao topo ↑"</a></div>
            </div>
            <p class="glory">"Toda glória e honra a Deus יהוה, em nome de Yeshua, o Messias (Jesus Cristo)."</p>
        </footer>
    }
}
