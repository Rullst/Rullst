# Primeira hora com o Rullst

> **Tradução** da seção [“Rullst in 30 seconds”](../../../README.md#quickstart) do `README.md` em inglês, feita a partir do commit [`654944c44`](https://github.com/Rullst/Rullst/commit/654944c445e464c2b55d4720e6dd70105a07e44d). Se as duas versões divergirem, vale a versão em inglês.

Estas páginas traduzem para o português do Brasil a trilha da primeira hora:
instalação, `cargo rullst new`, a primeira página e a primeira API. A página
[Escopo e limites](scope-and-limits.md) e o guia
[Robótica e IoT em sala de aula](iot-classroom.md) também estão traduzidos. O
restante da documentação, incluindo a referência da CLI e os demais
tutoriais, está em inglês.

## ⚡ O Rullst em 30 segundos

Com o Rust instalado ([rustup.rs](https://rustup.rs/)):

```bash
cargo install cargo-rullst --version =13.0.0-alpha.6 --locked
cargo rullst new my_app    # choose the Blank, API or SaaS starter
cd my_app
cargo rullst dev           # rebuilds and restarts every time you save
```

Pronto: uma aplicação rodando, gerada como Rust legível que você pode alterar,
sem mágica escondida em tempo de execução. A primeira compilação compila as
dependências do framework e pode levar alguns minutos; as recompilações
seguintes são bem mais rápidas
([tempos de compilação](../build-times.md)). A versão é informada porque a
13.0 ainda é um pré-lançamento
([por quê](https://github.com/Rullst/Rullst/blob/main/README.md#versions)).
Quer um ponto de partida maior? Copie uma aplicação de
[`examples/`](https://github.com/Rullst/Rullst/tree/main/examples): um LMS
completo ou a vitrine do blog
([onde ficam os exemplos](https://github.com/Rullst/Rullst/blob/main/README.md#where-the-examples-live)).

Travou? `cargo rullst doctor` verifica sua toolchain e o espaço livre em
disco e, dentro de um projeto, a configuração, o banco de dados e as
migrações. `cargo rullst --help` lista todos os comandos, `cargo rullst tour`
apresenta os principais, e `cargo rullst` sozinho abre um menu interativo.
Prefere um painel interativo? Execute `cargo rullst dash`
([fluxo de desenvolvimento](../tutorials/51-authenticated-hot-reload.md)).

## A trilha da primeira hora

1. [Primeiros passos e starters](1-getting-started.md): instale a CLI, crie
   um projeto com o assistente e conheça os starters Blank, API e SaaS.
2. [Tutorial 01: do zero ao Olá, Rullst](tutorials/01-hello-world.md): uma
   aplicação escrita à mão, com uma rota tipada e HTML renderizado no
   servidor.
3. [Crie sua primeira API REST](tutorials/rest-api-quickstart.md): um endpoint
   JSON verificado a partir de outro terminal.
4. [Tutorial 02: geradores da CLI](tutorials/02-cli-generators.md): o projeto
   `shop`, com controller, modelo, migração e recurso gerados.
5. [Tutorial 03: CRUD com Active Record](tutorials/03-active-record-crud.md):
   criar, ler, atualizar e excluir registros do modelo `Product`.
6. [Tutorial 05: migrações e seeders](tutorials/05-migrations-and-seeds.md):
   alterar o esquema, desfazer migrações e popular o banco de dados.
7. [Tutorial 11: autenticação](tutorials/11-authentication-system.md):
   cadastro, login, sessões no servidor e redefinição de senha.

Os blocos de código destas páginas são idênticos aos das páginas em inglês,
então os mesmos testes de documentação e as mesmas verificações da primeira
hora valem para eles. Cada página diz de qual commit da versão em inglês foi
traduzida; se as duas versões divergirem, vale a versão em inglês.
