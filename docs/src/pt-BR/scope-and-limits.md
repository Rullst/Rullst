# Escopo e limites

> **Tradução** da página em inglês [`scope-and-limits.md`](../scope-and-limits.md), feita a partir do commit [`746e8b724`](https://github.com/Rullst/Rullst/commit/746e8b7245f60672ffd384c0ba6e82b486ef5ca0). Se as duas versões divergirem, vale a versão em inglês.

Esta página lista o que o Rullst não faz por decisão de projeto e o que os
testes do próprio Rullst não conseguem comprovar, porque depende de serviços
externos, de hardware ou de versões de terceiros. Ela vale para toda versão
13.x até que uma versão a altere, e o changelog dessa versão diz isso. As
limitações próprias de uma versão (a atualização a partir do pré-lançamento
anterior, números medidos no código dessa versão, trabalho previsto para a
próxima) estão nas "Known limitations" dessa versão no
[changelog](../../../CHANGELOG.md).

## Projeto

- O Rullst tem um único mantenedor, com um acordo de sucessão descrito em
  [`GOVERNANCE.md`](../../../GOVERNANCE.md).
- A linha v12 chegou ao fim da vida em 16 de outubro de 2026; a 12.3.0 é sua
  última versão. O crates.io retirou (*yank*) as versões do `zip` de que o
  Studio da 12.x precisa, então um projeto 12.x novo não resolve mais suas
  dependências, e o caminho de atualização documentado da v5 para a 12.x pode
  também não resolver; use uma versão 13.x.

## Dependências e avisos de segurança

- `RUSTSEC-2026-0459` (SQLite 3.53 embutido no `libsqlite3-sys` 0.37.0, por
  meio do `sqlx-sqlite` 0.9.0 e de toda feature SQLite do Rullst; a correção é
  o `libsqlite3-sys` 0.39, que nenhuma versão do `sqlx-sqlite` aceita ainda) e
  `RUSTSEC-2026-0440` (`astral-tokio-tar`, sem manutenção, alcançado apenas
  pela dependência de desenvolvimento `testcontainers` 0.27.3 dos testes de
  integração; a 0.28 ainda depende dele) mantêm suas exceções com prazo até
  2027-01-28 em [Security Advisory Exceptions](../security-advisory-exceptions.md).
  Não abra arquivos SQLite que alguém menos confiável possa gravar. O workflow
  de CI que o `cargo rullst new` escreve ignora o `RUSTSEC-2026-0459` no passo
  `cargo audit` até a mesma data.

- Durante toda a 13.x, as 25 dependências públicas listadas na
  [política de compatibilidade](../compatibility-policy.md#public-dependencies)
  ficam nas suas linhas de versão. Uma atualização incompatível de uma delas
  só sai na 14.0, como exceção de segurança, ou atrás de uma nova feature
  opcional ao lado da linha da 13.x. As que ainda estão antes da 1.0, como
  `axum` 0.8, `sqlx` 0.9, `tower` 0.5, `tower-http` 0.7, `tower-sessions`
  0.15 (o `axum-session` do Connect), `validator` 0.21, `pgvector` 0.4 e
  `secrecy` 0.10, podem ficar atrás das versões mais novas durante a 13.x.

## Atualizações

- O `cargo rullst upgrade` muda as versões do Rullst, aplica as sugestões do
  compilador (`cargo fix`) e verifica se o projeto ainda compila; ele não
  reescreve nenhum outro código da aplicação. Suas regras localizam o que
  encontram pelo nome no código, nos manifestos, nos scripts e nos arquivos do
  projeto. Uma mudança que só aparece em tempo de execução (um valor montado
  em tempo de execução ou lido da configuração, um tipo de resposta definido
  por um middleware, uma entrada que chega nas requisições) não é localizada,
  e o comando não consegue provar que uma atualização de banco de dados, uma
  regra de autorização, uma integração com um provedor ou um deploy continuam
  funcionando
  ([o que o comando pode garantir](../tutorials/36-assisted-framework-upgrades.md#what-the-command-can-guarantee)).
  O compilador, a checklist de cada versão no guia de migração e os testes da
  própria aplicação continuam sendo a referência.

## Bancos de dados e o ORM

- O suporte a Turso/libSQL é limitado de propósito: ele é banco de dados
  principal apenas nos starters Blank e API (qualquer starter pode adicioná-lo
  como armazenamento auxiliar com `--turso`), por meio do `TursoOrm` e do
  contrato `TursoModel` que o `#[derive(Orm)]` com `#[orm(backend = "turso")]`,
  ou o `#[derive(TursoModel)]`, gera; esse model precisa declarar
  `#[orm(full_row_save)]`. Ele não funciona com `Schema`, `artisan!`,
  `Server::with_db`, Nexus, as páginas de banco de dados do Studio nem
  `make:billing`, e as transações dele são lotes de comandos
  (`TursoStore::transaction`). Paridade completa com o ORM não está planejada
  (veja os [não objetivos decididos](../../../ROADMAP.md#13x-minor-candidates-and-decided-non-goals)
  do ROADMAP). O cliente segue um `base_url` de mesma origem enviado por um
  servidor com várias instâncias (como o Turso Cloud); um `base_url` de outra
  origem é recusado com `PolyglotError::CrossOriginEndpoint`, e o insert não é
  confirmado. Quando o `TURSO_DATABASE_URL` está vazio ou é um valor `mock_`,
  o armazenamento é um arquivo SQLite local (`TURSO_OFFLINE_PATH` no starter
  gerado, por padrão `turso-development.db`) em qualquer ambiente, inclusive
  produção: um deploy que não define a variável grava nesse arquivo em vez de
  no Turso.
- O `save()` grava apenas as colunas alteradas de um model com um campo
  `RowSnapshot`. Um model com `#[orm(full_row_save)]`, e todo model Turso,
  grava todas as colunas, então duas requisições que alteram colunas
  diferentes da mesma linha ainda podem perder uma das alterações.
- Colunas nativas de data/hora do `Schema` precisam de uma feature de driver
  estrito (`strict-postgres`, `strict-mysql`, `strict-sqlite`). No pool `Any`
  do SQLx, `timestamps()` mantém colunas `TEXT`.
- Os tipos de data e hora do ORM (`rullst::orm::Timestamp`, `DateTime`, `Date`
  e `Time`) envolvem os do `jiff` 1. Um `Timestamp` guarda instantes de
  `-9999-01-02T01:59:59Z` a `9999-12-30T22:00:00.999999999Z`; um instante
  gravado fora desse intervalo, como o marcador de "nunca expira"
  `9999-12-31 23:59:59` numa coluna `timestamptz` ou `DATETIME` do MySQL, dá
  um erro de leitura que mostra o valor. Para "sem fim", grave NULL num campo
  `Option<Timestamp>`, ou use uma coluna `DateTime` sem fuso, que mantém o
  intervalo completo. Segundos intercalares não são representados: um `:60`
  gravado é lido como `:59`.
- No modo WAL (`sqlite_journal_mode = "wal"`), o arquivo do banco sozinho não
  é mais um backup (use `sqlite3 ".backup"`), todos os processos precisam
  rodar no mesmo host, e o WAL não deve ser usado num sistema de arquivos de
  rede (NFS, SMB, Azure Files: use `"delete"` nesses casos); o
  `cargo rullst doctor` avisa sobre um banco em WAL num ponto de montagem
  desses.
- As migrações na inicialização são opcionais (`RULLST_AUTO_MIGRATE=true`).
- As trilhas de auditoria duráveis do `rullst-ai` (`DurableToolAuditTrail`,
  `DurableRagAuditTrail`) são gravadores locais de um único processo, não
  serviços de auditoria autenticados ou distribuídos: seus quadros SHA-256
  detectam corrupção, mas não autenticam os eventos. Uma aplicação com várias
  instâncias implementa `ToolAuditSink` sobre um destino durável próprio
  ([ferramentas locais de IA protegidas](../ai-tool-security.md#durable-local-evidence)).

## Requisições e páginas

- Os deploys gerados comprimem as páginas porque o token CSRF é mascarado a
  cada resposta. O mascaramento cobre apenas esse token: uma aplicação não deve
  colocar outros segredos numa página comprimida ao lado de texto vindo da
  requisição.
- Multipart: o campo CSRF `_token` precisa vir antes dos campos de arquivo,
  dentro dos primeiros 64 KiB (ou envie o cabeçalho `X-CSRF-Token`). Os
  arquivos enviados não passam por antivírus nem têm os metadados removidos, e
  o WAF inspeciona apenas os campos de texto, não o conteúdo nem os nomes dos
  arquivos ([envio de arquivos em formulários](../form-uploads.md)).
- O `/docs` (Scalar) carrega uma versão fixa do Scalar, a 1.67.0, com seu hash
  de integridade, e sua política permite pelos hashes os elementos de estilo
  que essa versão insere; por isso, uma versão posterior do Scalar precisa de
  uma versão do Rullst com hashes novos. A página usa as fontes do sistema e
  não tem o agente "Ask AI": os dois estão desativados.

## Pacotes de escopo limitado

Os pacotes promovidos ao nível Extension mantêm o escopo documentado
([níveis de maturidade](../maturity.md)).

- O Labs nunca executa o código dos alunos: a aplicação é dona do executor e
  do seu isolamento ([contrato do executor](../labs-runner-contract.md)). Um
  contêiner Docker por tarefa não é isolamento suficiente para código hostil;
  o contrato lista o mínimo (um host ou VM separado, sem rede, com limites de
  recursos e restrições de chamadas de sistema e de sistema de arquivos).
- O Supervision não traz nenhum modelo de mídia: os adaptadores de presença na
  câmera e de atividade de áudio são da aplicação, e o navegador envia
  estatísticas, não quadros de vídeo nem áudio. Uma `CoursePolicy` parental é
  uma única janela UTC absoluta de no máximo 30 dias; horários permitidos
  recorrentes ficam no código da aplicação.
- O Privacy não tem provedor de verificação de idade real nem tutela
  verificada.
- O IoT foi validado num micro:bit V2, e seu build `std` num gateway
  alimentado por placas Arduino Uno. Transportes MQTT e CoAP, TLS/DTLS e
  brokers, outras placas, um armazenamento persistente do contador de
  rollback, gravação do firmware, a passagem para o bootloader e a perda de
  energia não foram validados.
- O adaptador Bunny Stream do Media é usado com uma biblioteca real por uma
  aplicação em produção, mas transcodificação e casos extremos da CDN não são
  cobertos pelos testes do próprio Rullst, que rodam offline. O status 7
  (*just-in-time*) do Bunny continua `Processing` (só o status 8 é
  `Playable`), e um vídeo `Playable` só toca no player do próprio Bunny. O
  adaptador S3/R2 é testado apenas offline; nenhum teste automatizado roda
  contra um serviço S3, R2 ou MinIO real.
- O adaptador InfinitePay cobre links de pagamento avulsos e notificações
  confirmadas na origem, validados com um pagamento Pix real na execução de
  aceitação da 13.0.0-alpha.5. Assinaturas, o portal, cobrança por uso,
  cupons e períodos de teste retornam `UnsupportedOperation`, e o lojista
  precisa antes ativar o checkout externo ("Checkout externo") no app da
  InfinitePay.

## Contas e provedores externos

- Nenhum teste automatizado roda contra a conta real de um provedor: a CI
  exercita os provedores de pagamento, vídeo, OAuth, e-mail e IA contra mocks,
  fixtures e servidores de teste locais, sem credenciais.
- O login com Google e GitHub é usado pelas aplicações em produção do
  mantenedor, sem teste automatizado com uma conta real; Microsoft, Apple e os
  outros provedores OAuth não foram usados com contas reais. O login da Microsoft precisa das claims opcionais `email`
  e `xms_edov` para vincular ou criar contas. O `make:oauth` precisa dos
  arquivos de conta do `cargo rullst auth` ou do starter SaaS, ou das funções
  do contrato de autenticação documentado (`src/auth_contract.rs`).
- PWA: os ícones gerados são provisórios. Os fluxos do Google Play
  (Bubblewrap) e da Microsoft Store (PWABuilder) estão documentados, mas não
  foram validados com envios reais às lojas; nenhum pacote para a App Store é
  gerado.
- `cargo rullst deploy`: a CI compila o Dockerfile gerado e sobe o
  `compose.yaml` e o `Caddyfile` gerados para os starters Blank e SaaS com
  Docker e Podman sem root num runner Ubuntu do GitHub, verificando HTTPS pelo
  Caddy com sua autoridade certificadora local, páginas comprimidas com o
  token CSRF mascarado, Server-Sent Events através da compressão, o
  endurecimento do contêiner, as migrações, o volume SQLite em modo WAL após
  um reinício e os comandos de backup e restauração. Domínios reais, DNS e
  Let's Encrypt, contas e firewalls de provedores, máquinas Arm,
  reinicializações e deploys com PostgreSQL/MySQL não são testados, e no
  Ubuntu 26.04 o `podman compose restart` pode deixar o Caddy parado (use
  `podman compose down` e `up -d`). Os guias de provedores e a página
  Advanced deployment foram escritos a partir da documentação dos provedores e
  das ferramentas, sem terem sido seguidos do início ao fim em contas reais.

## CLI e Studio

- O `cargo rullst auth` num starter Blank ou API precisa de exatamente um
  perfil de banco de dados estrito (um projeto com `drivers-all` é recusado) e
  adiciona migrações que estendem a tabela `users` do starter e criam a tabela
  `rullst_auth_sessions`. Os testes o executam em SQLite com linhas
  existentes; com PostgreSQL 15, MySQL 8.4 e MariaDB 11.4, a CI o executa
  contra um servidor em starters Blank e API recém-gerados com duas linhas e
  verifica com SQL as colunas adicionadas, com seus tipos e valores padrão, o
  índice único `users_email_unique`, os e-mails provisórios e as novas
  tabelas, depois um cadastro por HTTP que registra uma sessão, e o
  `db:rollback`; outras versões dos servidores e os dados de um deploy real
  não são testados. Faça backup do banco e teste numa cópia antes.
- O número de carbono do `cargo rullst footprint` é uma estimativa SCI a
  partir dos valores de `--cpu-watts`, `--grid-intensity` e
  `--network-kwh-per-gb` que você informa, não uma medição; o tamanho de uma
  imagem só é medido por um socket local do Docker ou do Podman; e o peso da
  página baixa apenas recursos da mesma origem, listando as origens de
  terceiros sem baixá-las, sem cookies nem login. Um site `https://` publicado
  recebe apenas o peso da página, e uma aplicação encontrada com `--url` não
  tem tempo de inicialização.
- Monitor de produção: os números são por processo e zeram ao reiniciar; os
  starters não o ativam. As notificações na área de trabalho do Studio
  autônomo só aparecem enquanto uma página do Studio está aberta num navegador
  (uma aba em segundo plano basta, e o navegador pode então verificá-la cerca
  de uma vez por minuto), e uma página aberta depois não mostra as anteriores.
  Um proxy que exige senha só pode vir do ambiente (`HTTPS_PROXY`,
  `HTTP_PROXY` ou `ALL_PROXY`), e proxies SOCKS não são suportados: um valor
  SOCKS é recusado e bloqueia todas as conexões. Os tokens são protegidos
  contra um arquivo de configuração vazado, não contra um malware rodando com
  o usuário do desenvolvedor
  ([monitoramento de produção](../production-monitoring.md#through-a-proxy)).

## Idiomas e documentação

- A formatação de i18n tem dados CLDR para os 27 locales documentados; um
  idioma sem dados usa os dados do inglês (`en`). As regras de plural cobrem
  18 idiomas, e uma mensagem no plural em outro idioma faz o build falhar.
  Moedas além das 15 com constante (BRL, USD, EUR, GBP, JPY, ARS, MXN, CLP,
  COP, PEN, UYU, CAD, AUD, CHF e CNY) são escritas com o código ISO. Um novo
  arquivo de locale precisa da linha documentada no `build.rs`
  ([traduções e locales](../i18n.md)).
- A CI executa apenas os comandos das páginas da primeira hora (o início
  rápido do README, os primeiros passos, os tutoriais 01, 02, 03, 05 e 11 e o
  guia rápido de API REST), sem servidores PostgreSQL ou MySQL nem contas
  externas. Os comandos das outras páginas não são executados, e seus blocos
  Rust marcados `rust,ignore` não são compilados. Só essas páginas e esta
  estão traduzidas para o português do Brasil; a CI verifica se os blocos de
  código são idênticos, byte a byte, aos da versão em inglês, mas só mostra
  um aviso quando o texto em inglês mudou depois do commit indicado na
  tradução, então uma tradução pode ficar desatualizada (vale a página em
  inglês).

## Evidências de release

- Evidências de fuzzing da release: uma alteração numa página do livro que os
  doctests da fachada incluem executa de novo os 12 alvos de fuzzing da
  fachada, porque a regra do grafo de build conta os arquivos incluídos como
  entradas ([reaproveitamento de evidências de fuzzing](../fuzz-evidence.md)).
