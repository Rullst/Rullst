# Como o Rullst é verificado, e o que isso não comprova

> **Tradução** da página em inglês [`how-rullst-is-verified.md`](../how-rullst-is-verified.md), feita a partir do commit [`fd32b89c8`](https://github.com/Rullst/Rullst/commit/fd32b89c8c4faf2761d4eaa14de2d3edd4b4ee1c). Se as duas versões divergirem, vale a versão em inglês.

Esta página é para quem quer saber até onde pode confiar no Rullst. Ela lista
as verificações que rodam antes de uma versão, a evidência que cada uma deixa
e o que nenhuma delas mostra. Cada afirmação tem um link para sua evidência.
Quando uma verificação tem um limite, o limite aparece ao lado dela.

Nada aqui é auditoria, certificação ou revisão externa. Nenhuma revisão
externa aconteceu. Passar nessas verificações significa que o código faz o que
seus testes e sua especificação dizem. Não significa que o projeto
(*design*) está certo, que a especificação está completa nem que uma aplicação
que você construa é segura.

Os fatos abaixo foram conferidos no repositório, na release e nas execuções de
workflow em 26 de novembro de 2026, para a
[13.0.0-rc.1](https://github.com/Rullst/Rullst/releases/tag/v13.0.0-rc.1).
Versões posteriores mudam os números; os changelogs delas dizem isso.

## As camadas

### 1. As aplicações do mantenedor

O Rullst é usado nas aplicações do próprio mantenedor. Academy, capoeira e
vene.win rodam em domínios reais, numa máquina Arm Oracle Ampere A1, atrás de
um Caddy compartilhado com certificados Let's Encrypt, e uma aplicação privada
também roda em Rullst
([Escopo e limites](scope-and-limits.md#contas-e-provedores-externos)). O
login com Google e GitHub e o adaptador Bunny Stream do `rullst-media` são
usados por essas aplicações
([níveis de maturidade](../maturity.md#what-the-documentation-tells-you)).

O que isso mostra: as partes que as aplicações usam funcionam em uso real, e
atualizar as aplicações expõe defeitos que os testes não pegaram. Defeitos
relatados pelas aplicações que migraram para a 13.0.0-alpha.6 foram corrigidos
na rc.1 ([plano de entrega da v13](../v13-delivery-plan.md)).

O que isso não mostra: as aplicações e o framework têm o mesmo autor, então
não é uso independente. Nenhum teste automatizado roda contra essas
instalações, e o que elas não usam não é coberto.

### 2. Testes e verificações obrigatórias

Todo pull request para `main` roda testes, Clippy estrito, verificações de
fronteira de features, a política de zero pânicos, a política de código
`unsafe`, a verificação do MSRV, a política de dependências e a busca por
segredos. A `main` exige 20 verificações nomeadas antes de um merge
([`GOVERNANCE.md`](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)).
A cobertura é medida com LLVM, e o workflow rejeita um resumo abaixo de 90%
para o repositório ou para as bibliotecas do framework
([Coverage](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#coverage),
[`coverage.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/coverage.yml)).
Os workflows de zero pânicos e de código `unsafe` estão descritos em
[Zero-panics and unsafe Rust](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#zero-panics-and-unsafe-rust):
a política de `unsafe` é uma fronteira imposta, não uma afirmação de que as
dependências não contêm Rust `unsafe`.

O que isso não mostra: um teste passa quando o código corresponde ao que o
teste espera. Uma expectativa errada também passa.

### 3. Testes de mutação

O [`mutants-pr.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants-pr.yml)
altera as linhas que um pull request toca, uma pequena mudança (um mutante) de
cada vez, e falha quando nenhum teste percebe. O
[`mutants.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants.yml),
manual, roda campanhas completas e direcionadas. Um mutante que sobrevive ou
ganha um teste ou é listado, com o motivo, como mutante equivalente
([registro dos testes de mutação](../mutation-testing.md)). A proteção de
branch não exige o portão dos pull requests, e uma campanha não faz parte da
admissão da release
([portão de mutação nos pull requests](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#pull-request-mutation-gate)).

Para a 13.0.0-rc.1, campanhas direcionadas mediram de novo os arquivos
alterados desde a 13.0.0-alpha.6. Dez arquivos do `cargo-rullst` tinham 732
mutantes: 579 capturados, 6 timeouts e 98 perdidos, e o resto inviável. Dos 94
mutantes perdidos distintos, 80 ganharam um teste e 14 foram classificados como
equivalentes ou inobserváveis
([PR #872](https://github.com/Rullst/Rullst/pull/872)). Uma nova medição dos
dez arquivos naquele branch deixou 21 mutantes perdidos e 6 timeouts, todos
entre os que o pull request classificou
([comentário no PR #872](https://github.com/Rullst/Rullst/pull/872#issuecomment-7195513104)).

O que isso não mostra: "equivalente" é um julgamento do mantenedor, registrado
com um motivo, não uma prova. Um mutante sobrevivente que só uma resposta de um
provedor real alcança fica para os testes com o provedor
([MAINTAINING.md](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md#6-mutation-campaigns)).

### 4. Fuzzing

O `fuzzing.yml` roda o libFuzzer em 42 alvos declarados, em onze workspaces de
fuzzing
([Fuzzing and OSS-Fuzz](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#fuzzing-and-oss-fuzz)).
Uma release exige uma execução bem-sucedida no commit da release. Para a
13.0.0-rc.1, é a [execução 45693088611](https://github.com/Rullst/Rullst/actions/runs/45693088611)
no commit `9ea50d98`: 42 alvos selecionados, nenhum reaproveitado de execução
anterior (o artefato `fuzz-evidence-9ea50d98…` dessa execução registra isso),
cada um com um orçamento de campanha de 19.800 segundos.

O que isso não mostra: fuzzing é teste aleatório limitado. Não prova que uma
entrada é segura nem que outra execução não encontraria nada
([reaproveitamento de evidências de fuzzing](../fuzz-evidence.md#publication-and-receipts)).
O diretório `oss-fuzz/` é um rascunho local; o OSS-Fuzz upstream não aceitou o
projeto.

### 5. Miri, Kani, sanitizers e testes de propriedade

- O **Kani** prova propriedades limitadas de decisões puras, com vinte
  harnesses em dez pacotes. O **Miri** roda 15 escopos nomeados de Rust puro.
  Nenhum dos dois cobre criptografia nativa, SQLite, caminhos de rede nem
  middleware concorrente inteiro.
- O **ThreadSanitizer e o AddressSanitizer** rodam em doze pacotes. Não há job
  de MemorySanitizer.
- Os **testes de propriedade** (`proptest.yml`) rodam a suíte com todas as
  features em modo release, com 10.000 casos nos contratos do ORM e do
  Connect.

Os quatro foram bem-sucedidos no commit `9ea50d98`
([Formal, dynamic and stress analysis](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#formal-dynamic-and-stress-analysis);
workflows
[`kani.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/kani.yml),
[`miri.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/miri.yml),
[`sanitizers.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/sanitizers.yml),
[`proptest.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/proptest.yml)).

O que eles não mostram: são evidência limitada, para os harnesses e escopos que
rodam.

### 6. Varreduras de segurança estáticas e dinâmicas

- O **CodeQL** (`codeql.yml`) analisa o código Rust e o JavaScript/TypeScript
  em pushes, pull requests e semanalmente.
- O **OWASP ZAP** (`dast-zap.yml`) varre um starter de API REST e um starter
  SaaS recém-gerados (páginas públicas e depois páginas autenticadas) e o
  exemplo LMS. Os alvos são representativos, não todos os starters, papéis ou
  instalações; administradores do Nexus não são varridos
  ([`dast-zap.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/dast-zap.yml),
  [Formal, dynamic and stress analysis](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#formal-dynamic-and-stress-analysis)).
- O **SonarCloud** analisa pull requests. Não é uma verificação obrigatória: a
  verificação dele falhou no [PR #872](https://github.com/Rullst/Rullst/pull/872),
  que foi integrado, e ele não está entre as verificações que a `main` exige
  ([ruleset](https://github.com/Rullst/Rullst/rules/24175738)).

CodeQL e ZAP estão entre os workflows que precisam passar para uma release
([`release-required-workflows.json`](https://github.com/Rullst/Rullst/blob/main/.github/release-required-workflows.json)).
O SonarCloud não está.

O que eles não mostram: varreduras encontram padrões conhecidos. Não
encontram uma falha num projeto que elas não enxergam.

### 7. Auditoria de dependências e admissão da release

- `cargo deny` e `cargo audit` rodam no
  [`supply-chain.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/supply-chain.yml).
  Dois avisos têm exceções com prazo até 2027-01-28, listadas com seus motivos
  em [Security Advisory Exceptions](../security-advisory-exceptions.md).
- O OSV-Scanner roda pelo workflow do OpenSSF Scorecard
  ([`scorecards.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/scorecards.yml));
  seus ignores estão listados na mesma página. A pontuação é evidência da
  cadeia de suprimentos, não uma certificação de segurança
  ([OpenSSF Scorecard](../openssf-scorecard.md)).
- A **admissão da release** (`release.yml`, só em tag) se recusa a publicar a
  menos que 23 workflows nomeados em
  [`release-required-workflows.json`](https://github.com/Rullst/Rullst/blob/main/.github/release-required-workflows.json)
  tenham sido bem-sucedidos no commit exato. Para a 13.0.0-rc.1 foram, no
  commit `9ea50d98`, e a
  [execução de release 45837583222](https://github.com/Rullst/Rullst/actions/runs/45837583222)
  publicou os vinte pacotes
  ([plano de entrega da v13](../v13-delivery-plan.md)). A release traz o SBOM,
  a saída do `cargo audit` e uma atestação de proveniência de build do GitHub.
  Isso não é um nível SLSA, uma afirmação de assinatura nem uma afirmação de
  conformidade regulatória
  ([Supply chain and release provenance](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#supply-chain-and-release-provenance)).

A admissão do primeiro candidato da rc.1 encontrou um caminho desatualizado do
modelo de ameaças e um aviso do RustSec (`url`); o segundo candidato encontrou
outro (`cookie`). Ambos foram corrigidos antes da tag
([#862](https://github.com/Rullst/Rullst/pull/862),
[#867](https://github.com/Rullst/Rullst/pull/867)).

### 8. Revisões céticas das afirmações públicas

Antes de uma release, um assistente de IA sem o contexto do trabalho da
release lê a documentação pública e o código, os testes e a CI, e relata o que
a documentação afirma mas não sustenta. Cada achado é corrigido ou declarado
nas limitações conhecidas
([MAINTAINING.md](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md#7-releasing-a-prerelease)).
O [`GOVERNANCE.md`](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)
diz claramente que um assistente não é um revisor humano independente.

## O que isso não comprova

- **Projeto e afirmações.** As camadas verificam que o código faz o que seus
  testes e sua especificação dizem. Elas não mostram que uma decisão de
  projeto, a especificação ou uma afirmação pública está certa.
- **Nenhuma revisão de segurança externa.** Nenhum terceiro revisou a
  segurança do Rullst. A revisão de segurança independente é trabalho em
  aberto no
  [roadmap](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md#verification-roadmap),
  e o [kit de auditoria externa](../external-audit-kit.md) é o material para um
  revisor. As ferramentas do próprio Rullst também não certificam: os comandos
  de [evidência de segurança](https://github.com/Rullst/Rullst/blob/main/SECURITY.md)
  imprimem o que verificaram e marcam o resto como não avaliado.
- **Nenhuma conta real de provedor nos testes automatizados.** A CI exercita
  os provedores de pagamento, vídeo, OAuth, e-mail e IA contra mocks locais,
  fixtures e servidores de teste. O adaptador S3/R2 do `rullst-media` é testado
  apenas offline
  ([Escopo e limites](scope-and-limits.md#contas-e-provedores-externos),
  [níveis de maturidade](../maturity.md#what-the-documentation-tells-you)).
- **Um único mantenedor.** Uma pessoa é dona do código, dos testes, da
  especificação e das verificações, e faz os merges. As aprovações exigidas na
  `main` são zero enquanto houver um único mantenedor ativo
  ([`GOVERNANCE.md`](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)).
  Assistentes preparam alterações dentro da autoridade do mantenedor, e o
  [`GOVERNANCE.md`](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)
  diz que eles não são um revisor independente. Uma verificação só pega o que o
  mantenedor pensou em verificar.
- **Sua aplicação.** Nada disso cobre o código que você escreve, sua
  instalação, seus dados ou sua configuração.

## Estudo de caso: verificação não é validação

Verificar pergunta se o trabalho cumpre suas verificações. Validar pergunta se
as verificações medem a coisa certa. O repositório
[Benchmarks](https://github.com/Rullst/Benchmarks) do Rullst mostra a
diferença duas vezes.

**A versão 1 media a coisa errada.** A primeira medição, feita à mão num
notebook em 22 de junho de 2026, listava o status "Passed" para entradas como
o FastAPI com 2,49 requisições por segundo em `/db-single`. Não havia
verificação de conformidade, configuração registrada nem saída reproduzível, e
o repositório hoje chama várias entradas de "evidentemente quebradas". Ela é
mantida apenas como registro e marcada como não comparável
([histórico](https://github.com/Rullst/Benchmarks/blob/main/history/2026-06-22-laptop.md)).
Sua sucessora, a versão 2, também se mostrou medir menos do que parecia: seu
nível `/db-single` era limitado pelo cliente na maioria das rodadas, então o
gerador de carga, e não o servidor, limitava o resultado
([História do Benchmarks](https://github.com/Rullst/Benchmarks#history)).

**A falha do `PUT /world/{id}` favoreceu o Rullst.** A versão 3 passou nas
próprias verificações de conformidade, e sua primeira execução oficial (Rullst
13.0.0-alpha.6) publicou resultados. Uma revisão independente em 20 de
novembro de 2026 descobriu que o endpoint `PUT` não media nenhuma escrita nas
implementações cujo ORM grava só as colunas alteradas, entre elas o Rullst: a
carga repetia pares que o aquecimento já havia gravado. A razão de `PUT` e a
pontuação de Application do Rullst ficaram mais altas do que seriam com uma
escrita. Os resultados afetados estão marcados com ‡ até que uma execução
posterior os substitua, cada `PUT` agora leva um valor novo, e o portão de
publicação recusa uma execução que não gravou
([História do Benchmarks](https://github.com/Rullst/Benchmarks#history),
[nota ‡](https://github.com/Rullst/Benchmarks#readme),
[especificação v3](https://github.com/Rullst/Benchmarks/blob/main/docs/v3-spec.md)).
O mesmo README diz que o mantenedor escreveu todas as implementações e
projetou o método.

A lição para esta página: uma verificação verde diz que a verificação passou.
Se ela era a verificação certa é outra pergunta, e uma revisão independente,
não as verificações do próprio benchmark, encontrou a falha do `PUT`.

## Confira você mesmo

- **Execuções de workflow.** A
  [aba Actions](https://github.com/Rullst/Rullst/actions) guarda todas as
  execuções linkadas acima. O
  [`WORKFLOWS.md`](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md)
  (a página [workflows](../workflows.md) do livro) diz o que cada workflow
  estabelece e o que não estabelece.
- **Evidência de fuzzing.** A execução de fuzzing de uma release envia um
  artefato `fuzz-evidence-<commit>` que lista o candidato, os digests da
  política e das entradas, e cada alvo, selecionado ou reaproveitado com sua
  execução original
  ([reaproveitamento de evidências de fuzzing](../fuzz-evidence.md)). A
  verificação de admissão também retém um `release-fuzz-evidence.json` para sua
  decisão; para a 13.0.0-rc.1, o registro a ler é o artefato da execução
  45693088611.
- **Admissão.** O
  [`release-required-workflows.json`](https://github.com/Rullst/Rullst/blob/main/.github/release-required-workflows.json)
  nomeia os workflows, e `python3 .github/check-release-admission.py` informa o
  que falta para um commit
  ([MAINTAINING.md](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md#7-releasing-a-prerelease)).
- **Arquivos da release.** A [release 13.0.0-rc.1](https://github.com/Rullst/Rullst/releases/tag/v13.0.0-rc.1)
  anexa o SBOM, o `cargo-audit.json`, os checksums e a atestação de
  proveniência.
- **Benchmarks.** O [repositório Benchmarks](https://github.com/Rullst/Benchmarks)
  publica seu método, a saída bruta e seus limites.
- **Registro de mutação.** O [registro dos testes de mutação](../mutation-testing.md)
  lista cada campanha e cada mutante equivalente aceito.

Se uma afirmação desta página não for sustentada pelo seu link, ou se você
encontrar uma falha, [abra uma issue](https://github.com/Rullst/Rullst/issues/new/choose).
Relate vulnerabilidades de segurança de forma privada, como o
[`SECURITY.md`](https://github.com/Rullst/Rullst/blob/main/SECURITY.md)
descreve.
