# Preservação da documentação anterior ao `gpt.md`

> **Snapshot histórico identificado:** commit
> [`96222fbd31bec3d20bc50db68c41bb85ca595779`](https://github.com/Rullst/Rullst/tree/96222fbd31bec3d20bc50db68c41bb85ca595779),
> de 24 de agosto de 2026. O `gpt.md` foi criado no commit seguinte,
> `ecf3ecb`. Nenhum texto anterior depende da memória de uma conversa para ser
> recuperado.

Este documento existe para garantir que a correção técnica não apague a visão
original do Rullst. Ele também evita um problema igualmente grave: recolocar
exemplos inseguros, URLs obsoletas ou capacidades inexistentes dentro de guias
que um usuário pode copiar como instrução atual.

## Política de preservação

1. O snapshot acima é a cópia exata e imutável de toda a documentação anterior.
2. O [roadmap principal](roadmap.md) preserva as ambições como itens ativos e
   acrescenta `Implementado`, `Parcial`, `Não implementado` ou `Não prometer`,
   sempre com uma recomendação.
3. Os roadmaps por crate preservam o detalhamento original. O quadro de auditoria
   no roadmap principal é a interpretação atual quando um checkbox histórico é
   mais amplo que a implementação.
4. A [especificação](spec.md) e os exemplos continuam operacionais: eles
   descrevem somente APIs e limites atuais. Tutoriais estão explicitamente fora
   desta reconstrução histórica e devem existir apenas na forma atual,
   copiável e segura.
5. O [changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md) mantém a
   alegação antiga e coloca uma nota adjacente de **escopo auditado na v12** em
   vez de apagá-la silenciosamente.

## Ambições recuperadas e interpretação atual

Esta tabela cobre as famílias de capacidades extraordinárias presentes no
snapshot. Os identificadores `M*` apontam para a classificação detalhada no
[roadmap](roadmap.md).

| Ambição original preservada | Estado atual e opinião | Destino canônico |
| :--- | :--- | :--- |
| CLI completa, generators, `make:resource`, docs hub (mdBook) e SDK TypeScript | `[~] Parcial` — vale concluir com matriz de projetos gerados e um schema de API canônico; inferência AST isolada não basta. | M1, M4, M5 e M34 |
| Recompilação sub-100 ms com mold/lld/Cranelift | `[~] Parcial` — vale otimizar e publicar benchmarks por máquina; não vale garantir um tempo universal. | M2 |
| Zero lock-in, eject para Axum/Tokio e módulos totalmente opcionais | `[~] Parcial` — escape hatches são valiosos; migração sem custo e opcionalidade universal não são garantias honestas. | M3 e M32 |
| Active Record, Repository, Turso/libSQL e réplicas SQLite transparentes | `[~] Parcial` — os padrões ORM existem; drivers e replicação precisam de semântica vendor-specific e testes reais. | M6 e M38 |
| Edge/Wasm distribuído e upgrades autônomos | `[~] Parcial` — o runtime portátil vale evoluir; atualização autônoma só com artefato assinado, aprovação e rollback. | M7 |
| Modelagem por intenção e índices de produção auto-otimizados | `[ ] Não implementado` — vale como recomendação explicável e aprovada; DDL autônomo em produção não vale o risco. | M8 |
| Auth local, OAuth/OIDC, TOTP, passkeys e WebAuthn completo | `[~] Parcial` — prioridade alta; exige conformance WebAuthn, recovery, revogação e política de sessão/JWT. | M9 |
| Nexus instantâneo, Omni/Tauri, billing e entitlements declarativos | `[~] Parcial` — Nexus e billing têm fundações reais; Omni e gates completos precisam de contratos independentes. | M11, M21 e M33 |
| RASP/WAF, Vault, honeypots, HMAC audit, headers A+, Login Jail, DLP, fingerprinting, IDOR scanner e SOC | `[~] Parcial` — os controles delimitados valem hardening contínuo; A+, zero leakage, zero latency, cobertura OWASP total e certificação não devem ser prometidos. | M12 e capability ledger |
| Kani em 100% dos paths, fuzzing como imunidade a DoS e sanitizers como prova de ausência de races | `[!] Não prometer` — as ferramentas são excelentes para alvos e execuções declarados; nenhuma prova segurança universal. | Programa v12, seções 4, 7 e 9 |
| SBOM como conformidade SOC 2/ISO/FedRAMP e “100% Rustls” como prova de segurança | `[~] Parcial` — inventário e política de transporte valem manter; conformidade exige controles operacionais/auditoria, e Rustls não elimina todo risco. | M12 e programa v12 |
| PQC, KMS/HSM, eBPF, contenção no kernel, sandbox Wasm e heap guard pages | `[ ] Não implementado` — vale somente por ameaça/protocolo concretos e com primitives auditadas; não criar criptografia própria. | M13 e roadmap Security |
| HTMX zero-bundle, adapters Leptos/Dioxus, cinco engines frontend | `[~] Parcial` — HTMX/SSR é real; adapters e engines precisam de compatibilidade e E2E, e “zero bundle” depende da opção escolhida. | M14 |
| Queues/cache/scheduler com Redis, RabbitMQ, Kafka, Streams, NATS e clouds | `[~] Parcial` — Memory/SQLite/Redis têm fundações; adapters ausentes valem apenas com contrato compartilhado e testes reais. | M15 |
| Wasm islands e `#[client_component]` totalmente reativos | `[~] Parcial` — vale concluir protocolo, serialização, hydration, empacotamento e browser E2E. | M16 |
| Realtime, S3/R2, media resizing e package registry | `[~] Parcial` — realtime/local storage têm bases; object storage e registry são trabalho futuro modular. | M17 |
| LiveView completo e sem lógica cliente | `[~] Parcial` — o loop WebSocket existe; auth, reconnect, backpressure, diffs e E2E ainda são necessários. | M18 |
| Radar “kernel-level”, Prometheus e traces distribuídos | `[~] Parcial` — telemetria local/export existe; não é eBPF/kernel nem waterfall OTel distribuída completa. | M19 e M35 |
| Event streaming zero-copy e ledger imutável | `[ ] Não implementado` — interessante após definir persistência, consistência, recuperação e verificação; HMAC chain local não é ledger distribuído. | M20 |
| Agentic DevOps, self-healing runtime e autofix autônomo | `[~] Parcial` — recomendações e patches revisáveis são úteis; mutação autônoma exige preview, escopo, aprovação, testes e rollback. | M22, M23 e M37 |
| IoT completo: MQTT, Modbus/BLE reais, mesh, OTA, HSM/PQC e hardware verificado | `[~] Parcial` — frames `no_std` e gate Ed25519 são reais; transporte, flash, boot, counters, hardware e certificação são programas separados. | M24 e M25 |
| Deploy realmente one-click/zero-downtime em PaaS/VPS | `[~] Parcial` — scaffolding é útil; DNS, credenciais, migration, health e rollback continuam responsabilidades operacionais. | M26 |
| Kubernetes pronto para produção | `[x] Implementado no escopo de scaffold` — manifests e probes existem; revisão e operação continuam com o usuário. | M27 |
| DI com custo zero | `[x] Fundação implementada` — API typed existe; custo zero é hipótese de benchmark, não garantia. | M28 |
| OpenAPI/Scalar e SDKs automáticos completos | `[~] Parcial` — UI e generators existem; fidelidade exige schemas tipados e testes de serialização. | M29 e M34 |
| gRPC/Tonic first-class | `[~] Parcial` — generator inicial existe; falta crate suportada e matriz de conformidade do projeto gerado. | M30 |
| Aerospace, veículos autônomos, robótica e defesa | `[ ] Não implementado` — visão extraordinária, mas não deve entrar no Core web; só vale como projeto safety-critical independente, com hardware, standards e governance. | M31 |
| AI SQL copilot e AI Admin capaz de mutar dados | `[ ] Não implementado no escopo autônomo` — vale como read-only/preview com allowlists, parâmetros, limites, autorização e auditoria. | M36 |
| NFS-e nacional direta com XMLDSig, mTLS e custo zero | `[ ] Live não implementado` — ambição muito valiosa como programa de homologação separado; hoje existe apenas preview offline não autorizado e live falha fechado. | Capital roadmap e Maybe SaaS |
| Alipay RSA2, métodos uniformes e taxas fixas dos gateways | `[~] Parcial` — adapters/mocks existem, mas método, preço e cobertura variam; Alipay live permanece desabilitado até interoperabilidade oficial. | Capital roadmap e capability ledger |
| AI Firewall invulnerável, offline AI autônoma e classificação sem vazamento | `[~] Parcial` — filtros, mocks e Ollama existem; heurísticas têm falsos positivos/negativos e tools sensíveis precisam de autorização externa ao modelo. | AI/Security roadmaps e programa v12 |
| Mail Radar, CSS inlining, AI dunning, inbound mail e deliverability universal | `[~] Parcial` — transports, pipeline e fixtures são úteis; as expansões valem com contract suite e operação observável. | Mail roadmap |
| Studio N+1 profiler, Cache/Redis browser e telemetry sempre “live” | `[~] Parcial` — ferramentas reais mostram dados/unavailable; N+1 e cache browser permanecem backlog. | Studio roadmap e capability ledger |

## Onde está cada documento original

Os links abaixo abrem o texto exato anterior ao `gpt.md`. Eles são referência
histórica; para uso atual, prefira a documentação do branch principal.

### Governança e visão

- [README original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/README.md)
- [ROADMAP original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/ROADMAP.md)
- [CHANGELOG original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/CHANGELOG.md)
- [AUDIT original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/AUDIT.md)
- [SECURITY_COMPLIANCE original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/SECURITY_COMPLIANCE.md)
- [WORKFLOWS original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/WORKFLOWS.md)

### Livro e especificações

- [Árvore `docs/src` original](https://github.com/Rullst/Rullst/tree/96222fbd31bec3d20bc50db68c41bb85ca595779/docs/src)
- [Spec original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/docs/src/spec.md)
- [Security Architecture original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/docs/src/security-architecture.md)
- [Threat Radar/SOC original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/docs/src/threat-radar-soc-guide.md)
- [Payment Gateways original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/docs/src/payment-gateways-guide.md)
- [Examples guide original](https://github.com/Rullst/Rullst/blob/96222fbd31bec3d20bc50db68c41bb85ca595779/docs/src/examples.md)

### Crates e aplicação de referência

- [Documentação original das crates](https://github.com/Rullst/Rullst/tree/96222fbd31bec3d20bc50db68c41bb85ca595779)
  — abra `README.md` e `ROADMAP.md` dentro de cada diretório `rullst-*`.
- [`examples/blog` original](https://github.com/Rullst/Rullst/tree/96222fbd31bec3d20bc50db68c41bb85ca595779/examples/blog)

## Regra para futuras correções

Ao descobrir uma alegação incorreta:

- não apagar a ambição do roadmap;
- não manter a alegação como fato numa spec ou tutorial copiável;
- acrescentar o status, o limite, a recomendação e a evidência;
- ligar a versão histórica quando a redação original tiver valor de registro;
- atualizar o capability ledger e o programa de release quando o risco for
  transversal.

Assim o Rullst pode conservar sua imaginação sem pedir que usuários confundam
uma visão excelente com uma garantia de produção.

> **Escopo excluído por decisão do mantenedor:** tutoriais antigos não precisam
> ser republicados nem anotados. O histórico Git continua existindo, mas somente
> tutoriais atuais devem aparecer no livro.
