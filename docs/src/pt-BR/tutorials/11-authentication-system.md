# Tutorial 11: estrutura de autenticação 🔒

> **Tradução** da página em inglês [`11-authentication-system.md`](../../tutorials/11-authentication-system.md), feita a partir do commit [`654944c44`](https://github.com/Rullst/Rullst/commit/654944c445e464c2b55d4720e6dd70105a07e44d). Se as duas versões divergirem, vale a versão em inglês.

O gerador de autenticação cria um ponto de partida revisável para login e
cadastro com sessão em cookie. É código-fonte da aplicação, não um serviço de
identidade hospedado nem a promessa de que toda política de produção é
automática.

---

## Passo 1: execute o gerador

O starter SaaS já contém este código: nele, o `cargo rullst auth` informa que
a autenticação já está estruturada, não grava nada e termina com sucesso. Em
um projeto Blank ou API com banco de dados (SQLite, PostgreSQL, MySQL ou
MariaDB), como o projeto `shop` do [Tutorial 02](02-cli-generators.md),
execute a partir da raiz do projeto:

```bash
cargo rullst auth
```

Esses starters vêm com uma tabela `users` mínima (`id`, `name`, `created_at`
e `updated_at`, de `src/migrations/m20260601000000_create_users_table.rs`). O
comando mantém essa migração como está e adiciona outra que estende a tabela,
então funciona tanto se o `cargo rullst db:migrate` (ou o primeiro
`cargo rullst dev`) já criou a tabela quanto se não criou, e as linhas que já
estão nela mantêm seus dados. Cada linha existente recebe o e-mail provisório
`user-<id>@no-email.invalid` (o domínio reservado `.invalid` nunca recebe
e-mails) e nenhuma senha, então não consegue entrar até você definir o e-mail
real e a pessoa redefinir a senha.

O comando atual cria:

- `src/models/user.rs` e `src/models/password_reset.rs`;
- `src/migrations/m<timestamp>_create_users_table.rs` ou, em um starter Blank
  ou API, `src/migrations/m<timestamp>_add_account_columns_to_users_table.rs`
  (as colunas `email`, `password_hash`, `oauth_provider`, `oauth_id`,
  `is_admin` e `session_epoch` e o índice único em `email`; desfazê-la remove
  apenas esses itens), `src/migrations/m<timestamp>_create_password_reset_tokens_table.rs`,
  `src/migrations/m<timestamp>_create_rullst_auth_sessions_table.rs` (as
  sessões de login; desfazê-la remove a tabela, o que desconecta todo mundo) e
  o módulo de migrações;
- `src/controllers/auth_controller.rs`,
  `src/controllers/password_controller.rs` e
  `src/controllers/sessions_controller.rs` (a página "Your devices", com os
  seus dispositivos);
- `src/middlewares/auth_middleware.rs`;
- `src/pages/auth.rs`;
- as declarações de módulo correspondentes, incluindo `controllers`,
  `middlewares`, `models` e `pages` no `src/lib.rs` (ou no `src/main.rs`);
- em um starter Blank ou API, `pub use crate::models::user::User;` no lugar da
  struct `User` de dois campos do starter no `src/main.rs`, para que o projeto
  tenha um único modelo de `users` (uma struct `User` que você alterou fica
  como está); e
- uma atualização, feita na medida do possível, dos arquivos gerados
  `.llms.txt`, `.rullst/context-map.json` e `diagram.md` (um `diagram.md`
  escrito à mão fica inalterado).

Ele também ativa as features guarda-chuva `orm`, `auth` e `mail` no
`Cargo.toml` e o registro de sessões do banco de dados do projeto
(`auth-session-registry-sqlite`, `-postgres` ou `-mysql`, a partir do perfil
`strict-*` dele; um projeto sem exatamente um perfil estrito é recusado).
Quando todos esses arquivos já existem, como no starter SaaS ou depois de uma
execução anterior, ele não grava nada e tem sucesso. Ele falha antes de gravar
qualquer coisa quando só alguns deles existem, quando existe uma migração
`*_create_password_reset_tokens_table.rs` ou
`*_add_account_columns_to_users_table.rs`, ou quando uma migração
`*_create_users.rs`/`*_create_users_table.rs` diferente da migração original do
starter cria a tabela de usuários: nesse caso, adicione as colunas com
`cargo rullst make:migration` e integre os arquivos de conta à mão, como diz a
mensagem de erro. Projetos com o Turso como banco principal não são
suportados, e um projeto sem executor de migrações de banco de dados (um
starter `--no-database`) também não: crie o projeto com um banco de dados,
para que o `src/main.rs` declare `pub mod migrations;` e chame
`rullst::artisan!(crate::migrations::get_migrations())`. Ele não aceita a flag
`auth --api` e não registra rotas da aplicação silenciosamente. Revise o diff
gerado antes de editá-lo.

---

## Passo 2: registre os handlers gerados

Conecte no roteador da aplicação as rotas geradas de exibição, envio, logout e
as autenticadas. A árvore exata de rotas é uma decisão da aplicação; um
mapeamento típico está abaixo. Coloque as linhas `use` dele no topo do
`src/main.rs`, que já importa `routes`, e as instruções `let` em `router()`:

```rust,ignore
use rullst::{routing::post, server::from_fn};
use crate::controllers::{auth_controller, sessions_controller};
use crate::middlewares::auth_middleware::auth_middleware;

let public_auth = routes![
    get("/login" => auth_controller::login_view),
    get("/register" => auth_controller::register_view),
    post("/logout" => auth_controller::logout),
]
// Each credential submission runs Argon2id: budget it per client.
.route("/login", post(auth_controller::login_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)))
.route("/register", post(auth_controller::register_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)));

let signed_in = routes![
    get("/dashboard" => auth_controller::dashboard),
    // Ends the session on every device.
    post("/logout/everywhere" => auth_controller::logout_everywhere),
    // Your devices: each signed-in session, which can be signed out.
    get("/sessions" => sessions_controller::sessions_view),
    post("/sessions/revoke" => sessions_controller::revoke_session),
    post("/sessions/revoke-others" => sessions_controller::revoke_other_sessions),
]
.layer(from_fn(auth_middleware));
```

Monte os fluxos de senha da mesma forma: as rotas de redefinição são públicas,
as de troca precisam do `auth_middleware`, e todo envio tem limite de taxa.

```rust,ignore
use crate::controllers::password_controller;

let password_reset = routes![
    get("/password/forgot" => password_controller::forgot_password_view),
    get("/password/reset" => password_controller::reset_password_view),
]
// Reset requests send mail and resets run Argon2id.
.route("/password/forgot", post(password_controller::forgot_password_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)))
.route("/password/reset", post(password_controller::reset_password_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)));

let password_change = routes![
    get("/password/change" => password_controller::change_password_view),
]
.route("/password/change", post(password_controller::change_password_submit)
    .layer(from_fn(auth_controller::credential_rate_limit)))
.layer(from_fn(auth_middleware));
```

Adicione cada grupo ao roteador do starter em `router()` (`src/main.rs`) com
`.merge_axum(public_auth.into_axum())` e assim por diante, antes dos
comentários acima da linha `.merge_axum(rullst::health::health_router())`. O
`Server` aplica a proteção contra CSRF a todas as rotas, também em
desenvolvimento; em um roteador criado antes da 13.0.0-alpha.6, que tem sua
própria camada `csrf_middleware`, os grupos vêm antes dessa camada. O
`router()` do starter SaaS mostra as mesmas rotas registradas uma a uma.

O `credential_rate_limit` permite dez envios por minuto para cada par que se
conecta. Atrás de um proxy reverso, configure `[security] trusted_proxies`
para que os clientes não dividam o limite do proxy.

Os e-mails de redefinição passam pela fachada `Mail`: defina `MAIL_DRIVER` e
`MAIL_FROM` para staging e produção, e `RULLST_PUBLIC_ORIGIN` com a origem
HTTPS da aplicação (por exemplo `https://app.example.com`), que todo link de
redefinição usa. Os links nunca são montados a partir do cabeçalho `Host` da
requisição. Sem `RULLST_PUBLIC_ORIGIN`, os links de desenvolvimento apontam
para `http://localhost:3000`, e staging e produção registram um erro em vez
de enviar um link. Em desenvolvimento, o driver de e-mail padrão registra
apenas metadados, então uma compilação de debug sem `MAIL_DRIVER` também
mostra o link de redefinição no terminal.

Os e-mails dão ao seu produto o nome de `APP_NAME` (do ambiente do processo e,
depois, do `.env`) ou, quando ela não está definida, da constante
`APPLICATION_NAME` no topo do `password_controller.rs`, que começa com o nome
do pacote. Eles são escritos no idioma da requisição: o locale negociado por
`LOCALES.layer()` quando a aplicação usa `rullst::i18n`; senão, o
`Accept-Language` do navegador, comparado por idioma com inglês, português
brasileiro ou espanhol; senão, o `DEFAULT_MAIL_LOCALE` (inglês), que você
também pode editar ali.

As páginas de formulário geradas esperam as extensões de CSRF e de nonce de CSP
do Core. Aplique a linha de base de segurança canônica e coloque as rotas
autenticadas atrás do middleware de autenticação gerado.

---

## Passo 3: migre e teste

```bash
cargo rullst db:migrate
cargo rullst dev
```

Antes da produção, teste o cadastro, o login, o logout (uma cópia do cookie
precisa parar de funcionar), "Your devices", sair em todos os dispositivos, a
troca e a redefinição de senha, credenciais inválidas, e-mail duplicado,
cookies e links de redefinição expirados, adulterados ou revogados, falha de
CSRF, limite de taxa, a entrega de e-mails pelo seu provedor e a rotação de
chaves. Configure um `APP_KEY` forte por meio de um gerenciador de segredos, e
HTTPS na borda.

---

## O que a estrutura gerada garante hoje

- As senhas recebem hash com Argon2id no pool de bloqueio do Tokio; o texto
  puro não é gravado no modelo de usuário. No máximo quatro hashes ou
  verificações rodam ao mesmo tempo (cada um ocupa cerca de 19 MiB); um envio
  que espera dois segundos sem capacidade recebe 503. Ajuste
  `MAX_CONCURRENT_PASSWORD_WORK` ao servidor.
- O cadastro aceita senhas com pelo menos 12 caracteres e no máximo 72 bytes
  e normaliza o e-mail.
- O login faz uma verificação de senha fictícia para usuários desconhecidos,
  para reduzir a diferença óbvia de tempo que permitiria enumerar contas.
- Todo login é uma sessão no servidor: uma linha de `rullst_auth_sessions`
  (criada pela migração dela no banco de dados da aplicação) que guarda apenas
  um digest do segredo aleatório que fica no cookie criptografado
  `rullst_session`, o id da conta, a hora do login, a hora do último uso e a
  família do navegador e da plataforma ("Firefox on Linux"), nunca o endereço
  IP nem o `User-Agent` bruto. O `auth_middleware` confere essa linha em toda
  requisição. **Log out** a apaga, então uma cópia do cookie (de um computador
  compartilhado ou de um dispositivo roubado) também para de funcionar.
  **Your devices** (`/sessions`) lista as sessões da conta e desconecta uma
  delas ou todas as outras. Uma requisição é conferida quando começa, então
  uma que já está em andamento quando a sessão dela é revogada termina. Um
  stream de Server-Sent Events ou um WebSocket que você adicionar pode
  terminar junto com a sessão: passe `auth_middleware::revalidation(&session)`
  para `RealtimeSse::channel_revalidated` ou `WebSocket::with_revalidation`.
  Veja [gestão de sessões](../../session-management.md#server-side-login-sessions).
- A coluna `users.session_epoch` (0 em uma conta nova) vai em todo cookie de
  login, e o `auth_middleware` recusa um cookie cuja época seja diferente.
  `User::revoke_sessions` a incrementa em um único `UPDATE` e apaga as sessões
  da conta, encerrando todos os cookies dela; o botão *Sign out everywhere*
  (`logout_everywhere`, sair em todos os dispositivos) a chama. Chame
  `User::revoke_sessions` ao desativar uma conta. Veja
  [sessões criptografadas](../../crates/auth.md#revocable-cookies-the-session-epoch).
- **Change password** (`/password/change`, com login) pede a senha atual, a
  nova e uma confirmação. Uma senha atual errada recebe a mensagem genérica
  "The current password is incorrect". A nova senha segue a política do
  cadastro. `User::change_password` grava o novo hash e incrementa a época da
  sessão em um único `UPDATE`, que só se aplica enquanto a conta ainda está na
  época do cookie que fez o pedido; as sessões da conta são apagadas, esse
  navegador recebe uma sessão nova na nova época, e todos os outros
  dispositivos são desconectados. Um link de redefinição pedido antes da troca
  para de funcionar.
- **Forgot password** (`/password/forgot`) responde com a mesma página, depois
  de pelo menos 400 ms, tenha o e-mail uma conta ou não. Para uma conta, ele
  cria um `PasswordResetToken` (32 bytes aleatórios do sistema operacional),
  guarda apenas o digest SHA-256 dele em `password_reset_tokens`, com validade
  de 30 minutos, e envia o link por e-mail em segundo plano. A tabela guarda um
  token por conta: um pedido novo substitui o token anterior, mas pedidos para
  a mesma conta com menos de 60 segundos de intervalo não enviam nada.
- **Validade do link de redefinição**: `RESET_TOKEN_LIFETIME_MINUTES`, em
  `src/models/password_reset.rs` (30 no código gerado), a define. Mude-a ali:
  uma verificação em tempo de compilação a mantém entre 10 minutos e 24 horas
  (1440), e a validade gravada, o e-mail ("This link expires in 30 minutes.",
  ou "2 hours" para horas inteiras) e a resposta de esqueci-a-senha
  acompanham esse valor. Abaixo de 10 minutos, um e-mail que o provedor coloca
  em greylist ou na fila por alguns minutos chega depois que o link expirou, e
  a pessoa só pode pedir de novo. Acima de 24 horas, o link continua sendo uma
  porta de entrada funcional para a conta enquanto estiver em uma caixa de
  correio sincronizada com vários dispositivos, encaminhada ou lida por outra
  pessoa, e um dia já atende quem lê e-mails uma vez por dia. O uso único e a
  substituição por um pedido mais novo reduzem essa exposição, mas não a
  eliminam; por isso o
  [OWASP Forgot Password Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html)
  pede tokens de vida curta.
- **Reset password** (`/password/reset?token=…`) mostra o formulário apenas
  para um token válido, sem cache e com `Referrer-Policy: no-referrer`.
  Enviá-lo apaga o token, grava o novo hash e incrementa a época da sessão em
  uma única transação e, depois, apaga as sessões da conta, então o link
  funciona uma vez e todos os dispositivos são desconectados. A pessoa não
  entra na conta: a página redireciona para `/login?reset=done`. Os dois
  fluxos enviam por e-mail um aviso de "senha alterada".

Esses controles não substituem a revisão da aplicação. Adicione verificação de
conta, controles contra abuso, política de auditoria, MFA/passkeys, limpeza
agendada das sessões expiradas (`purge_expired`) e o comportamento de
privacidade e retenção de acordo com o modelo de ameaças do produto. Um link de
redefinição prova apenas o acesso à caixa de correio; para contas de alto
valor, adicione um segundo fator à recuperação.
