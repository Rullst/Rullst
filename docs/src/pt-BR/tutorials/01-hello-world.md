# Tutorial 01: do zero ao Olá, Rullst 🚀

> **Tradução** da página em inglês [`01-hello-world.md`](../../tutorials/01-hello-world.md), feita a partir do commit [`654944c44`](https://github.com/Rullst/Rullst/commit/654944c445e464c2b55d4720e6dd70105a07e44d). Se as duas versões divergirem, vale a versão em inglês.

**O que você vai construir:** uma aplicação web completa com uma rota tipada e
HTML renderizado no servidor. Não é preciso conta de banco de dados nem de
provedor de IA. No fim, você vai conseguir apontar o handler que produziu a
página no seu navegador.

[Escolha outro caminho de início](../../start-here.md) ·
[Próximo: geradores da CLI](02-cli-generators.md)

Este tutorial leva quem está começando da instalação do Rust até uma aplicação
web Rullst rodando. Ele usa o pré-lançamento `13.0.0-rc.2` da v13. A linha
v12 chegou ao fim da vida útil em 16 de outubro de 2026 (veja o
[registro da versão](../../v12.md)); aplicações v12 existentes seguem o
[guia de migração para a v13](../../migration-v13.md).

## 1. Instale o Rust e o Cargo

No Linux ou no macOS, use o instalador oficial rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

No Windows, baixe o rustup em [rustup.rs](https://rustup.rs) ou execute:

```powershell
winget install --id Rustlang.Rustup
```

Reinicie o terminal se necessário e verifique a toolchain:

```bash
rustc --version
cargo --version
```

O Rullst v13 exige o Rust 1.96.0 ou mais recente, a MSRV registrada na
[política de compatibilidade](../../compatibility-policy.md); recomendamos a
versão estável mais recente do Rust. Execute `rustup update` se
`rustc --version` mostrar uma versão mais antiga.

## 2. Crie um projeto

```bash
cargo new my_first_app
cd my_first_app
```

Todos os comandos abaixo devem ser executados neste diretório, onde fica o
`Cargo.toml`.

## 3. Adicione o Rullst

Adicione o pré-lançamento da v13 a partir do crates.io. Informe a versão exata:
o Cargo nunca escolhe um pré-lançamento sozinho, e a linha v12 chegou ao fim da
vida útil em 16 de outubro de 2026, então um `cargo add rullst` simples
escolheria uma 12.x sem manutenção:

```bash
cargo add rullst@=13.0.0-rc.2
cargo add tokio --features full
```

O Cargo registra o grafo de dependências resolvido no `Cargo.lock`. Faça commit
desse arquivo em uma aplicação, para que o checkout continue reproduzível, e
revise as próximas atualizações de dependências antes do deploy.

## 4. Defina a primeira rota

Substitua o `src/main.rs` por:

```rust,no_run
use rullst::{html, routes, Markup, Server};

async fn home() -> Markup {
    html! {
        <div class="min-h-screen bg-slate-900 text-emerald-400 flex flex-col items-center justify-center font-sans">
            <h1 class="text-5xl font-extrabold mb-4">"Hello, Rullst! 📜🦀"</h1>
            <p class="text-slate-400 text-lg">"Your first typed route is running."</p>
        </div>
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = routes![
        get("/" => home)
    ];

    Server::new(app).run(3000).await?;
    Ok(())
}
```

A inicialização do servidor, que pode falhar, usa `?`, então falhas de bind e
de execução são devolvidas ao processo em vez de causar um panic vindo do
framework.

## 5. Execute a aplicação

Inicie o servidor:

```bash
cargo run
```

A primeira compilação compila todas as dependências e pode levar vários minutos
([Tempos de compilação](../../build-times.md)). Quando o terminal mostrar
`Rullst framework serving on http://127.0.0.1:3000`, abra
[http://127.0.0.1:3000](http://127.0.0.1:3000). Pare o servidor com `Ctrl+C`.

**Deixe com a sua cara:** mude o título para o nome do seu projeto, reinicie
com `cargo run` e confirme que a resposta mudou. Os nomes de classes
utilitárias do exemplo não instalam o Tailwind sozinhos; ver uma página sem
estilo não é uma falha do servidor. Os starters Blank e SaaS da CLI trazem uma
folha de estilo local (`static/rullst.css`).

Se a porta já estiver ocupada, pare o processo conflitante que for seu ou
inicie em outra porta com `PORT=3001 cargo run`: a variável `PORT` (do
ambiente e, depois, do `.env`) tem precedência sobre o argumento `.run(3000)`.
Guarde a saída do terminal: é o primeiro lugar onde procurar um diagnóstico de
compilação ou de inicialização.

## 6. Continue com a CLI

A CLI pode gerar starters completos e módulos de projeto. Instale o
pré-lançamento da v13 a partir do crates.io (informe a versão exata; o Cargo
nunca escolhe um pré-lançamento sozinho):

```bash
cargo install cargo-rullst --version =13.0.0-rc.2 --locked
cargo rullst --version
```

O segundo comando mostra `rullst 13.0.0-rc.2`. O [Tutorial 02](02-cli-generators.md)
cria um projeto com ela; o `my_first_app` escrito à mão não é necessário lá.

O gerador `new` da CLI usa a própria versão do framework da CLI. Uma CLI
compilada a partir de um checkout da `main` é a linha de desenvolvimento da
v13: ela gera dependências por caminho absoluto para esse checkout exato,
inclusive quando executada em outro lugar. Mantenha esse checkout onde está e
revise o código-fonte antes de compartilhar um projeto gerado. Veja a
[referência da CLI](../../cli_reference.md) para todos os comandos e limites.

## Principais aprendizados

- A macro `rullst::html!` gera código Rust comum de montagem de strings e
  escapa valores dinâmicos. Ela devolve `Markup`, que um handler retorna como
  `text/html` e que outro `html!` insere como HTML. A renderização ainda faz
  as alocações e o trabalho implicados pelo template gerado.
- Atributos HTML booleanos dentro de `html!` são escritos sozinhos
  (`required`) ou de forma condicional (`checked?={is_done}` escreve `checked`
  ou nada). A presença é o valor, então `disabled="false"` ainda desativaria o
  controle.
- Handlers que podem falhar podem retornar `Result<Response, YourAppError>`,
  usando um erro definido pela aplicação que converte os erros tipados
  relevantes do framework e do domínio; a inicialização do servidor propaga
  `ServerError` com `?`.
- A branch `main`, que muda o tempo todo, é uma fonte de integração, não um
  artefato de release imutável. Para uma aplicação reproduzível, fixe a versão
  exata do crates.io (`=13.0.0-rc.2`) e faça commit do `Cargo.lock`.
