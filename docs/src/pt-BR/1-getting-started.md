# Primeiros passos

> **Tradução** da página em inglês [`1-getting-started.md`](../1-getting-started.md), feita a partir do commit [`654944c44`](https://github.com/Rullst/Rullst/commit/654944c445e464c2b55d4720e6dd70105a07e44d). Se as duas versões divergirem, vale a versão em inglês.

Boas-vindas ao guia de primeiros passos do **Rullst**!

**Seu objetivo:** instalar a CLI da v13 correspondente, gerar uma aplicação
pequena, abri-la localmente e fazer sua primeira alteração. Prefere escrever
a primeira rota você mesmo? Use o [Zero ao Olá, Rullst](tutorials/01-hello-world.md).

O Rullst é uma suíte de frameworks Rust com tipagem estrita para aplicações
full-stack, projetada em torno de APIs explícitas, desempenho mensurável e
padrões de defesa em profundidade.

## 1. Instalação

Primeiro, garanta que o Rust esteja instalado. A forma oficial e recomendada é acessar [rustup.rs](https://rustup.rs/).

**No macOS e no Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**No Windows:**
Baixe e execute o `rustup-init.exe` do site.

Em seguida, instale a **CLI do Rullst** da mesma linha de versões do
framework. Este livro é gerado a partir da `main`, onde a v13 está em
desenvolvimento. A linha v12 chegou ao fim da vida útil em 16 de outubro de
2026 (a 12.3.0 foi sua última versão, e um projeto 12.x novo já não resolve
suas dependências depois que o crates.io retirou (yank) as versões do `zip`
de que o Studio dela precisa), então instale o pré-lançamento da v13. Informe
a versão exata: o Cargo nunca escolhe um pré-lançamento sozinho, então nem um
`cargo install cargo-rullst` simples nem `^13` o instalam:

```bash
cargo install cargo-rullst --version =13.0.0-rc.2 --locked
```

Fixe o framework da mesma forma no `Cargo.toml` (`"=13.0.0-rc.2"`); o
[guia de migração para a v13](../migration-v13.md) explica por quê e o que mudou.

Quem revisa o código-fonte e precisa de alterações locais do checkout pode,
em vez disso, clonar este repositório e instalar a CLI a partir desse checkout
exato. Na `main`, isso instala a CLI de desenvolvimento da v13:

```bash
git clone --branch main https://github.com/Rullst/Rullst.git
cd Rullst
cargo install --locked --path cargo-rullst
cd ..
```

Instalada a partir do código-fonte, a CLI reutiliza o checkout exato de onde
foi compilada, mesmo quando a criação do projeto é feita a partir de outro
diretório, desde que esse checkout não tenha sido movido nem apagado. Por
isso, os manifestos gerados contêm dependências por caminho absoluto e não são
portáveis. Executar a partir da raiz do repositório continua sendo uma
alternativa explícita. Para aplicações comuns, prefira o pré-lançamento
publicado e os pacotes correspondentes do registro.

O Cargo instala os executáveis `cargo-rullst` e `rullst` em
`$CARGO_HOME/bin` (`~/.cargo/bin` por padrão). Para instalar em outro lugar,
defina `CARGO_INSTALL_ROOT` ou passe `--root <dir>`. Para remover a CLI,
execute:

```bash
cargo uninstall cargo-rullst
```

Acrescente o mesmo `--root <dir>` se você instalou com um.

## 2. Criando seu primeiro projeto

Execute o assistente de criação com o nome da sua aplicação (ou execute
`cargo rullst` sozinho e escolha **Create New Project** na tela inicial):

```bash
cargo rullst new my_app
```

O assistente faz as perguntas no terminal e não grava nada até você
confirmar. Esc volta uma pergunta.

1. **Starter**: **Blank**, **API** ou **SaaS**, cada um com uma descrição de
   uma linha e uma prévia dos arquivos que gera (veja
   [Starters do Rullst](#3-starters-do-rullst)).
2. **Banco de dados**: o banco de dados principal. O **SQLite** é a primeira
   execução local mais simples; PostgreSQL, MySQL e MariaDB precisam do
   serviço de banco de dados rodando.
3. **Funcionalidades**: funcionalidades opcionais, como IA, Redis ou um
   complemento de armazenamento. Não escolher nenhuma é válido; um complemento
   de armazenamento não é outro banco de dados principal.
4. **Revisão**: os arquivos e os comandos que ele vai executar. Escolha
   **Create the project**, **Back** para mudar uma resposta ou **Cancel** para
   sair sem criar nada.

Para uma aplicação com banco de dados, a CLI compila a nova aplicação uma vez,
executa a migração inicial e mostra três próximos passos numerados:
`cd my_app`, `cargo rullst dev` e a URL local da página de boas-vindas
gerada, por padrão `http://127.0.0.1:3000`:

```bash
cd my_app
cargo rullst dev
```

Abra a URL local que o comando informa. Encontre o código da página gerada,
mude um título e salve. Confirme que a página alterada aparece depois que a
recompilação e a reinicialização terminam com sucesso. Pare o desenvolvimento
com `Ctrl+C` antes de mover ou remover o diretório do projeto.

`cargo rullst new my_app --default --dry-run` mostra o plano, a árvore de
arquivos e os comandos sem criar nada, e `cargo rullst tour` apresenta os
principais comandos sem alterar nenhum projeto. Sem um terminal (no CI ou em
um script), o `new` precisa de `--default`; flags como `--blueprint` e
`--database` respondem às perguntas correspondentes. Veja a
[referência da CLI](../cli_reference.md#cargo-rullst-new-name).

O workflow de CI do projeto (`.github/workflows/ci.yml`) falha com qualquer
aviso do Cargo ou do compilador. Com o Cargo 1.100 ou posterior, isso inclui
uma dependência declarada no `Cargo.toml` que você não usa: remova-a ou, se
ela for intencional, veja a configuração `[lints.cargo]` em
[Testando sua aplicação](../testing-your-app.md#githubworkflowsciyml-and-githubdependabotyml).

Prefere um starter pequeno e determinístico sem passar pelo assistente?

```bash
cargo rullst new first_app --default --blueprint blank --database sqlite \
  --skip-initial-migration
cd first_app
cargo rullst dev
```

A primeira compilação compila todas as dependências e pode levar vários
minutos (cerca de dois minutos e meio para o starter Blank na máquina medida
em [Tempos de compilação](../build-times.md), que também traz dicas).
`--skip-initial-migration` adia o trabalho inicial do gerador; o comando de
desenvolvimento executa a migração inicial antes de iniciar uma aplicação
gerada com banco de dados. Nunca aponte este projeto de aprendizado para um
banco de dados de produção.

> [!TIP]
> O comando `cargo rullst dev` compila o projeto e inicia o servidor local.
> Salvar alterações dispara uma recompilação de verdade e uma reinicialização
> supervisionada do processo; uma compilação com falha mantém a aplicação
> anterior no ar. O cliente do navegador, na mesma origem, atualiza a página
> depois que o novo processo responde com seu marcador de geração. O estado
> em memória é reiniciado. Nenhuma opção de scaffold de hot reload é
> necessária. Veja a [referência da CLI](../cli_reference.md#cargo-rullst-dev).
> 
> A CLI gera, de propósito, um único perfil de aplicação auditado: Active
> Record com views `html!` renderizadas no servidor e melhoria progressiva com
> HTMX. O framework ainda expõe fundações de repositório de nível mais baixo
> para integrações que a aplicação controla. Veja o
> [guia de escolhas de arquitetura](../architecture-decisions.md).


## 3. Starters do Rullst

`cargo rullst new` gera um de três starters. O assistente pergunta qual (ou
passe `--blueprint blank`, `api` ou `saas`); todos usam o esquema de cores do
Rullst e padrões de HTML/HTMX renderizados no servidor, e a alocação e a
latência dependem da página gerada e do ambiente de execução.

### Blank
**Caso de uso:** desenvolvimento personalizado, do zero.
É o template mínimo, com views `html!` renderizadas no servidor e HTMX, sem um
bundle JavaScript local do projeto. Ele inclui um contador reativo simples
para demonstrar a comunicação conduzida pelo servidor. Outras stacks de
frontend são código da aplicação, não starters gerados.

### API
**Caso de uso:** serviços JSON sem interface (headless).
O starter Blank sem HTML (`--blueprint api` ou `--api`): rotas JSON, uma
rota de escrita de exemplo protegida por um bearer `API_TOKEN` gravado no
`.env` e a mesma linha de base de segurança de produção. Ele pode rodar sem
banco de dados (`--no-database`) ou com o Turso como banco principal.

### SaaS
**Caso de uso:** produtos por assinatura e cobrança.
Um ponto de partida de SaaS opinativo, já conectado com:
- Autenticação de usuários (login, cadastro, gestão de sessões, sair em todos
  os dispositivos, troca de senha e redefinição de senha por e-mail, com links
  montados a partir de `RULLST_PUBLIC_ORIGIN`).
- Painéis de preços do Stripe e telas de checkout de assinatura.
- Painel seguro do usuário e um admin do Nexus para administradores
  autenticados.

### Aplicações de exemplo maiores

Até a 13.0.0-alpha.3, a CLI também gerava os starters LMS, Blog, Portfolio e
ERP. O diretório [`examples/`](../examples.md) do repositório guarda as
aplicações completas que os substituem — `examples/lms` (cursos, aulas, um
player acessível, matrícula e progresso vinculado ao dono) e a vitrine
`examples/blog`, cujo CRUD com o Nexus também substitui os exemplos Portfolio
e ERP removidos na 13.0.0-alpha.5 —, testadas no workspace. Copie uma delas
para começar uma aplicação maior; `cargo rullst new --blueprint lms` (ou
`blog`, `portfolio`, `erp`) não cria nada e, em vez disso, indica o exemplo.

---

> [!TIP]
> **Evolução dos starters:** os starters são verificados continuamente por
> smoke tests do gerador. Trate o código gerado como um ponto de partida da
> aplicação: inspecione a configuração e rode de novo o `cargo check` e os
> testes de segurança depois de personalizar.

### Acesso local ao Studio e ao Nexus

O starter SaaS (e os exemplos LMS e Blog) mostram botões visíveis para as duas
superfícies de controle durante o desenvolvimento local:

- O **Nexus** fica montado em `/nexus` e, em uma compilação de debug, aceita
  apenas um cliente de loopback verificado, então o primeiro clique local não
  precisa de senha provisória.
- O **Studio** inicia como um serviço separado, só de debug, em
  `http://127.0.0.1:5555`.

Essa conveniência não pode ser ativada em um binário de release por meio de
`RULLST_ENV` nem do alias legado `APP_ENV`. Uma compilação de release não
inicia a tarefa do Studio gerada, e o Nexus exige um login de verdade:

- O **SaaS** (e o exemplo LMS) admitem o usuário autenticado na aplicação cuja
  coluna `users.is_admin` é verdadeira, verificada em cada requisição; sem
  credenciais extras. Conceda o primeiro administrador com o seu cliente de
  banco de dados
  (`UPDATE users SET is_admin = TRUE WHERE email = 'you@example.com';`); esse
  administrador pode conceder acesso a outros pelo modelo Users no Nexus. Veja
  [usando o login da aplicação](../4-rullst-nexus.md#using-the-applications-sign-in).
- O **exemplo de blog** exige valores únicos de `NEXUS_ADMIN_USERNAME` e
  `NEXUS_ADMIN_PASSWORD` (Basic Auth atrás de TLS verificado) em uma
  compilação de release e não inicia sem eles; em vez disso, uma aplicação
  pode iniciar sem `/nexus` e mostrar uma linha com o nome da variável que
  falta, veja
  [servindo a aplicação sem o Nexus](../4-rullst-nexus.md#serving-the-application-without-nexus).

Coloque o Nexus de produção atrás de uma fronteira de TLS verificada; não
exponha o Studio publicamente. O Studio não tem modo de produção: ele roda
apenas em compilações de debug e no loopback.

## 4. Configuração do banco de dados e stores especializados

O assistente toma, de propósito, duas decisões separadas:

1. Escolher o backend principal do **Active Record com SQLx**: **SQLite**,
   **PostgreSQL**, **MySQL** ou **MariaDB**. O MariaDB usa o protocolo de rede
   do MySQL, mas tem seu próprio contrato de contêiner executável. O starter
   Blank (página web ou API JSON) pode, em vez disso, escolher o
   **Turso/libSQL** como banco principal, por meio de um adaptador separado
   que não tem Nexus, páginas de banco de dados no Studio, migrações `Schema`,
   `artisan!` nem o query builder do SQLx.
2. Opcionalmente, adicionar stores auxiliares: SQL na borda com
   **Turso/libSQL**, leituras de documentos/grafos com o **SurrealDB** ou
   estruturas do **Redis**. Eles usam suas próprias APIs e não substituem o
   pool principal. Para MongoDB, DuckDB ou Qdrant, use o crate oficial, como
   mostram as [receitas](../recipes/index.md).

[Suporte a bancos de dados](../database-support.md) lista com o que cada backend
funciona; leia antes de escolher.

Depois da geração, o fluxo padrão compila a nova aplicação uma vez e executa a
migração inicial. Uma primeira compilação limpa pode levar vários minutos no
starter SaaS, que é maior. Use `--skip-initial-migration` quando o banco de
dados em rede ainda não estiver configurado e depois execute
`cargo rullst db:migrate` no projeto gerado. `cargo rullst dev` aplica as
migrações pendentes ao iniciar, mas um servidor em produção nunca faz isso:
execute o comando `db:migrate` do binário a cada release, como descrito em
[Antes do primeiro deploy](../tutorials/26-one-click-paas-deploy.md#before-the-first-deploy).

As flags não interativas espelham o segundo passo:

```bash
cargo rullst new edge_app --default --database mariadb --turso --surrealdb \
  --skip-initial-migration
```

A automação pode fixar o starter, o banco de dados e os recursos opcionais de
runtime suportados. A arquitetura da aplicação é fixa de propósito: Active
Record com `html!`/HTMX renderizados no servidor:

```bash
cargo rullst new subscriptions --default --blueprint saas \
  --database postgres --ai \
  --skip-initial-migration
```

Para uma aplicação blank sem banco de dados relacional principal, use a flag
explícita `--no-database`. Ela não pode ser combinada com `--database`. Os
perfis SQLx gerados desativam as features padrão das dependências do pacote
guarda-chuva do Rullst e do `rullst-orm` e escolhem exatamente um backend
relacional estrito, para que um perfil PostgreSQL/MySQL/MariaDB escolhido não
seja compilado por acidente com um padrão SQLite implícito.

Para criar uma API com o Turso como banco principal usando o starter blank
atual, de escopo limitado:

```bash
cargo rullst new edge_api --default --api --database turso \
  --skip-initial-migration
cd edge_api
cargo rullst db:migrate
```

O `.env` gerado escolhe uma alternativa offline persistente com SQL de verdade
e não inventa um `DATABASE_URL` do SQLx:

```env
TURSO_DATABASE_URL=mock_local
TURSO_AUTH_TOKEN=
TURSO_OFFLINE_PATH=turso-development.db
```

Para usar o Turso Cloud, coloque o token em uma variável própria, e não na
URL:

```bash
turso db create my-app-db
turso db tokens create my-app-db
```

```env
TURSO_DATABASE_URL=libsql://my-app-db-username.turso.io
TURSO_AUTH_TOKEN=replace-with-a-secret-token
```

O derive conhecido escolhe explicitamente o backend Turso:

```rust
#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "users", backend = "turso", full_row_save)]
struct User {
    id: i64,
    name: String,
}
```

`User::all()`, `find`, `save`, `create`, `delete`, `count`, filtros, ordenação
e paginação passam então a ser executados pelo store principal `TursoOrm`.
`make:model --migration`, `make:migration`, `db:migrate`, `db:status` e
`db:rollback` preservam esse backend. O contrato atual de scaffold de primeira
classe se limita, de propósito, aos starters Blank e API; o starter SaaS,
específico do SQLx, recusa `--database turso` até ser adaptado. Não se promete
sincronização transparente de réplicas embarcadas.

O `TursoStore` continua disponível para SQL preparado, materialização limitada
de resultados, transações e migrações com checksum, tanto na configuração
principal quanto na adicional. Veja [Persistência poliglota](../polyglot-persistence.md)
para a configuração completa, as restrições de segurança e exemplos de cada
store opcional.
