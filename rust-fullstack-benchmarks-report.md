# Relatório de Benchmark: Rullst vs Frameworks Concorrentes (Rust)

Este relatório detalha a comparação arquitetural e de performance entre o Rullst e outros frameworks full-stack e backend ecossistema Rust, como Loco, Leptos, Dioxus e Axum (como base).

## Escopo do Benchmark

O benchmark foi focado em três pilares principais onde a arquitetura dos frameworks interage mais fortemente com a performance pura:

1. **Server-Side Rendering (SSR):** Geração de HTML no backend para os clientes.
2. **Roteamento em Memória (Texto Puro):** Custos de roteamento e middlewares para requisições de texto.
3. **Serialização JSON:** O "feijão com arroz" das APIs para interações com SPAs.

## 1. Por que o Rullst Vence no SSR (Server-Side Rendering)?

Ao realizar a renderização Server-Side, o Rullst possui uma vantagem estrutural gigantesca em comparação com os principais concorrentes:

* **Contra Dioxus e Leptos:** Esses frameworks nasceram como soluções para interfaces de usuário usando a abordagem de componentes em árvore. Isso exige a construção (mesmo em SSR) de um **Virtual DOM** ou de estruturas de controle no Rust, que posteriormente precisam ser cacheadas, percorridas e serializadas para uma `String`.
* **Contra Loco (via Tera/Askama):** Loco depende de engines tradicionais de template (como Tera). Templates são arquivos de texto parseados e interpretados em *runtime* (no caso do Tera) ou que geram grande boilerplate de compilação.
* **A Solução Rullst (`html!` macro):** O Rullst realiza tudo em tempo de compilação sem estruturas intermediárias. A macro `html!` não cria objetos de DOM ou Virtual DOMs; ela simplesmente expande o código em operações diretas de concatenação em um buffer pré-alocado (`String::with_capacity(..)`) injetando variáveis de escopo usando `std::fmt::Write`.
  * *Resultado:* Zero overhead de runtime. O Rullst vence por evitar alocações repetidas de memória, entregando o HTML gerado mais rápido.

## 2. Roteamento (Rullst vs Loco / Axum)

O roteamento é onde o custo do framework é pago.

* **Loco:** O Loco é focado na produtividade no estilo Rails. No entanto, para oferecer essa experiência MVC robusta, ele embute uma pesada camada de abstração com `Hooks`, middlewares de autenticação, Injeção de Dependências e context structs. Tudo isso roda em cima do Axum.
* **Axum:** O Axum puro é muito rápido, mas não provê nada fora da caixa. Fica a cargo do desenvolvedor acoplar serializadores, bancos de dados, segurança WAF, etc.
* **O Ponto Ótimo do Rullst:** O Rullst usa o ecossistema Tower/Axum, assim como o Loco. Contudo, o Rullst gera o seu sistema de roteamento via macros compiladas (`routes!`) e usa "Zero-Cost Abstractions" para compilar handlers. Ele provê a velocidade quase idêntica ao Axum "puro", mas com uma produtividade "Full-Stack". Na camada de resposta JSON, o Rullst expõe os tipos primitivos e os envia no formato serializado sem conversões dinâmicas.

## 3. Filosofia Full-Stack e o Custo do Virtual DOM

O que difere o Rullst de frameworks puramente voltados para SPA/WASM (como Leptos e Dioxus) é que ele abraça a filosofia **Server-Driven UI** (frequentemente em conjunto com HTMX e Alpine.js), utilizando as macros compiladas para HTML e a extensão `LiveComponent` para updates reativos via WebSockets.

Neste cenário:
- A resposta inicial ao cliente tem **zero delay de parsing JavaScript**, renderizando na velocidade de luz comparado a uma arquitetura SSR de Virtual DOM.
- As abstrações "Zero-Panic" do Rullst significam não apenas previsibilidade de erros, mas compilações altamente otimizadas na remoção de blocos dinâmicos (`unwrap/expect`).

## Conclusão

O Rullst oferece a performance do C++/Rust (via Axum direto e compilação HTML nativa) empacotado em uma experiência de desenvolvimento similar ao Laravel, Loco ou Rails. Ele vence os demais por:
1. Eliminar completamente árvores de Virtual DOM durante o SSR.
2. Evitar engines lentas de Templates processadas em runtime (Tera/Liquid).
3. Utilizar conciliação zero-cost nos roteadores do Axum com macros que geram as rotas no momento de build do crate.