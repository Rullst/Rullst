//! The Brazilian Portuguese comparison page (`/pt-br/comparar/`).

use rullst::html;
use rullst::html::Markup;

use crate::pages::{Context, Lang, blocks, made_with};

/// The Brazilian Portuguese comparison page (`/pt-br/comparar/`).
pub fn page(cx: &Context, results: Markup) -> Markup {
    html! {
        <!doctype html>
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="referrer" content="no-referrer" />
                <meta name="description" content="Rullst, Loco e Topcoat lado a lado: renderização, dados, autenticação, jobs, e-mail, cobrança, admin, CLI, deploy, estabilidade e maturidade, com a versão em que cada fato foi verificado." />
                <meta http-equiv="Content-Security-Policy" content="default-src 'self'; style-src 'self'; script-src 'none'; img-src 'self'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'" />
                <meta property="og:title" content="Rullst, Loco e Topcoat comparados" />
                <meta property="og:url" content="https://rullst.win/pt-br/comparar/" />
                <meta property="og:locale" content="pt_BR" />
                <meta property="og:locale:alternate" content="en_US" />
                <title>"Rullst, Loco e Topcoat comparados — Rullst"</title>
                {blocks::head_links("https://rullst.win/pt-br/comparar/", "https://rullst.win/compare/", "https://rullst.win/pt-br/comparar/", &cx.assets.css)}
            </head>
            <body>
                {blocks::static_nav(Lang::PtBr, "/compare/", "/pt-br/comparar/", "/pt-br/#performance")}
                <main id="main" class="shell section compare-page">
                    <p class="eyebrow">"Três frameworks web em Rust. Fatos verificados."</p>
                    <h1>"Rullst, Loco e Topcoat."</h1>
                    <p>"Os três constroem aplicações web em Rust sobre o Tokio, e cada um é a melhor escolha para alguns projetos. Esta página compara os três tema a tema, com base no que a documentação de cada projeto diz. Quando um projeto não documenta algo, a página diz isso em vez de adivinhar."</p>
                    <p class="compare-checked" id="checked">"Verificado em "<time datetime="2026-11-14">"14 de novembro de 2026"</time>" com Rullst "<code>"13.0.0-alpha.5"</code>", Loco "<code>"loco-rs 1.3.1"</code>" e Topcoat "<code>"topcoat 0.12.1"</code>". Quem escreveu esta página é o mantenedor do Rullst. Se algum fato estiver errado ou desatualizado, "<a href="https://github.com/Rullst/Rullst/issues">"abra uma issue"</a>"."</p>
                    <section id="fit" aria-labelledby="fit-heading">
                        <h2 id="fit-heading">"Quando cada um faz sentido"</h2>
                        <ul class="compare-fit">
                            <li>
                                <h3>"Rullst"</h3>
                                <p>"Faz sentido quando você quer uma suíte coordenada sobre Axum e SQLx que já traga sessões e passkeys, proteção contra CSRF, cobrança (Stripe e InfinitePay), e-mail transacional, um admin de modelos e uma sala de controle local para desenvolvimento, com HTML renderizado no servidor."</p>
                                <p>"Leve em conta: a 13.0 ainda é um pré-lançamento, e o projeto foi publicado pela primeira vez em maio de 2026 pelo seu único mantenedor."</p>
                            </li>
                            <li>
                                <h3>"Loco"</h3>
                                <p>"Faz sentido quando você quer uma versão 1.x estável com convenções no estilo Rails, Sea-ORM, autenticação por JWT ou cookie, filas de jobs em Redis, PostgreSQL ou SQLite e geradores de modelos, scaffolds e workers. É a maior comunidade dos três."</p>
                                <p>"Leve em conta: pagamentos e uma interface de admin não fazem parte da documentação dele."</p>
                            </li>
                            <li>
                                <h3>"Topcoat"</h3>
                                <p>"Faz sentido quando você quer componentes renderizados no servidor cujas expressões "<code>"$(...)"</code>" também rodam no navegador como JavaScript gerado, sem bundle Wasm e sem etapa de build no cliente."</p>
                                <p>"Leve em conta: o README o descreve como inicial e experimental; autenticação, jobs em segundo plano e documentação de deploy estão no roadmap dele."</p>
                            </li>
                        </ul>
                    </section>
                    <section id="topics" aria-labelledby="topics-heading">
                        <h2 id="topics-heading">"Tema a tema"</h2>
                        {blocks::topic(
                            "topic-rendering",
                            "Modelo de renderização",
                            html! { "HTML renderizado no servidor pela macro "<code>"html!"</code>", com escape por padrão, e htmx e a build CSP do Alpine.js para interatividade. Sem bundle no cliente por padrão." },
                            html! { "Templates no servidor com Tera 2, por trás de um renderizador substituível. Também um modo SPA no cliente (Vite e React) e um modo de binário único com assets embutidos." },
                            html! { "Renderização no servidor com a macro "<code>"view!"</code>" e componentes assíncronos. As expressões "<code>"$(...)"</code>" são Rust com checagem de tipos, também traduzido para JavaScript para dar reatividade no navegador. Integrações opcionais com htmx, Alpine AJAX e Datastar." },
                        )}
                        {blocks::topic(
                            "topic-database",
                            "Banco de dados e ORM",
                            html! { "ORM próprio ("<code>"#[derive(Orm)]"</code>") sobre SQLx para SQLite, PostgreSQL e MySQL/MariaDB, com migrações "<code>"Schema"</code>". Turso/libSQL, SurrealDB e estruturas de dados do Redis por adaptadores separados." },
                            html! { "Sea-ORM 2.0 com migrações. Os drivers de banco documentados são SQLite e PostgreSQL." },
                            html! { "Sem ORM no crate "<code>"topcoat"</code>". "<code>"topcoat new --database toasty"</code>" gera uma aplicação que usa o Toasty sobre SQLite, com as migrações do Toasty." },
                        )}
                        {blocks::topic(
                            "topic-security",
                            "Autenticação e segurança padrão",
                            html! { "Hash de senha com Argon2, sessões criptografadas, passkeys, RBAC e login OAuth2/OIDC. O starter SaaS já liga o middleware de CSRF; o "<code>"rullst-security"</code>" também oferece cabeçalhos seguros, uma camada de WAF e bloqueio de login." },
                            html! { "JWT por padrão, com hash argon2 e chaves de API. Sessões em cookie assinado ou criptografado e OAuth2/OIDC com presets de GitHub e Google são opcionais. O middleware de cabeçalhos seguros existe, mas vem desligado; middleware de CSRF não está documentado." },
                            html! { "Tokens de sessão com armazenamento fornecido por você, em cookies "<code>"__Host-"</code>", Secure, HttpOnly e SameSite=Lax. Uma política de origem padrão rejeita requisições de outras origens que alteram estado. Autenticação está no roadmap; hash de senha, cabeçalhos seguros e tokens de CSRF não estão documentados." },
                        )}
                        {blocks::topic(
                            "topic-jobs",
                            "Jobs em segundo plano",
                            html! { "Filas no "<code>"rullst-core"</code>" (em processo, SQLite e Redis). O "<code>"rullst-messaging"</code>" acrescenta novas tentativas, filas de mensagens mortas, um relay de outbox e mensageria com Redis Streams." },
                            html! { "Filas em Redis, PostgreSQL ou SQLite, modos em processo e um agendador no estilo cron." },
                            html! { "Não há suporte embutido. Jobs em segundo plano estão no roadmap." },
                        )}
                        {blocks::topic(
                            "topic-mail",
                            "E-mail",
                            html! { <code>"rullst-mail"</code>": e-mail com templates via Resend, AWS SES, SendPulse ou SMTP, entregue em segundo plano, com supressão de destinatários e um kit de conformidade para o seu próprio transporte." },
                            html! { "Um mailer SMTP embutido que envia pela fila de jobs, com um modo stub para testes. Um recurso opcional de notificações acrescenta Slack e webhooks." },
                            html! { "Um recurso opcional de e-mail com transportes SMTP, arquivo (.eml) e em memória." },
                        )}
                        {blocks::topic(
                            "topic-billing",
                            "Pagamentos e cobrança",
                            html! { <code>"rullst-capital"</code>": adaptadores Stripe e InfinitePay, webhooks verificados com proteção contra replay, cotas, direitos de acesso e faturas." },
                            html! { "Não documentado." },
                            html! { "Não documentado." },
                        )}
                        {blocks::topic(
                            "topic-admin",
                            "Admin e observabilidade",
                            html! { "Nexus, um admin gerado para os modelos registrados; Studio, uma sala de controle local para desenvolvimento; exportação de traces distribuídos via OpenTelemetry, testada na CI com um OpenTelemetry Collector em um contêiner local." },
                            html! { "Nenhuma interface de admin documentada. Logs com "<code>"tracing"</code>", endpoints "<code>"/_ping"</code>", "<code>"/_health"</code>" e "<code>"/_readiness"</code>" e "<code>"cargo loco doctor"</code>"." },
                            html! { "Sem framework de admin (existe uma aplicação de demonstração de admin). Logs e endpoints de saúde não estão documentados; a geração de OpenAPI chegou na 0.12.1." },
                        )}
                        {blocks::topic(
                            "topic-cli",
                            "CLI e geradores",
                            html! { <code>"cargo rullst new"</code>" (starters Blank, API e SaaS) e geradores "<code>"make:*"</code>" para modelos, migrações, controllers, resources, workers, e-mail, cobrança, MFA, OAuth e outros; "<code>"dev"</code>", "<code>"doctor"</code>" e "<code>"upgrade"</code>" assistido." },
                            html! { <code>"loco new"</code>" (templates SaaS, REST API, lightweight e advanced) e "<code>"cargo loco generate"</code>" para modelos, migrações, scaffolds, controllers, tasks, workers, mailers e arquivos de deploy." },
                            html! { <code>"topcoat new"</code>", "<code>"dev"</code>", "<code>"fmt"</code>", "<code>"ui"</code>" (copia componentes para o projeto), "<code>"asset bundle"</code>" e "<code>"l10n check"</code>". Geradores de modelos ou controllers não estão documentados." },
                        )}
                        {blocks::topic(
                            "topic-deployment",
                            "Orientação de deploy",
                            html! { "Guias para DigitalOcean, Hetzner, Oracle Cloud e Google Cloud. A CI testa o deploy gerado para VM dos starters Blank e SaaS, com SQLite, no Docker e no Podman em um runner x86_64; máquinas Arm não são testadas, e os guias dos provedores foram escritos a partir da documentação deles, sem serem seguidos de ponta a ponta em contas reais." },
                            html! { "Um binário de release, mais configurações geradas de Docker, nginx ou AWS Lambda, e "<code>"loco doctor"</code>" antes de ir para produção." },
                            html! { "A documentação de deploy está no roadmap. Há um exemplo para Cloudflare Workers e hospedagem própria via "<code>"serve()"</code>" ou "<code>"Router::handle"</code>"." },
                        )}
                        {blocks::topic(
                            "topic-stability",
                            "Estabilidade, versionamento e janela de suporte",
                            html! { "A 13.0 está em pré-lançamento; a 13.0.0-alpha.6 congelou as funcionalidades e a API pública, e a 13.0.0-rc.1 traz correções, documentação e as mudanças que o mantenedor aprovou uma a uma depois do congelamento, duas delas incompatíveis. Depois da 13.0.0, o mantenedor pretende manter a 13.x como linha ativa por 12 meses e, depois da 14.0.0, dar a ela correções de segurança por mais 1 a 3 meses (uma intenção, não uma garantia). Versionamento semântico; MSRV Rust 1.96." },
                            html! { "A 1.0.0 (julho de 2026) foi a primeira versão estável. As notas de atualização listam mudanças incompatíveis; uma declaração de semver e uma janela de suporte não estão documentadas. MSRV Rust 1.94." },
                            html! { "O README diz: inicial e experimental, espere mudanças incompatíveis. O changelog diz que segue o versionamento semântico. Nenhuma janela de suporte documentada. MSRV Rust 1.99." },
                        )}
                        {blocks::topic(
                            "topic-maturity",
                            "Maturidade e comunidade",
                            html! { "Publicado pela primeira vez em maio de 2026. Licença MIT. 21 estrelas e 7 contribuidores no GitHub em 14 de novembro de 2026." },
                            html! { "Primeira versão em novembro de 2023. Licença Apache-2.0. 9.629 estrelas e 157 contribuidores no GitHub em 14 de novembro de 2026." },
                            html! { "Repositório criado em abril de 2026, primeira versão funcional em julho de 2026, na organização tokio-rs do GitHub. Licença MIT. 7.221 estrelas e 41 contribuidores no GitHub em 14 de novembro de 2026." },
                        )}
                    </section>
                    <section id="ui-frameworks" aria-labelledby="ui-frameworks-heading">
                        <h2 id="ui-frameworks-heading">"Por que não Leptos ou Dioxus?"</h2>
                        <p class="compare-checked" id="checked-ui">"Verificado em "<time datetime="2026-11-19">"19 de novembro de 2026"</time>" com Leptos "<code>"leptos 0.8.23"</code>" e Dioxus "<code>"dioxus 0.7.10"</code>"."</p>
                        <p>"Leptos e Dioxus são full-stack em outro sentido: são frameworks de interface. Os mesmos componentes Rust podem renderizar HTML no servidor e depois rodar no navegador como WebAssembly, que hidrata esse HTML, e as server functions permitem que o navegador chame código Rust que roda só no servidor. O Dioxus também tem como alvo desktop e dispositivos móveis."</p>
                        <p>"Por serem frameworks de interface, nenhum dos dois traz ORM, migrações ou autenticação, e nenhum documenta e-mail, jobs em segundo plano ou painel de admin. O livro do Leptos diz que o próprio Leptos não fornece nem o servidor HTTP nem o banco de dados; a documentação do Dioxus diz que ele não oferece uma forma embutida de gerenciar autenticação, e o tutorial de banco de dados dele usa o "<code>"rusqlite"</code>" e cita SQLx e SeaORM entre as alternativas. As aplicações montam essas partes com outros crates: o exemplo de autenticação do Leptos e o guia de autenticação do Dioxus usam o "<code>"axum_session_auth"</code>" com SQLx."</p>
                        <p>"Eles não estão na seção Desempenho porque ela mede aplicações equivalentes atendendo requisições HTTP. O lado servidor de uma aplicação Leptos roda sobre Actix Web ou Axum, pelas integrações do Leptos, e o Dioxus Fullstack é construído sobre o Axum, que já é a referência do benchmark; uma aplicação equivalente ainda precisaria de ORM e autenticação montados à mão. As abordagens se complementam: uma interface em Leptos ou Dioxus pode consumir uma API HTTP do Rullst."</p>
                    </section>
                    <section id="benchmarks" aria-labelledby="benchmarks-heading">
                        <h2 id="benchmarks-heading">"Desempenho"</h2>
                        <p>"Estes números não são escritos à mão: cada build diário deste site insere os resultados publicados mais recentes da suíte "<a href="/Benchmarks/">"Rullst/Benchmarks"</a>" para os três frameworks, o Actix Web e o Gin, além de uma configuração mínima do Rullst. O mantenedor do Rullst também mantém esse benchmark e escreveu todas as implementações dele."</p>
                        <div class="benchmark-howto">
                            <h3>"Como ler estes números"</h3>
                            <ul>
                                <li><strong>"1,00× é o Axum."</strong>" Cada pontuação divide o resultado de um framework pelo resultado do Axum, um framework Rust pequeno usado como referência, medido na mesma máquina em rodadas alternadas. 0,50× é metade do resultado do Axum e 2,00× é o dobro. Mais é melhor em todas as pontuações: em memória e tempo de inicialização a razão é invertida, então uma pontuação maior significa menos RAM ou uma inicialização mais rápida."</li>
                                <li><strong>"Empates e ±."</strong>" As máquinas são máquinas de nuvem compartilhadas, então os resultados variam de uma execução para outra. \"±6%\" ao lado de uma pontuação é a margem dentro da qual duas pontuações não podem ser distinguidas. Pontuações tão próximas dividem um intervalo de posições, como \"3–7\": essas implementações estão nas posições 3 a 7, sem ordem entre elas."</li>
                                <li><strong>"As categorias respondem a perguntas diferentes."</strong>" Hello world: quanto um framework custa por requisição quando a página é trivial. Banco de dados: como ele lê, grava e renderiza linhas. Aplicação: o que ele faz com uma requisição real (entrada, validação, uma gravação, um formulário com proteção contra CSRF, um usuário autenticado). Memória: quanta RAM ele precisa sob carga. Inicialização: quão rápido ele inicia. Estabilidade (passou ou falhou): se ele continua funcionando sob carga mais pesada."</li>
                                <li><strong>"O banco de dados costuma vir primeiro."</strong>" Numa aplicação real, o banco de dados costuma ser o limite antes do framework, então uma diferença grande em Hello world diminui ou desaparece quando as requisições leem e gravam dados; Banco de dados e Aplicação estão mais perto do que uma aplicação faz."</li>
                                <li><strong>"A pontuação geral é um resumo."</strong>" Ela combina as cinco pontuações de categoria, com Hello world, Banco de dados e Aplicação contados duas vezes e Memória e Inicialização uma vez, para uma leitura rápida. As categorias são o resultado de verdade: outros pesos mudam a ordem, como mostram os resultados completos."</li>
                                <li><strong>"O Rullst aparece duas vezes."</strong>" \"Rullst\" é a aplicação que o "<code>"cargo rullst new"</code>" gera, com o middleware de produção que ele liga: WAF, proteção contra CSRF, cookies seguros, cabeçalhos seguros e log de acesso; é o Rullst com posição. \"Rullst (mínimo)\" usa o mesmo roteador, o mesmo ORM e os mesmos templates "<code>"html!"</code>" sem essas camadas, servido como o app do Axum; é um diagnóstico, com pontuações e sem posição. A diferença entre as duas linhas é o custo dessas camadas."</li>
                            </ul>
                        </div>
                        <div class="benchmark-results" data-benchmark-results>
                            {results}
                        </div>
                    </section>
                    <section id="sources" aria-labelledby="sources-heading">
                        <h2 id="sources-heading">"Fontes"</h2>
                        <ul>
                            <li>"Rullst: "<a href="https://github.com/Rullst/Rullst#integrations">"integrações no README"</a>", "<a href="/Rullst/book/maturity.html">"níveis de maturidade"</a>", "<a href="/Rullst/book/compatibility-policy.html#supported-release-window">"janela de versões suportadas"</a>", "<a href="/Rullst/book/database-support.html">"suporte a bancos de dados"</a>", "<a href="https://crates.io/crates/rullst">"crates.io"</a>" (documentação em inglês)."</li>
                            <li>"Loco: "<a href="https://loco.rs/docs/">"documentação"</a>", "<a href="https://github.com/loco-rs/loco/tree/v1.3.1">"repositório na v1.3.1"</a>", "<a href="https://crates.io/crates/loco-rs">"crates.io"</a>"."</li>
                            <li>"Topcoat: "<a href="https://github.com/tokio-rs/topcoat/tree/v0.12.1">"repositório na v0.12.1"</a>" (README, "<code>"docs/"</code>", changelog), "<a href="https://docs.rs/topcoat/0.12.1">"docs.rs"</a>", "<a href="https://crates.io/crates/topcoat">"crates.io"</a>"."</li>
                            <li>"Leptos: "<a href="https://book.leptos.dev/">"o livro do Leptos"</a>" ("<a href="https://book.leptos.dev/ssr/22_life_cycle.html">"o ciclo de vida de um carregamento de página"</a>", "<a href="https://book.leptos.dev/server/25_server_functions.html">"server functions"</a>", "<a href="https://book.leptos.dev/server/26_extractors.html">"extractors"</a>"), "<a href="https://github.com/leptos-rs/leptos/tree/v0.8.23/projects/session_auth_axum">"exemplo de autenticação na v0.8.23"</a>", "<a href="https://crates.io/crates/leptos">"crates.io"</a>"."</li>
                            <li>"Dioxus: "<a href="https://dioxuslabs.com/learn/0.7/">"documentação da 0.7"</a>" ("<a href="https://dioxuslabs.com/learn/0.7/guides/platforms/web/">"web"</a>", "<a href="https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/">"roteador e estado"</a>", "<a href="https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/">"autenticação"</a>", "<a href="https://dioxuslabs.com/learn/0.7/tutorial/databases/">"bancos de dados"</a>"), "<a href="https://crates.io/crates/dioxus">"crates.io"</a>"."</li>
                        </ul>
                    </section>
                </main>
                <footer class="shell footer-bottom"><a href="/pt-br/">"Página inicial do Rullst"</a>{made_with::short(Lang::PtBr)}<a href="mailto:officialrullst@gmail.com">"Fale com a equipe"</a></footer>
            </body>
        </html>
    }
}
