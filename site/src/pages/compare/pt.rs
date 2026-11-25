//! The Brazilian Portuguese comparison page (`/pt-br/comparar/`).

use rullst::html;
use rullst::html::Markup;

use crate::pages::blocks::{self, Choice};
use crate::pages::{Context, Lang, made_with};

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
                    <p class="eyebrow"><span class="short-line" aria-hidden="true"></span>" Três frameworks web em Rust. Fatos verificados."</p>
                    <h1>"Rullst, Loco e Topcoat,"<br /><span class="accent-green">"o que cada um escolheu e quanto isso custa."</span></h1>
                    <p class="compare-lede">"Os três constroem aplicações web em Rust sobre o Tokio, e cada um é a melhor escolha para alguns projetos. Em cada tema, esta página diz em palavras simples o que ele é e depois o que cada framework escolheu, a vantagem dessa escolha e o custo dela. A escolha vem da documentação de cada projeto; quando um projeto não documenta algo, a página diz isso em vez de adivinhar. A vantagem e o custo são a leitura de quem escreve a página sobre essa escolha, não uma medição."</p>
                    <p class="compare-checked" id="checked">"Verificado em "<time datetime="2026-11-25">"25 de novembro de 2026"</time>" com Rullst "<code>"13.0.0-rc.1"</code>" (o código-fonte no "<code>"main"</code>"), Loco "<code>"loco-rs 1.3.1"</code>" e Topcoat "<code>"topcoat 0.12.1"</code>", a versão mais recente de cada um nesse dia. O site de documentação do Loco ainda mostrava as páginas da 1.2.0 nesse dia, então o que diz respeito a sessões por cookie, OAuth e notificações vem dos códigos-fonte da 1.3.1 no repositório dele. Quem escreveu esta página é o mantenedor do Rullst. Se algum fato estiver errado ou desatualizado, "<a href="https://github.com/Rullst/Rullst/issues">"abra uma issue"</a>"."</p>
                    <ul class="compare-jump" aria-label="Temas">
                        <li><a href="#fit">"Quando cada um faz sentido"</a></li><li><a href="#topic-rendering">"Renderização"</a></li><li><a href="#topic-database">"Banco de dados"</a></li><li><a href="#topic-security">"Segurança"</a></li><li><a href="#topic-jobs">"Jobs"</a></li><li><a href="#topic-mail">"E-mail"</a></li><li><a href="#topic-billing">"Pagamentos"</a></li><li><a href="#topic-admin">"Admin"</a></li><li><a href="#topic-cli">"CLI"</a></li><li><a href="#topic-deployment">"Deploy"</a></li><li><a href="#topic-stability">"Estabilidade"</a></li><li><a href="#topic-maturity">"Comunidade"</a></li><li><a href="#benchmarks">"Desempenho"</a></li>
                    </ul>
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
                                <p>"Leve em conta: o README o descreve como inicial e experimental; autenticação e jobs em segundo plano estão no roadmap dele, assim como a documentação de deploy."</p>
                            </li>
                        </ul>
                    </section>
                    <section id="topics" aria-labelledby="topics-heading">
                        <h2 id="topics-heading">"Tema a tema"</h2>
                        {blocks::topic(Lang::PtBr,
                            "topic-rendering",
                            "Modelo de renderização",
                            html! { <strong>"Em palavras simples: "</strong>"como uma página chega ao navegador: como HTML pronto, montado no servidor, ou como um programa que monta a página no navegador. Isso decide quanto JavaScript você escreve e envia." },
                            [
                                Choice {
                                    chose: html! { "HTML renderizado no servidor pela macro "<code>"html!"</code>", com escape por padrão, e htmx e a build CSP do Alpine.js para interatividade. Sem bundle no cliente por padrão." },
                                    plus: html! { "As páginas funcionam com pouco ou nenhum JavaScript, e o texto vindo de usuários é escapado, a menos que você o marque como confiável." },
                                    minus: html! { "Comportamento rico no navegador além de htmx e Alpine dá mais trabalho, e renderizar listas longas com "<code>"html!"</code>" é um dos alvos de desempenho da 13.1." },
                                },
                                Choice {
                                    chose: html! { "Templates no servidor com Tera 2, por trás de um renderizador substituível. Também um modo SPA no cliente (React) e uma opção de embutir os assets no binário." },
                                    plus: html! { "Uma linguagem de templates conhecida, e a escolha entre páginas no servidor e uma SPA em React ao criar o projeto." },
                                    minus: html! { "Uma SPA em React exige ferramentas de JavaScript próprias, então os dois estilos são uma decisão a tomar cedo." },
                                },
                                Choice {
                                    chose: html! { "Renderização no servidor com a macro "<code>"view!"</code>" e componentes assíncronos. As expressões "<code>"$(...)"</code>" são Rust com checagem de tipos, também traduzido para JavaScript para dar reatividade no navegador, sem bundle Wasm e sem etapa de build no cliente. Integrações opcionais com htmx, Alpine AJAX e Datastar." },
                                    plus: html! { "Comportamento no navegador escrito em Rust, sem um build de cliente separado." },
                                    minus: html! { "O README o descreve como inicial e experimental: espere mudanças que quebram a compatibilidade." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-database",
                            "Banco de dados e ORM",
                            html! { <strong>"Em palavras simples: "</strong>"a maioria das aplicações guarda os dados num banco de dados. Um ORM liga tabelas a tipos Rust para você escrever menos SQL, e os bancos que ele suporta decidem onde você pode rodar." },
                            [
                                Choice {
                                    chose: html! { "ORM próprio ("<code>"#[derive(Orm)]"</code>") sobre o SQLx para SQLite, PostgreSQL e MySQL/MariaDB, com migrações "<code>"Schema"</code>". Turso/libSQL, SurrealDB e estruturas de dados do Redis por adaptadores separados." },
                                    plus: html! { "Um ORM e um sistema de migrações, sobre mais tipos de banco do que os outros documentam, com SQLx parametrizado por baixo." },
                                    minus: html! { "Um ORM escrito para o Rullst, e não um ORM de terceiros já estabelecido, então você aprende a API dele, e ele tem um único mantenedor." },
                                },
                                Choice {
                                    chose: html! { "Sea-ORM 2.0 com migrações. Os drivers de banco documentados são SQLite e PostgreSQL." },
                                    plus: html! { "Usa um ORM que é um projeto à parte." },
                                    minus: html! { "MySQL não está entre os drivers documentados." },
                                },
                                Choice {
                                    chose: html! { "Sem ORM no crate "<code>"topcoat"</code>". "<code>"topcoat new --database toasty"</code>" gera uma aplicação que usa o Toasty sobre SQLite, com as migrações do Toasty." },
                                    plus: html! { "Um projeto novo já começa com uma camada de banco e migrações ligadas." },
                                    minus: html! { "Documentado só para SQLite pelo Toasty; uma integração melhor com o Toasty está no roadmap." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-security",
                            "Autenticação e padrões de segurança",
                            html! { <strong>"Em palavras simples: "</strong>"o hash guarda senhas de modo que um banco roubado não as revele (Argon2 é um deliberadamente lento). Uma sessão criptografada faz com que um cookie roubado não possa ser lido nem alterado. Uma passkey troca a senha por uma chave no seu dispositivo. RBAC deixa papéis decidirem quem pode o quê, e OAuth2/OIDC é o \"entrar com o Google\". Cabeçalhos seguros, proteção CSRF e um WAF (um filtro de requisições hostis) bloqueiam ataques comuns. Uma proteção ligada em toda requisição também custa tempo em toda requisição." },
                            [
                                Choice {
                                    chose: html! { "Hash de senhas com Argon2 e sessões criptografadas no "<code>"rullst-auth"</code>"; passkeys, RBAC e login OAuth2/OIDC quando você precisar. Em todo starter gerado, o "<code>"Server"</code>" monta cabeçalhos seguros e proteção CSRF com double-submit em todos os ambientes, e um WAF que recusa requisições suspeitas em staging e produção e só as registra em desenvolvimento; o "<code>"rullst-security"</code>" acrescenta um bloqueio de tentativas de login (login jail) e mais camadas." },
                                    plus: html! { "Mais proteções ligadas por padrão: uma aplicação nova já começa com elas e um teste gerado as verifica, em vez de você achar e ligar cada uma." },
                                    minus: html! { "Toda requisição paga por elas, o que reduz a vazão (veja "<a href="#benchmarks">"Desempenho"</a>"). Um único mantenedor, nenhuma auditoria externa, e as regras de autorização da sua aplicação continuam sendo suas." },
                                },
                                Choice {
                                    chose: html! { "JWT por padrão, com hash Argon2id e chaves de API. Sessões por cookie assinado ou criptografado opcionais, e OAuth2 com presets de GitHub e Google. O middleware de cabeçalhos seguros (presets como "<code>"github"</code>" e "<code>"owasp"</code>") existe, mas vem desativado por padrão; middleware CSRF não é documentado. Ligados por padrão: limite de payload, captura de panics, ETag, ids de requisição e logs." },
                                    plus: html! { "Tokens sem estado servem bem a clientes de API, e os cabeçalhos seguros têm presets prontos que você pode ligar." },
                                    minus: html! { "Os cabeçalhos seguros ficam desligados até você ligá-los, e nenhuma proteção CSRF nem WAF é documentada, então uma aplicação que usa sessões por cookie acrescenta as suas." },
                                },
                                Choice {
                                    chose: html! { "Tokens de sessão com armazenamento que você fornece, em cookies "<code>"__Host-"</code>", Secure, HttpOnly, SameSite=Lax. Uma política de origem padrão rejeita requisições entre origens que alterem estado. Autenticação está no roadmap; hash de senhas, cabeçalhos seguros e tokens CSRF não são documentados." },
                                    plus: html! { "Configuração rigorosa de cookies e uma verificação de origem ligada por padrão, numa superfície pequena." },
                                    minus: html! { "Ainda sem autenticação embutida: você traz o armazenamento, o tratamento de senhas e os cabeçalhos. A política de origem não protege uma requisição GET que altere estado." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-jobs",
                            "Jobs em segundo plano",
                            html! { <strong>"Em palavras simples: "</strong>"trabalho que não deve fazer o visitante esperar, como enviar e-mail ou rodar uma tarefa noturna. Uma fila guarda esses jobs e os executa em segundo plano, tentando de novo quando um falha." },
                            [
                                Choice {
                                    chose: html! { "Filas no "<code>"rullst-core"</code>" (em processo, SQLite e Redis). O "<code>"rullst-messaging"</code>" acrescenta novas tentativas, filas de mensagens mortas, um relay de outbox e mensageria com Redis Streams." },
                                    plus: html! { "Novas tentativas e filas de mensagens mortas (um lugar para jobs que continuam falhando) vêm com a suíte." },
                                    minus: html! { "Redis Streams é o único adaptador de broker remoto, e o relay de outbox não faz do banco e do broker uma única transação atômica." },
                                },
                                Choice {
                                    chose: html! { "Filas em Redis, PostgreSQL ou SQLite, modos em processo e um agendador no estilo cron." },
                                    plus: html! { "Um agendador para jobs recorrentes e a escolha entre três armazenamentos de fila." },
                                    minus: html! { "Outros brokers não são documentados." },
                                },
                                Choice {
                                    chose: html! { "Não embutido. Jobs em segundo plano estão no roadmap." },
                                    plus: html! { "Nada a configurar se você não precisa de jobs." },
                                    minus: html! { "Traga a sua fila, ou espere o item do roadmap." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-mail",
                            "E-mail",
                            html! { <strong>"Em palavras simples: "</strong>"redefinição de senha e recibos são e-mails. Um motor de e-mail preenche um template e entrega a mensagem a um serviço de envio." },
                            [
                                Choice {
                                    chose: html! { <code>"rullst-mail"</code>": e-mail com templates por Resend, AWS SES, SendPulse ou SMTP, entregue em segundo plano, com supressão de destinatários e um kit de conformidade para o seu próprio transporte." },
                                    plus: html! { "Vários serviços atrás de uma só interface, com uma lista de supressão para endereços que você decide não escrever de novo." },
                                    minus: html! { "Resend e SMTP são usados pelas aplicações em produção do mantenedor; AWS SES e SendPulse só foram testados offline." },
                                },
                                Choice {
                                    chose: html! { "Um enviador SMTP embutido que envia pela fila de jobs, com modo stub para testes. Um recurso opcional de notificações acrescenta Slack e webhooks." },
                                    plus: html! { "O e-mail passa pela fila de jobs, e o stub facilita os testes." },
                                    minus: html! { "SMTP é o transporte documentado para entrega real." },
                                },
                                Choice {
                                    chose: html! { "Um recurso opcional de e-mail com transportes SMTP, de arquivo (.eml) e em memória." },
                                    plus: html! { "Os transportes de arquivo e em memória deixam testar e-mail sem enviar nada." },
                                    minus: html! { "SMTP é o único dos três que envia e-mail de verdade." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-billing",
                            "Pagamentos e cobrança",
                            html! { <strong>"Em palavras simples: "</strong>"cobrar clientes, receber com segurança as notificações do provedor de pagamento (webhooks) e controlar o que cada cliente pode usar." },
                            [
                                Choice {
                                    chose: html! { <code>"rullst-capital"</code>": adaptadores de Stripe e InfinitePay, webhooks verificados com proteção contra repetição, cotas, direitos de uso e faturas." },
                                    plus: html! { "Código de pagamento que você não escreve, com verificação de assinatura de webhooks e proteção contra repetição." },
                                    minus: html! { "O Stripe tem compras reais no SaaS do mantenedor, mas nenhum teste automatizado contra uma conta real; o InfinitePay foi validado com um pagamento Pix real. Outro gateway significa escrever um adaptador." },
                                },
                                Choice {
                                    chose: html! { "Não documentado." },
                                    plus: html! { "Nenhum código de pagamento no framework: use a biblioteca de qualquer provedor." },
                                    minus: html! { "Você constrói e protege a integração, webhooks incluídos." },
                                },
                                Choice {
                                    chose: html! { "Não documentado." },
                                    plus: html! { "Nenhum código de pagamento no framework: use a biblioteca de qualquer provedor." },
                                    minus: html! { "Você constrói e protege a integração, webhooks incluídos." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-admin",
                            "Admin e observabilidade",
                            html! { <strong>"Em palavras simples: "</strong>"uma tela de admin deixa a equipe editar dados sem código. Observabilidade é como você vê o que a aplicação em execução está fazendo: logs, verificações de saúde e traces." },
                            [
                                Choice {
                                    chose: html! { "Nexus, um admin gerado para os modelos registrados; Studio, uma sala de controle local para desenvolvimento; exportação OpenTelemetry para traces distribuídos, testada na CI contra um OpenTelemetry Collector em um contêiner local." },
                                    plus: html! { "Um admin e telemetria ao vivo vêm com o framework, e os traces usam um formato padrão." },
                                    minus: html! { "O Studio é uma ferramenta local de desenvolvimento, não um painel de produção, e o admin é mais uma superfície a proteger com papéis." },
                                },
                                Choice {
                                    chose: html! { "Nenhuma interface de admin documentada. Logs com "<code>"tracing"</code>", endpoints "<code>"/_ping"</code>", "<code>"/_health"</code>" e "<code>"/_readiness"</code>" e "<code>"cargo loco doctor"</code>"." },
                                    plus: html! { "Endpoints de saúde que um balanceador de carga pode usar e um comando que verifica a sua configuração." },
                                    minus: html! { "Sem interface de admin: você constrói ou adiciona uma." },
                                },
                                Choice {
                                    chose: html! { "Nenhum framework de admin (existe uma aplicação de demonstração de admin). Logs e endpoints de saúde não são documentados; a geração de OpenAPI chegou na 0.12.1." },
                                    plus: html! { "Pode gerar uma descrição OpenAPI da sua API." },
                                    minus: html! { "Admin, orientação sobre logs e endpoints de saúde não são documentados." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-cli",
                            "CLI e geradores",
                            html! { <strong>"Em palavras simples: "</strong>"uma ferramenta de linha de comando cria o projeto e escreve código repetitivo (modelos, controllers) para você começar de uma estrutura que funciona." },
                            [
                                Choice {
                                    chose: html! { <code>"cargo rullst new"</code>" (starters Blank, API e SaaS) e geradores "<code>"make:*"</code>" para modelos, migrações, controllers, resources, workers, e-mail, cobrança, MFA, OAuth e mais; "<code>"dev"</code>", "<code>"doctor"</code>" e "<code>"upgrade"</code>" assistido." },
                                    plus: html! { "O código gerado é Rust comum que você possui, e o "<code>"upgrade"</code>" ajuda a mudar de versão." },
                                    minus: html! { "Muitos comandos para aprender, e o código gerado é seu para manter." },
                                },
                                Choice {
                                    chose: html! { <code>"loco new"</code>" (SaaS com renderização no servidor e no cliente, REST API, templates leve e avançado) e "<code>"cargo loco generate"</code>" para modelos, migrações, scaffolds, controllers, tarefas, workers, mailers e arquivos de deploy." },
                                    plus: html! { "Geradores no estilo Rails para quase tudo, arquivos de deploy incluídos." },
                                    minus: html! { "Os geradores só rodam em builds de debug, e o template é escolhido de forma interativa." },
                                },
                                Choice {
                                    chose: html! { <code>"topcoat new"</code>", "<code>"dev"</code>", "<code>"fmt"</code>", "<code>"ui"</code>" (copia componentes para o projeto), "<code>"asset bundle"</code>" e "<code>"l10n check"</code>". Geradores de modelo ou controller não são documentados." },
                                    plus: html! { "Componentes copiados para o seu projeto são código seu, e os arquivos de tradução podem ser verificados." },
                                    minus: html! { "Você escreve modelos e controllers à mão." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-deployment",
                            "Orientação de deploy",
                            html! { <strong>"Em palavras simples: "</strong>"colocar a aplicação numa máquina que as pessoas alcancem: um build, um servidor, HTTPS e um modo de reiniciá-la." },
                            [
                                Choice {
                                    chose: html! { "Guias para DigitalOcean, Hetzner, Oracle Cloud e Google Cloud, e um Dockerfile, Compose e Caddy gerados. A CI testa o deploy em VM gerado dos starters Blank e SaaS, em SQLite, com Docker e Podman somente em um runner x86_64. As aplicações em produção do mantenedor, Academy, capoeira e vene.win, rodam em uma máquina Arm Oracle Ampere A1, atrás de um Caddy compartilhado com certificados Let's Encrypt; nenhum teste da CI roda em Arm. Os guias de provedores foram escritos a partir da documentação dos provedores, sem serem seguidos de ponta a ponta em contas reais." },
                                    plus: html! { "Um caminho de deploy testado e uma configuração real de produção em uma máquina Arm que a CI não cobre." },
                                    minus: html! { "Máquinas Arm, domínios reais e Let's Encrypt, contas de provedores, reinicializações e deploys com PostgreSQL ou MySQL não são testados na CI, e os guias de provedores não foram experimentados em contas reais." },
                                },
                                Choice {
                                    chose: html! { "Um binário de release, mais configurações geradas de Docker, nginx ou AWS Lambda, e "<code>"loco doctor"</code>" antes de ir ao ar." },
                                    plus: html! { "Arquivos de deploy gerados e um comando que verifica a configuração." },
                                    minus: html! { "Não encontrei em quais hospedagens esses arquivos gerados foram testados." },
                                },
                                Choice {
                                    chose: html! { "A documentação de deploy está no roadmap. Um exemplo em Cloudflare Workers e hospedagem própria por "<code>"serve()"</code>" ou "<code>"Router::handle"</code>"." },
                                    plus: html! { "Um exemplo o roda no Cloudflare Workers, e você pode hospedá-lo do seu jeito." },
                                    minus: html! { "Ainda sem guia de deploy." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-stability",
                            "Estabilidade, versionamento e janela de suporte",
                            html! { <strong>"Em palavras simples: "</strong>"o quanto uma atualização pode quebrar o seu código e por quanto tempo uma versão continua recebendo correções." },
                            [
                                Choice {
                                    chose: html! { "A 13.0 está em pré-lançamento; a 13.0.0-alpha.6 congelou seus recursos e a API pública, e a 13.0.0-rc.1, o primeiro candidato a lançamento, acrescenta correções, documentação e as mudanças que o mantenedor aprovou uma a uma depois do congelamento: duas delas quebram a API, e outras mudam o que um deploy funcionando da alpha.6 faz, como recusar configurações vazias em staging e produção. Depois da 13.0.0, o mantenedor pretende manter a 13.x como linha ativa por 12 meses e depois dar correções de segurança por mais 1 a 3 meses após a 14.0.0 (uma intenção, não uma garantia). Versionamento Semântico; MSRV Rust 1.96." },
                                    plus: html! { "Uma intenção de suporte declarada e Versionamento Semântico, com um checklist de migração da alpha.6 para a rc.1." },
                                    minus: html! { "Ainda não é final, e a janela é uma intenção, não uma garantia." },
                                },
                                Choice {
                                    chose: html! { "A 1.0.0 (julho de 2026) foi a primeira versão estável. As notas de atualização listam as mudanças que quebram a compatibilidade; uma declaração de semver e uma janela de suporte não são documentadas. MSRV Rust 1.94." },
                                    plus: html! { "Uma linha 1.x estável com notas de atualização e um requisito de compilador mais antigo que o dos outros." },
                                    minus: html! { "Nenhuma janela de suporte é documentada." },
                                },
                                Choice {
                                    chose: html! { "O README diz: inicial e experimental, espere mudanças que quebram a compatibilidade. O changelog diz que segue o Versionamento Semântico e marca o que quebra. Nenhuma janela de suporte documentada. MSRV Rust 1.99." },
                                    plus: html! { "O changelog marca o que quebra." },
                                    minus: html! { "Espere mudanças que quebram a compatibilidade, e ele exige um compilador Rust bem recente." },
                                },
                            ])}
                        {blocks::topic(Lang::PtBr,
                            "topic-maturity",
                            "Maturidade e comunidade",
                            html! { <strong>"Em palavras simples: "</strong>"uma comunidade maior significa mais respostas, exemplos e gente corrigindo bugs; uma menor significa que você pode ficar por conta própria." },
                            [
                                Choice {
                                    chose: html! { "Publicado pela primeira vez em maio de 2026. Licença MIT. 21 estrelas no GitHub e 7 colaboradores em 25 de novembro de 2026; dois dos colaboradores são bots e um é a conta de um agente de programação de IA; o Rullst tem um único mantenedor." },
                                    plus: html! { "Uma licença MIT e um nível de maturidade documentado para cada pacote." },
                                    minus: html! { "A menor comunidade dos três, um único mantenedor que é um ponto único de falha e nenhuma auditoria externa." },
                                },
                                Choice {
                                    chose: html! { "Primeira versão em novembro de 2023. Licença Apache-2.0. 9.667 estrelas no GitHub e 157 colaboradores em 25 de novembro de 2026." },
                                    plus: html! { "A maior comunidade dos três e o histórico de versões mais longo." },
                                    minus: html! { "O site de documentação estava atrasado em relação à versão mais recente quando foi verificado (páginas da 1.2.0 com a 1.3.1 já publicada), então alguns recursos só estão documentados no repositório." },
                                },
                                Choice {
                                    chose: html! { "Repositório criado em abril de 2026, primeira versão funcional em julho de 2026, na organização tokio-rs do GitHub. Licença MIT. 7.317 estrelas no GitHub e 41 colaboradores em 25 de novembro de 2026." },
                                    plus: html! { "Mantido dentro da organização tokio-rs, com uma base de colaboradores crescente." },
                                    minus: html! { "Um projeto jovem na versão 0.x." },
                                },
                            ])}
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
                        <p>"Estes números não são escritos à mão: cada build diário deste site insere os resultados mais recentes da suíte "<a href="/Benchmarks/">"Rullst/Benchmarks"</a>" para os três frameworks, o Actix Web e o Gin, além de uma configuração mínima do Rullst. O mantenedor do Rullst também mantém esse benchmark e escreveu todas as implementações dele. A página de resultados dele diz se estes resultados passam pela porta de publicação do próprio benchmark."</p>
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
                            <li>"Rullst: "<a href="https://github.com/Rullst/Rullst#integrations">"integrações no README"</a>", "<a href="/Rullst/book/maturity.html">"níveis de maturidade"</a>", "<a href="/Rullst/book/compatibility-policy.html#supported-release-window">"janela de versões suportadas"</a>", "<a href="/Rullst/book/database-support.html">"suporte a bancos de dados"</a>", "<a href="https://crates.io/crates/rullst">"crates.io"</a>", "<a href="/Rullst/book/security-layers.html">"camadas de segurança"</a>" e "<a href="/Rullst/book/scope-and-limits.html#external-accounts-and-providers">"escopo e limites"</a>" (documentação em inglês)."</li>
                            <li>"Loco: "<a href="https://loco.rs/docs/">"documentação"</a>", "<a href="https://github.com/loco-rs/loco/tree/v1.3.1">"repositório na v1.3.1"</a>", "<a href="https://crates.io/crates/loco-rs">"crates.io"</a>", a "<a href="https://loco.rs/docs/reference/middleware/">"referência de middleware"</a>", "<a href="https://loco.rs/docs/how-to/jwt-auth/">"autenticação JWT"</a>" e as "<a href="https://github.com/loco-rs/loco/blob/v1.3.1/Cargo.toml">"feature flags da 1.3.1"</a>"."</li>
                            <li>"Topcoat: "<a href="https://github.com/tokio-rs/topcoat/tree/v0.12.1">"repositório na v0.12.1"</a>" (README, "<code>"docs/"</code>", changelog), "<a href="https://docs.rs/topcoat/0.12.1">"docs.rs"</a>", "<a href="https://crates.io/crates/topcoat">"crates.io"</a>", o "<a href="https://github.com/tokio-rs/topcoat/blob/v0.12.1/docs/session.md">"guia de sessões"</a>" e o "<a href="https://github.com/tokio-rs/topcoat/blob/v0.12.1/docs/mail.md">"guia de e-mail"</a>"."</li>
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
